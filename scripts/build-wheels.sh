#!/bin/bash
# Script to build Python wheels for Omniparse
# This script builds wheels for the current platform

set -e

echo "Building Python wheels for Omniparse..."

# Check if maturin is installed
if ! command -v maturin &> /dev/null; then
    echo "Error: maturin is not installed"
    echo "Install with: pip install maturin"
    exit 1
fi

# Create dist directory if it doesn't exist
mkdir -p dist

# Build wheel for current platform
echo "Building wheel for current platform..."
maturin build --release --out dist

# List built wheels
echo ""
echo "Built wheels:"
ls -lh dist/*.whl

# Verify wheel contents
echo ""
echo "Verifying wheel contents..."
WHEEL=$(ls -t dist/*.whl | head -1)
echo "Latest wheel: $WHEEL"
python -m zipfile -l "$WHEEL"

echo ""
echo "Build complete!"
echo ""
echo "To build for multiple Python versions:"
echo "  maturin build --release --out dist --interpreter python3.8 python3.9 python3.10 python3.11 python3.12"
echo ""
echo "To build for all platforms, use the GitHub Actions workflow:"
echo "  .github/workflows/python-bindings.yml"
