use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{Method, StatusCode, header},
    routing::{get, post},
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::cors::{Any, CorsLayer};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
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
    #[serde(skip_deserializing)]
    pub id: Option<i32>,
    pub mac_address: String,
    pub timestamp: String,
    pub slimes: Option<serde_json::Value>,
    pub benchmark: Option<BenchmarkReport>,
    pub client_version: String,
    pub signature: String,
}

#[derive(Deserialize)]
pub struct Pagination {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub struct AppState {
    db: PgPool,
}

async fn submit(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<FullReport>,
) -> Result<(StatusCode, Json<i32>), (StatusCode, String)> {
    let score = payload
        .benchmark
        .as_ref()
        .map(|b| b.multi_thread.score)
        .unwrap_or(0);

    let raw_json = serde_json::to_value(&payload)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let row: (i32,) = sqlx::query_as(
        "INSERT INTO reports (mac_address, score, timestamp, client_version, signature, data) 
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id",
    )
    .bind(&payload.mac_address)
    .bind(score as i64)
    .bind(&payload.timestamp)
    .bind(&payload.client_version)
    .bind(&payload.signature)
    .bind(raw_json)
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(row.0)))
}

fn parse_report_row(id: i32, data: serde_json::Value) -> Option<FullReport> {
    let mut report: FullReport = serde_json::from_value(data).ok()?;
    report.id = Some(id);
    Some(report)
}

async fn get_leaderboard(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<FullReport>>, (StatusCode, String)> {
    let limit = pagination.limit.unwrap_or(10);
    let offset = pagination.offset.unwrap_or(0);

    let rows: Vec<(i32, serde_json::Value)> = sqlx::query_as(
        r#"
        SELECT id, data FROM (
            SELECT DISTINCT ON (mac_address) id, data, score
            FROM reports
            ORDER BY mac_address, score DESC
        ) sub
        ORDER BY score DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let results = rows
        .into_iter()
        .filter_map(|(id, data)| parse_report_row(id, data))
        .collect();

    Ok(Json(results))
}

async fn get_report_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<FullReport>, (StatusCode, String)> {
    let row: (i32, serde_json::Value) =
        sqlx::query_as("SELECT id, data FROM reports WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((StatusCode::NOT_FOUND, "Report not found".to_string()))?;

    let report = parse_report_row(row.0, row.1).ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to parse stored data".to_string(),
    ))?;

    Ok(Json(report))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", post(submit).get(get_leaderboard))
        .route("/{id}", get(get_report_by_id))
        .with_state(state)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&args.database_url)
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS reports (
            id SERIAL PRIMARY KEY,
            mac_address TEXT NOT NULL,
            score BIGINT NOT NULL,
            timestamp TEXT NOT NULL,
            client_version TEXT NOT NULL,
            signature TEXT,
            data JSONB NOT NULL
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
