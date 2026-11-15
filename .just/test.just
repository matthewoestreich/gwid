# Default recipe: list all available commands
default:
    @just --list test
    @just --usage test

# Run all tests
all:
    cargo nextest run

# Test v4
v4:
    cargo nextest run v4

# Test v7
[arg('all', long='all', value='--all-features', help='Enable all features')]
[arg('crypto', long='crypto', value='v7-crypto-rand', help='Enable v7-crypto-rand feature')]
[arg('global', long='global', value='v7-global-monotonicity', help='Enable v7-global-monotonicity feature')]
v7 all='' global='' crypto='' *ARGS:
    #!/usr/bin/env bash
    if [[ -n "{{ all }}" ]]; then 
        printf "\n\nRUNNING ALL FFEATURES\n\n"
        cargo nextest run v7 {{ all }} {{ ARGS }}
    elif [[ -z "{{ global }}" && -z "{{ crypto }}" ]]; then
        printf "\n\nNO ARGS PROVIDED!\n\n"
        cargo nextest run v7 {{ ARGS }}
    else 
        printf "\n\nRUNNING FFEATURES: {{ global }} {{ crypto }}\n\n"
        cargo nextest run v7 --features '{{ global }} {{ crypto }}' {{ ARGS }}     
    fi
