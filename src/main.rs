use clap::Parser;
use colored::Colorize;
use slimefetch::application_header;

mod benchmark;
use crate::benchmark::{BenchmarkResults, run_benchmark_multithread, run_benchmark_singlethread};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Skip CPU benchmark
    #[arg(short, long)]
    pub skip_benchmark: bool,

    /// Benchmark: Upper limit for prime calculation (higher number = longer test)
    #[arg(short, long, default_value_t = 500_000)]
    pub prime_limit: u64,

    /// Benchmark: Enforce cpu thread amount to use.
    /// Defaults to detected cpu core count.
    #[arg(short, long)]
    pub jobs: Option<usize>,

    /// Enable verbose output logging
    #[arg(short, long)]
    pub verbose: bool,
}

fn main() {
    let cli = Cli::parse();

    println!("{}", application_header().bright_blue());

    let logical_core_count = match cli.jobs {
        Some(j) => j,
        None => num_cpus::get(),
    };

    print_section_header("Single Threaded CPU Benchmark");
    let singlethread_benchmark = run_benchmark_singlethread(cli.prime_limit);
    print_detailed_result(&singlethread_benchmark);
    print_section_header("Multi Threaded CPU Benchmark");
    let multithread_benchmark = run_benchmark_multithread(cli.prime_limit, logical_core_count);
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
