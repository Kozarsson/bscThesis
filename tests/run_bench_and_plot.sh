#!/bin/bash
set -e

# Run Rust benchmarks
cargo bench

# Change to the src directory and run the Python visualisation
cd src
python3 visualise.py