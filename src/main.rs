use std::{
    collections::HashMap,
    io::{self, Write},
};

use chrono::Utc;
use clap::Parser;
use colored::Colorize;
use mac_address::get_mac_address;
use serde::Serialize;
use sysinfo::System;

use slimes::{
    application_header,
    benchmark::{BenchmarkResults, run_benchmark_multithread, run_benchmark_singlethread},
    slimes::get_all_slimes,
    vprintln,
};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Skip CPU benchmark
    #[arg(short, long)]
    pub skip_benchmark: bool,

    /// Skil system info
    #[arg(short = 'S', long)]
    pub skip_system_info: bool,

    /// Benchmark: Upper limit for prime calculation (higher number = longer test)
    #[arg(short, long, default_value_t = 500_000)]
    pub prime_limit: u64,

    /// Benchmark: Enforce cpu thread amount to use.
    /// Defaults to detected cpu core count.
    #[arg(short, long)]
    pub jobs: Option<usize>,

    /// Don't send data to leaderboard server
    #[arg(short, long)]
    pub offline: bool,

    /// Leaderboard server URL to send report to
    #[arg(long, default_value = "https://alatreon.org/slimes")]
    pub server_url: String,

    /// Enable verbose output logging
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Serialize)]
struct FullReport {
    mac_address: String,
    timestamp: String,
    slimes: Option<HashMap<String, Vec<String>>>,
    benchmark: Option<BenchmarkReport>,
}

#[derive(Serialize)]
struct BenchmarkReport {
    prime_limit: u64,
    logical_cores: usize,
    single_thread: BenchmarkResults,
    multi_thread: BenchmarkResults,
}

fn main() {
    let cli = Cli::parse();
    let mut report = FullReport {
        mac_address: get_mac_address()
            .ok()
            .flatten()
            .map(|m| m.to_string())
            .unwrap_or_else(|| format!("unknown-mac#{}", Uuid::new_v4())),
        timestamp: Utc::now().to_rfc3339(),
        slimes: None,
        benchmark: None,
    };

    vprintln!(
        cli.verbose,
        "Initialized report {} {}",
        report.mac_address,
        report.timestamp
    );

    let mut report_slimes = HashMap::new();

    println!("{}", application_header().bright_blue());

    if !cli.skip_system_info {
        let slimes = get_all_slimes();

        let mut sys = System::new_all();
        sys.refresh_all();

        for slime in slimes {
            let label = slime.label().to_string();
            let values = slime.values(&sys, cli.verbose);

            slime.print_from_values(&values);
            report_slimes.insert(label, values);
        }
        println!();

        report.slimes = Some(report_slimes);
    }

    if !cli.skip_benchmark {
        let logical_core_count = match cli.jobs {
            Some(j) => j,
            None => num_cpus::get(),
        };

        print_section_header("Single Threaded CPU Benchmark");
        let singlethread_benchmark = run_benchmark_singlethread(cli.prime_limit, cli.verbose);
        print_detailed_result(&singlethread_benchmark);
        print_section_header("Multi Threaded CPU Benchmark");
        let multithread_benchmark =
            run_benchmark_multithread(cli.prime_limit, logical_core_count, cli.verbose);
        print_detailed_result(&multithread_benchmark);

        let multi_thread_speedup_ratio = if singlethread_benchmark.score > 0 {
            multithread_benchmark.score / singlethread_benchmark.score
        } else {
            0
        };

        let scaling_color_formatter =
            if multi_thread_speedup_ratio as f64 > (logical_core_count as f64 * 0.7) {
                |s: String| s.green()
            } else {
                |s: String| s.yellow()
            };

        println!(
            "Parallel Scaling  : {}",
            scaling_color_formatter(format!("{:.2}x", multi_thread_speedup_ratio)).bold()
        );

        report.benchmark = Some(BenchmarkReport {
            prime_limit: cli.prime_limit,
            logical_cores: logical_core_count,
            single_thread: singlethread_benchmark,
            multi_thread: multithread_benchmark,
        });
    }

    if !cli.offline {
        let json_payload = serde_json::to_string_pretty(&report).unwrap();
        vprintln!(cli.verbose, "Generated JSON report: {:?}", &json_payload);

        println!();
        if !confirm_upload() {
            return;
        }

        send_to_server(&cli.server_url, &report);
    }
}

fn confirm_upload() -> bool {
    print!("Send data to leaderboard server? [Y/n] ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    let input = input.trim().to_lowercase();

    input.is_empty() || input.starts_with('y')
}

fn send_to_server(url: &str, report: &FullReport) {
    let client = reqwest::blocking::Client::new();
    match client.post(url).json(&report).send() {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("{}", "Successfully uploaded results!".green().bold());
            } else {
                eprintln!(
                    "{} Error sending to server: {}",
                    "Error:".red(),
                    resp.status()
                );
            }
        }
        Err(e) => eprintln!("{} Failed to connect: {}", "Error:".red(), e),
    }
}

pub fn print_section_header(title_text: &str) {
    println!(
        "{}",
        format!("[ {} ]", title_text).white().bold().underline()
    );
}

pub fn print_detailed_result(results: &BenchmarkResults) {
    println!(
        "  Batch Count     : {} batch{}",
        results.batch_count,
        if results.batch_count > 1 { "es" } else { "" }
    );
    println!("  Total Duration  : {:.4}s", results.duration.as_secs_f64());
    println!("  Primes Found    : {}", results.primes_found);
    println!(
        "  Calculated Score: {}",
        format!("{}", results.score).green().bold()
    );
}
