#!/bin/bash
# Void Grid 3v3 CTF Run Script

# Get directory of this script
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
cd "$DIR"

echo "=== VOID GRID CTF INITIALIZER ==="

if [ ! -d "venv" ]; then
    echo "Virtual environment not found. Building..."
    python3 -m venv venv
    source venv/bin/activate
    pip install --upgrade pip
    pip install tornado numpy requests
else
    source venv/bin/activate
fi
# Setup is complete, launching Tornado server

echo "Launching Tornado server at http://localhost:8080/"
echo "Open your browser to start the simulation!"
echo "Press Ctrl+C to shutdown grid."

python server.py
