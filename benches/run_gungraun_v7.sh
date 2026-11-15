#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE_NAME="gungraun-valgrind"
PROFILE="${1:-}"
GUNGRAUN_HOME="/workspace/benches"

# Build the image using the Dockerfile in benches/
docker build \
  -f "$ROOT_DIR/benches/Dockerfile.gungraun" \
  -t "$IMAGE_NAME" \
  "$ROOT_DIR"

# Run benchmarks from the repository root
docker run --rm -it \
  --privileged \
  -v "$ROOT_DIR":/workspace \
  -w /workspace \
  -e GUNGRAUN_HOME="$GUNGRAUN_HOME" \
  "$IMAGE_NAME" \
  bash -euo pipefail -c '
    profile_args=()

    if [[ -n "$1" ]]; then
      profile_args+=(--profile "$1")
    fi

    printf "\n~~~~~ GLOBAL MONOTONICITY: DISABLED ~~~~~\n"
    cargo bench --quiet \
      --bench gungraun_v7 \
      --target x86_64-unknown-linux-gnu \
      "${profile_args[@]}"

    printf "\n~~~~~ GLOBAL MONOTONICITY: ENABLED ~~~~~\n"
    cargo bench --quiet \
      --bench gungraun_v7 \
      --features v7-global-monotonicity \
      --target x86_64-unknown-linux-gnu \
      "${profile_args[@]}"
  ' _ "$PROFILE"
