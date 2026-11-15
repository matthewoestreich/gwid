// Gungraun benchmarks require the `gungraun-runner` binary and Valgrind, which
// aren't available on wasm32 targets, so the harness is compiled out there.

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "macos"),))]
mod bench {
    use gungraun::prelude::*;
    use std::hint::black_box;

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
        name = v7gungraun_global_mono,
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

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "macos")))]
use gungraun::{Dhat, main, prelude::*};

// GLOBAL MONO ENABLED
#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "macos"),
    feature = "v7-global-monotonicity"
))]
use bench::v7gungraun_global_mono;

#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "macos"),
    feature = "v7-global-monotonicity"
))]
main!(
    config = LibraryBenchmarkConfig::default().tool(Dhat::default()),
    library_benchmark_groups = v7gungraun_global_mono
);

// GLOBAL MONO DISABLED
#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "macos"),
    not(feature = "v7-global-monotonicity")
))]
use bench::v7gungraun;

#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "macos"),
    not(feature = "v7-global-monotonicity")
))]
main!(
    config = LibraryBenchmarkConfig::default().tool(Dhat::default()),
    library_benchmark_groups = v7gungraun
);

// Stub entry point for platforms where the benchmarks aren't supported.
#[cfg(any(target_arch = "wasm32", target_os = "macos"))]
fn main() {}
