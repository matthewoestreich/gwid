use rand::RngExt as _;
use std::{
    hint::black_box,
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};

fn main() {
    // let mut concurrent_thread_counts = vec![1, 2, 4, 8, 12, 16, 24, 32, 64];
    // Shuffle order we test threads in. Do this 10 times to really shake things up.
    // for _ in 0..10 {
    //     shuffle_vec(&mut concurrent_thread_counts);
    // }
    let concurrent_thread_counts = vec![32];
    benchmark_concurrent_multi_threaded_throughput(ConcurrentThroughputConfig {
        ids_per_run: 100_000_000, // 100_000_000,
        warmup_ids: 1_000_000,
        runs: 10,
        thread_counts: concurrent_thread_counts,
    });
}

struct ConcurrentThroughputConfig {
    ids_per_run: u64,
    warmup_ids: u64,
    runs: usize,
    thread_counts: Vec<usize>,
}

#[allow(dead_code)]
struct BenchmarkRun {
    generated: u64,
    elapsed: Duration,
    ids_per_second: f64,
}

fn benchmark_concurrent_multi_threaded_throughput(config: ConcurrentThroughputConfig) {
    assert!(
        config.ids_per_run > 0,
        "ids_per_run must be greater than zero"
    );
    assert!(config.runs > 0, "runs must be greater than zero");
    assert!(
        !config.thread_counts.is_empty(),
        "thread_counts cannot be empty"
    );
    assert!(
        config.thread_counts.iter().all(|&n| n > 0),
        "thread counts must be greater than zero"
    );

    println!(
        "=== Concurrent throughput (generate IDs as fast as possible over multiple threads) ==="
    );
    println!("  IDs per run: {}", with_commas(config.ids_per_run));
    println!("  Warmup IDs: {}", with_commas(config.warmup_ids));
    println!("  Runs per thread count: {}", config.runs);
    println!("  Thread counts order: {:?}", config.thread_counts);

    for &num_threads in &config.thread_counts {
        println!("\n  Testing {num_threads} threads:");

        // Warmup
        run_concurrent_generation(config.warmup_ids, num_threads);

        let mut results = Vec::with_capacity(config.runs);

        // Measured runs
        for run in 1..=config.runs {
            let elapsed = run_concurrent_generation(config.ids_per_run, num_threads);

            let ids_per_second = config.ids_per_run as f64 / elapsed.as_secs_f64();

            let result = BenchmarkRun {
                generated: config.ids_per_run,
                elapsed,
                ids_per_second,
            };

            println!(
                "    Run {run}: {} IDs/sec ({:.2}ms)",
                with_commas(format!("{:.2}", result.ids_per_second)),
                result.elapsed.as_secs_f64() * 1000.0,
            );

            results.push(result);
        }

        // Sort by throughput to calculate median.
        results.sort_by(|a, b| a.ids_per_second.total_cmp(&b.ids_per_second));

        let median = results[results.len() / 2].ids_per_second;
        let min = results.first().unwrap().ids_per_second;
        let max = results.last().unwrap().ids_per_second;

        println!("\n    Results:");
        println!(
            "      Generated per run:  {}",
            with_commas(config.ids_per_run)
        );
        println!(
            "      Median throughput:  {} IDs/sec",
            with_commas(format!("{median:.2}"))
        );
        println!(
            "      Minimum throughput: {} IDs/sec",
            with_commas(format!("{min:.2}"))
        );
        println!(
            "      Maximum throughput: {} IDs/sec",
            with_commas(format!("{max:.2}"))
        );
    }
}

fn run_concurrent_generation(total_ids: u64, num_threads: usize) -> Duration {
    let barrier = Arc::new(Barrier::new(num_threads + 1));
    let mut handles = Vec::with_capacity(num_threads);

    let ids_per_thread = total_ids / num_threads as u64;
    let remainder = total_ids % num_threads as u64;

    for thread_index in 0..num_threads {
        let barrier = Arc::clone(&barrier);

        let count = ids_per_thread + u64::from(thread_index < remainder as usize);

        handles.push(thread::spawn(move || {
            // Wait until all workers are ready.
            barrier.wait();

            for _ in 0..count {
                let id = gwid::v7::generate_raw().unwrap();
                black_box(id);
            }
        }));
    }

    // Start timing immediately before releasing the workers.
    let start = Instant::now();

    barrier.wait();

    for handle in handles {
        handle.join().unwrap();
    }

    start.elapsed()
}

fn with_commas<T: std::fmt::Display>(value: T) -> String {
    let value = value.to_string();
    let (integer, decimal) = value.split_once('.').unwrap_or((&value, ""));

    let formatted = integer
        .chars()
        .rev()
        .enumerate()
        .fold(String::new(), |mut result, (i, c)| {
            if i > 0 && i % 3 == 0 {
                result.push(',');
            }
            result.push(c);
            result
        })
        .chars()
        .rev()
        .collect::<String>();

    if decimal.is_empty() {
        formatted
    } else {
        format!("{formatted}.{decimal}")
    }
}

#[allow(dead_code)]
fn shuffle_vec<T>(values: &mut [T]) {
    let mut rng = rand::rng();

    for i in (1..values.len()).rev() {
        let j = rng.random_range(0..=i);
        values.swap(i, j);
    }
}
