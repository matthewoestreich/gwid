// Gungraun benchmarks require the `gungraun-runner` binary and Valgrind, which
// aren't available on wasm32 targets, so the harness is compiled out there.
#![allow(dead_code)]

#[cfg(not(target_arch = "wasm32"))]
mod bench {
    use std::hint::black_box;

    use gungraun::prelude::*;

    #[cfg(feature = "v7-global-monotonicity")]
    #[library_benchmark]
    fn generate_raw_global_mono() -> u128 {
        black_box(gwid::v7::generate_raw().unwrap())
    }

    #[cfg(feature = "v7-global-monotonicity")]
    #[library_benchmark]
    fn generate_strict_raw_global_mono() -> u128 {
        black_box(gwid::v7::generate_strict_raw().unwrap())
    }

    #[cfg(feature = "v7-global-monotonicity")]
    library_benchmark_group!(
        name = v7gungraun,
        benchmarks = [generate_raw_global_mono, generate_strict_raw_global_mono,]
    );

    #[cfg(not(feature = "v7-global-monotonicity"))]
    #[library_benchmark]
    fn generate_raw() -> u128 {
        black_box(gwid::v7::generate_raw().unwrap())
    }

    #[cfg(not(feature = "v7-global-monotonicity"))]
    #[library_benchmark]
    fn generate_strict_raw() -> u128 {
        black_box(gwid::v7::generate_strict_raw().unwrap())
    }

    #[cfg(not(feature = "v7-global-monotonicity"))]
    library_benchmark_group!(
        name = v7gungraun,
        benchmarks = [generate_raw, generate_strict_raw,]
    );
}

#[cfg(not(target_arch = "wasm32"))]
use bench::v7gungraun;

#[cfg(not(target_arch = "wasm32"))]
gungraun::main!(library_benchmark_groups = v7gungraun);

#[cfg(not(not(target_arch = "wasm32")))]
fn main() {}
