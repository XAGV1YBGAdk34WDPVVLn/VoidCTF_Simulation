#!/bin/bash
# Void Grid 3v3 CTF Run Script

# Get directory of this script
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
cd "$DIR"

echo "=== VOID GRID CTF INITIALIZER ==="

# Build the Rust application to make sure it is up to date
echo "Compiling Void Grid server..."
cargo build --release

echo "Launching Void Grid Rust server at http://localhost:8080/"
echo "Open your browser to start the simulation!"
echo "Press Ctrl+C to shutdown grid."

./target/release/voidgrid
