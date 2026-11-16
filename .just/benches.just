# Default recipe: list all available commands
default:
    @just --list benches
    @just --usage benches

# Run benches
all:
    cargo bench
    just benches v7_gungraun 

# Bench v7
[arg('all', long='all', value='--all-features', help='Enable all features')]
[arg('crypto', long='crypto', value='v7-crypto-rand', help='Enable v7-crypto-rand feature')]
[arg('global', long='global', value='v7-global-monotonicity', help='Enable v7-global-monotonicity feature')]
v7 all='' global='' crypto='':
    #!/usr/bin/env bash
    if [[ -n "$all" ]]; then 
        cargo bench --bench v7 {{ all }}
    elif [[ -z "$global" && -z "$crypto" ]]; then
        cargo bench --bench v7
    else 
        cargo bench --bench v7 --features '{{ global }} {{ crypto }}'       
    fi

# Bench v7 gungraun
v7_gungraun:
    ../benches/run_gungraun_v7.sh 

# Bench v7 gungraun with debug symbols
v7_gungraun_debug:
    #!/usr/bin/env bash

    PROFILE="bench-debug"

    BENCH_SNIPPET="$(cat <<EOF
    printf "\nPROFILE=%s\n" "$PROFILE"

    printf "\n~~~~~ GLOBAL MONOTONICITY: DISABLED ~~~~~\n"
    cargo bench --quiet \
      --bench gungraun_v7 \
      --target x86_64-unknown-linux-gnu \
      --profile "$PROFILE"

    printf "\n~~~~~ GLOBAL MONOTONICITY: ENABLED ~~~~~\n"
    cargo bench --quiet \
      --bench gungraun_v7 \
      --features v7-global-monotonicity \
      --target x86_64-unknown-linux-gnu \
      --profile "$PROFILE"
    EOF
    )"

    # NOT ON LINUX, REQUIRES DOCKER
    if [[ "$(uname -s)" != "Linux" ]]; then
        if ! docker info >/dev/null 2>&1; then
            printf "\033[31mERROR: This benchmark requires Docker!\033[0m\n"
            printf "\033[31mERROR: Either Docker is not installed or Docker is not running!\033[0m\n"
            exit 69
        fi

        printf "not running on Linux, using Docker!"
        ../benches/run_gungraun_v7.sh "$BENCH_SNIPPET"
    else
        # RUNNING ON LINUX
        printf "running on Linux! Checking for dependencies..."

        # Check if gungraun-runner is installed
        GUNGRAUN_RUNNER_VERSION=$(gungraun-runner --version 2>/dev/null)
        if [[ -z "$GUNGRAUN_RUNNER_VERSION" ]]; then
            printf "\033[31mERROR: gungraun-runner is required!\033[0m\n"
            printf "\033[31mERROR: Please use\033[0m `cargo install --version 0.20.0 gungraun-runner` \033[31mto install it!\033[0m\n"
        else
          echo "Version: $version_output"
        fi

        eval "$BENCH_SNIPPET"
    fi

