#!/usr/bin/env bash
set -e

cargo run --manifest-path backend/Cargo.toml &
(
  cd frontend && npm run serve
) &

wait
