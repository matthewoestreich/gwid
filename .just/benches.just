# Default recipe: list all available commands
default:
    @just --list benches

# Run benches
all:
    cargo bench
    just benches v7_gungraun 

# Bench v7
[arg('global', long='global', value='--features v7-global-monotonicity', help='Enable v7-global-monotonicity feature')]
v7 global='':
    cargo bench --bench v7 {{global}}

# Bench v7 gungraun
v7_gungraun:
    ../benches/run_gungraun_v7.sh 

# Bench v7 gungraun with debug symbols
v7_gungraun_debug:
    ../benches/run_gungraun_v7.sh bench-debug
