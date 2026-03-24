use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{Query, State},
    http::{Method, StatusCode, header},
    routing::post,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::cors::{Any, CorsLayer};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the SQLite database file
    #[arg(short, long, default_value = "slimes.db")]
    database_url: String,

    /// Port to listen on
    #[arg(short, long, default_value_t = 8081)]
    port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BenchmarkResults {
    pub duration: Duration,
    pub primes_found: u64,
    pub score: u64,
    pub batch_count: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub prime_limit: u64,
    pub logical_cores: usize,
    pub single_thread: BenchmarkResults,
    pub multi_thread: BenchmarkResults,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FullReport {
    pub mac_address: String,
    pub timestamp: String,
    pub slimes: Option<HashMap<String, Vec<String>>>,
    pub benchmark: Option<BenchmarkReport>,
}

#[derive(Deserialize)]
pub struct Pagination {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

pub struct AppState {
    db: SqlitePool,
}

async fn submit(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<FullReport>,
) -> Result<StatusCode, (StatusCode, String)> {
    let score = payload
        .benchmark
        .as_ref()
        .map(|b| b.multi_thread.score)
        .unwrap_or(0);
    let raw_json = serde_json::to_string(&payload)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    sqlx::query("INSERT INTO reports (mac_address, score, timestamp, data) VALUES (?, ?, ?, ?)")
        .bind(payload.mac_address)
        .bind(score as i64)
        .bind(payload.timestamp)
        .bind(raw_json)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::CREATED)
}

async fn get_leaderboard(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<FullReport>>, (StatusCode, String)> {
    let limit = pagination.limit.unwrap_or(10);
    let offset = pagination.offset.unwrap_or(0);

    let rows: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT data FROM reports r
        WHERE score = (SELECT MAX(score) FROM reports WHERE mac_address = r.mac_address)
        GROUP BY mac_address
        ORDER BY score DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let results = rows
        .into_iter()
        .filter_map(|row| serde_json::from_str(&row).ok())
        .collect();

    Ok(Json(results))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", post(submit).get(get_leaderboard))
        .with_state(state)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let path = &args.database_url;
    if !path.is_empty() && !std::path::Path::new(path).exists() {
        tracing::info!("Creating database file at {}", path);
        std::fs::File::create(path)?;
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(format!("sqlite:{}", &args.database_url).as_str())
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS reports (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            mac_address TEXT NOT NULL,
            score INTEGER NOT NULL,
            timestamp TEXT NOT NULL,
            data TEXT NOT NULL
        );",
    )
    .execute(&pool)
    .await?;

    let shared_state = Arc::new(AppState { db: pool });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);

    let governor_conf = GovernorConfigBuilder::default()
        .per_second(5)
        .burst_size(10)
        .finish()
        .unwrap();

    let governor_limiter = governor_conf.limiter().clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(60));
            governor_limiter.retain_recent();
        }
    });

    let app = Router::new()
        .layer(cors)
        .layer(GovernorLayer::new(governor_conf))
        .merge(routes(shared_state));

    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("Listening on {}", addr);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();

    Ok(())
}
