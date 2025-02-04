#!/usr/bin/env bash

echo "Building linetime (no unfolding when run without linetime)"
scripts/build_with_progress.sh
echo ""

echo "Building linetime (unfolding when run with linetime)"
scripts/build_with_progress.sh 2>&1 | target/release/linetime