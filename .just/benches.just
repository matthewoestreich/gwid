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
    # Docker either not installed or not running..
    if [[ "$OSTYPE" != linux* ]]; then
        if ! docker info >/dev/null 2>&1; then
            printf "\033[31mERROR: This benchmark requires Docker!\033[0m\n"
            printf "\033[31mERROR: Either Docker is not installed or Docker is not running!\033[0m\n"
        fi
            exit 69
    fi
    # Docker is installed & running
    PROFILE="bench-debug"
    ../benches/run_gungraun_v7.sh "$PROFILE"
