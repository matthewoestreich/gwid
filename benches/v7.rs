#![allow(dead_code)]

use rand::RngExt as _;
use std::{
    hint::black_box,
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};

const RANDOM_SPACE: f64 = (1 << 20) as f64;

fn main() {
    run_benchmark_concurrent_multi_threaded_throughput(|| gwid::v7::generate_raw().unwrap());
    // run_concurrent_multi_threaded_collision_distribution(|| gwid::v7::generate_raw().unwrap());
}

// -----------------------------------------------------------------------------------------------------------
// ----------------------------------------------------------------------------------------------------------
// I MADE THESE 'run_*' METHODS SO ITS EASIER TO COMMENT OUT BENCHMARKS WE DONT WANT TO RUN INSIDE OF 'main'!
// -----------------------------------------------------------------------------------------------------------
// -----------------------------------------------------------------------------------------------------------

fn run_benchmark_concurrent_multi_threaded_throughput<F>(id_fn: F)
where
    F: Fn() -> u128 + Send + Sync + 'static,
{
    let mut concurrent_thread_counts = vec![2, 4, 8, 12, 16, 24, 32, 64];
    // Shuffle order we test threads in. Do this 10 times to really shake things up.
    for _ in 0..10 {
        shuffle_vec(&mut concurrent_thread_counts);
    }
    benchmark_concurrent_multi_threaded_throughput(
        id_fn,
        ConcurrentThroughputConfig {
            ids_per_run: 100_000_000,
            warmup_ids: 1_000,
            runs: 10,
            thread_counts: concurrent_thread_counts,
        },
    );
}

fn run_concurrent_multi_threaded_collision_distribution<F>(id_fn: F)
where
    F: Fn() -> u128 + Send + Sync + 'static,
{
    benchmark_concurrent_multi_threaded_collision_distribution(
        id_fn,
        ConcurrentCollisionDistributionConfig {
            ids_to_generate: 10_000_000,
            threads: 10,
            runs: 5,
        },
    );
}

struct BenchmarkRun {
    generated: u64,
    elapsed: Duration,
    ids_per_second: f64,
}

struct CollisionDistributionRun {
    total_ids: u64,
    unique_ids: u64,
    total_collisions: u64,
    milliseconds_occupied: u64,
    max_ids_per_ms: u64,
    max_collisions_per_ms: u64,
    expected_collisions: f64,
    elapsed: std::time::Duration,
}

struct ConcurrentThroughputConfig {
    ids_per_run: u64,
    warmup_ids: u64,
    runs: usize,
    thread_counts: Vec<usize>,
}

struct ConcurrentCollisionDistributionConfig {
    ids_to_generate: u64,
    threads: usize,
    runs: usize,
}

fn benchmark_concurrent_multi_threaded_throughput<F>(id_fn: F, config: ConcurrentThroughputConfig)
where
    F: Fn() -> u128 + Send + Sync + 'static,
{
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
        "\n=== Concurrent throughput (generate IDs as fast as possible over multiple threads) ==="
    );
    println!("  IDs per run: {}", with_commas(config.ids_per_run));
    println!("  Warmup IDs: {}", with_commas(config.warmup_ids));
    println!("  Runs per thread count: {}", config.runs);
    println!("  Thread counts order: {:?}", config.thread_counts);

    for &num_threads in &config.thread_counts {
        println!("\n  Testing {num_threads} threads:");

        // Warmup
        run_concurrent_generation(config.warmup_ids, num_threads, &id_fn);

        let mut results = Vec::with_capacity(config.runs);

        // Measured runs
        for run in 1..=config.runs {
            let elapsed = run_concurrent_generation(config.ids_per_run, num_threads, &id_fn);

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

fn benchmark_concurrent_multi_threaded_collision_distribution<F>(
    id_fn: F,
    config: ConcurrentCollisionDistributionConfig,
) where
    F: Fn() -> u128 + Send + Sync + 'static,
{
    assert!(config.ids_to_generate > 0);
    assert!(config.threads > 0);
    assert!(config.runs > 0);

    println!("\n=== Collision Distribution (concurrent, multiple threaded) ===");
    println!("  IDs per run: {}", with_commas(config.ids_to_generate));
    println!("  Threads: {}", config.threads);
    println!("  Runs: {}", config.runs);

    let mut results = Vec::with_capacity(config.runs);

    let f = Arc::new(id_fn);

    for run in 1..=config.runs {
        let barrier = Arc::new(Barrier::new(config.threads + 1));
        let mut handles = Vec::with_capacity(config.threads);

        let ids_per_thread = config.ids_to_generate / config.threads as u64;
        let remainder = config.ids_to_generate % config.threads as u64;

        // Spawn workers and distribute the workload.
        for thread_index in 0..config.threads {
            let barrier = Arc::clone(&barrier);
            let count = ids_per_thread + u64::from(thread_index < remainder as usize);
            let f = Arc::clone(&f);

            handles.push(thread::spawn(move || {
                let mut ids = Vec::with_capacity(count as usize);

                // Wait until all workers are ready.
                barrier.wait();

                for _ in 0..count {
                    let id = f();
                    ids.push(id);
                    black_box(id);
                }

                ids
            }));
        }

        // Release all workers simultaneously.
        let start = Instant::now();
        barrier.wait();

        // Collect each worker's results.
        let mut thread_results = Vec::with_capacity(config.threads);

        for handle in handles {
            thread_results.push(handle.join().unwrap());
        }

        let elapsed = start.elapsed();

        // Merge results after generation has finished.
        let mut ids = Vec::with_capacity(config.ids_to_generate as usize);

        for thread_ids in thread_results {
            ids.extend(thread_ids);
        }

        // Sort globally so cross-thread collisions are detected too.
        ids.sort_unstable();

        let mut total_collisions = 0u64;
        let mut milliseconds_occupied = 0u64;
        let mut max_ids_per_ms = 0u64;
        let mut max_collisions_per_ms = 0u64;
        let mut expected_collisions = 0.0f64;

        let mut index = 0;

        while index < ids.len() {
            let timestamp = ids[index];
            let bucket_start = index;

            // Find the end of this timestamp bucket.
            while index < ids.len() && ids[index] == timestamp {
                index += 1;
            }

            let bucket = &ids[bucket_start..index];
            let bucket_size = bucket.len() as u64;

            milliseconds_occupied += 1;
            max_ids_per_ms = max_ids_per_ms.max(bucket_size);

            // Expected collisions using the birthday-problem approximation.
            expected_collisions +=
                (bucket_size * bucket_size.saturating_sub(1)) as f64 / (2.0 * RANDOM_SPACE);

            // Count duplicate random values.
            let bucket_collisions =
                bucket.windows(2).filter(|pair| pair[0] == pair[1]).count() as u64;

            total_collisions += bucket_collisions;

            max_collisions_per_ms = max_collisions_per_ms.max(bucket_collisions);
        }

        let total_ids = ids.len() as u64;
        let unique_ids = total_ids - total_collisions;

        let result = CollisionDistributionRun {
            total_ids,
            unique_ids,
            total_collisions,
            milliseconds_occupied,
            max_ids_per_ms,
            max_collisions_per_ms,
            expected_collisions,
            elapsed,
        };

        let expected_0_collisions_got_0_collisions =
            result.expected_collisions == 0.0 && result.total_collisions == 0;
        let had_collisions = !expected_0_collisions_got_0_collisions;

        println!("\n  Run {run}: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
        println!(
            "    Total IDs: _ _ _ _ _ _ _ _ _ _ {}",
            with_commas(result.total_ids)
        );
        println!(
            "    Unique IDs: __ _ _ _ _ _ _ _ _ {}",
            with_commas(result.unique_ids)
        );
        println!(
            "    Expected collisions: _ _ _ _ _ {:.2}",
            result.expected_collisions
        );
        println!(
            "    Actual (Total) collisions: _ _ {}",
            with_commas(result.total_collisions)
        );

        if had_collisions {
            println!(
                "    Actual / expected: _ _ _ _ _ _ {:.2}x",
                result.total_collisions as f64 / result.expected_collisions
            );
            println!(
                "    Expected collision percentage: {:.6}%",
                result.expected_collisions / result.total_ids as f64 * 100.0
            );
            println!(
                "    Actual collision percentage: _ {:.6}%",
                result.total_collisions as f64 / result.total_ids as f64 * 100.0
            );
        }

        println!(
            "    Milliseconds occupied: _ _ _ _ {}",
            with_commas(result.milliseconds_occupied)
        );
        println!(
            "    Average IDs/ms: __ _ _ _ _ _ _ {:.2}",
            result.total_ids as f64 / result.milliseconds_occupied as f64
        );
        println!(
            "    Maximum IDs/ms: __ _ _ _ _ _ _ {}",
            with_commas(result.max_ids_per_ms)
        );

        if had_collisions {
            println!(
                "    Maximum collisions/ms: _ _ _ _ {}",
                with_commas(result.max_collisions_per_ms)
            );
        }

        println!(
            "    Throughput: __ _ _ _ _ _ _ _ _ {} IDs/sec",
            with_commas(format!(
                "{:.2}",
                result.total_ids as f64 / result.elapsed.as_secs_f64()
            ))
        );

        results.push(result);
    }
}

fn run_concurrent_generation<F>(total_ids: u64, num_threads: usize, run_fn: &F) -> Duration
where
    F: Fn() -> u128 + Send + Sync,
{
    let ids_per_thread = total_ids / num_threads as u64;
    let remainder = total_ids % num_threads as u64;

    let barrier = Arc::new(Barrier::new(num_threads + 1));

    thread::scope(|scope| {
        let mut handles = Vec::with_capacity(num_threads);

        for thread_index in 0..num_threads {
            let barrier = Arc::clone(&barrier);
            let count = ids_per_thread + u64::from(thread_index < remainder as usize);

            handles.push(scope.spawn(move || {
                barrier.wait();

                for _ in 0..count {
                    let id = run_fn();
                    black_box(id);
                }
            }));
        }

        let start = Instant::now();

        barrier.wait();

        for handle in handles {
            handle.join().unwrap();
        }

        start.elapsed()
    })
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
