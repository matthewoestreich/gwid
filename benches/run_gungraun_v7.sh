#!/usr/bin/env bash

# Resolve repository root (parent of benches/)
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

IMAGE_NAME="gungraun-valgrind"

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
  "$IMAGE_NAME" \
  bash -c '
    cargo bench \
      --bench gungraun_v7 \
      --no-default-features \
      --target x86_64-unknown-linux-gnu &&
    cargo bench \
      --bench gungraun_v7 \
      --all-features \
      --target x86_64-unknown-linux-gnu
  '
