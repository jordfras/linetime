#!/usr/bin/env bash

rm -rf target/debug
cargo build --release

# Set environment variables to ensure coloring and progress bar is attempted
# even though output is piped.
export CARGO_TERM_PROGRESS_WHEN=always
export CARGO_TERM_PROGRESS_WIDTH=100
export CARGO_TERM_COLOR=always

cargo build 2>&1 | target/release/linetime
