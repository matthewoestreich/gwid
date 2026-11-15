# Default recipe: list all available commands
default:
    @just --list test
    @just --usage test

# Run unit tests
all:
    cargo nextest run

# Test v4
v4:
    cargo nextest run v4

# Test v7
v7:
    cargo nextest run v7
