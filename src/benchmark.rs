use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

// Factor determining how many times the calculation runs in the multi-threaded test.
pub const MULTI_THREAD_LOAD_FACTOR: usize = 32;

pub struct BenchmarkResults {
    pub duration: Duration,
    pub primes_found: u64,
    pub score: u64,
    pub batch_count: u64,
}

pub fn run_benchmark_singlethread(prime_limit: u64) -> BenchmarkResults {
    let start_time = Instant::now();
    let primes_found = calculate_primes(1, prime_limit);
    let duration = start_time.elapsed();

    let score = calculate_score(duration, 1, prime_limit);

    BenchmarkResults {
        duration,
        primes_found,
        score,
        batch_count: 1,
    }
}

pub fn run_benchmark_multithread(prime_limit: u64, jobs: usize) -> BenchmarkResults {
    let start_time = Instant::now();

    // The total number of calculation batches to perform across all threads
    let total_jobs_to_process = MULTI_THREAD_LOAD_FACTOR;

    // Atomic counter for distributing jobs to threads without heavy locking
    let shared_job_index_counter = Arc::new(AtomicUsize::new(0));
    // Mutex protected accumulator for the total primes found across all threads
    let shared_total_prime_count = Arc::new(Mutex::new(0));
    // Vector to store the execution duration of each individual thread
    let shared_thread_durations = Arc::new(Mutex::new(Vec::new()));

    let mut thread_handles = vec![];

    for _ in 0..jobs {
        let job_counter_reference = Arc::clone(&shared_job_index_counter);
        let prime_count_reference = Arc::clone(&shared_total_prime_count);
        let durations_reference = Arc::clone(&shared_thread_durations);
        let limit_per_run = prime_limit;

        let handle = thread::spawn(move || {
            let thread_execution_start = Instant::now();
            let mut local_thread_prime_count = 0;

            loop {
                // Fetch the next job index and increment atomically
                let job_index = job_counter_reference.fetch_add(1, Ordering::Relaxed);

                if job_index >= total_jobs_to_process {
                    break;
                }

                local_thread_prime_count += calculate_primes(1, limit_per_run);
            }

            let thread_execution_duration = thread_execution_start.elapsed();

            // Record thread duration
            durations_reference
                .lock()
                .unwrap()
                .push(thread_execution_duration);

            // Commit local count to the global sum
            let mut global_count_lock = prime_count_reference.lock().unwrap();
            *global_count_lock += local_thread_prime_count;
        });
        thread_handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in thread_handles {
        handle.join().unwrap();
    }

    let duration = start_time.elapsed();
    let final_count = *shared_total_prime_count.lock().unwrap();

    // Calculate average time a thread spent working
    // let durations_guard = shared_thread_durations.lock().unwrap();
    // let total_thread_microseconds: u128 = durations_guard.iter().map(|d| d.as_micros()).sum();

    // let average_thread_microseconds = if !durations_guard.is_empty() {
    //     total_thread_microseconds / durations_guard.len() as u128
    // } else {
    //     0
    // };
    // let average_thread_duration = Duration::from_micros(average_thread_microseconds as u64);

    // Score normalized by load factor to represent "Speed per unit of work"
    let score = calculate_score(duration, MULTI_THREAD_LOAD_FACTOR as u64, prime_limit);

    BenchmarkResults {
        duration,
        score,
        primes_found: final_count,
        batch_count: MULTI_THREAD_LOAD_FACTOR as u64,
    }
}

/// Performs the CPU-intensive prime number calculation.
pub fn calculate_primes(range_start: u64, range_end: u64) -> u64 {
    let mut prime_count = 0;
    let mut current_number = range_start;

    if current_number <= 2 {
        if range_end >= 2 {
            prime_count += 1;
        }
        current_number = 3;
    }

    if current_number.is_multiple_of(2) {
        current_number += 1;
    }

    while current_number <= range_end {
        if is_number_prime(current_number) {
            prime_count += 1;
        }
        current_number += 2;
    }
    prime_count
}

/// Helper function to check primality.
pub fn is_number_prime(number: u64) -> bool {
    if number <= 1 {
        return false;
    }

    let search_limit = (number as f64).sqrt() as u64;

    for i in (3..=search_limit).step_by(2) {
        if number.is_multiple_of(i) {
            return false;
        }
    }
    true
}

/// Calculates a score based on work done divided by time taken.
pub fn calculate_score(duration: Duration, batch_multiplier: u64, prime_limit: u64) -> u64 {
    let microseconds = duration.as_micros() as u64;

    if microseconds == 0 {
        return 0;
    }

    (prime_limit * batch_multiplier * 10_000) / microseconds
}
