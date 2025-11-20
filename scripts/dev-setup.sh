#!/bin/bash
# Development setup script for Omniparse Python bindings

set -e

echo "=== Omniparse Python Bindings Development Setup ==="
echo ""

# Check for Python
if ! command -v python3 &> /dev/null; then
    echo "Error: Python 3 is not installed"
    exit 1
fi

PYTHON_VERSION=$(python3 --version)
echo "✓ Found $PYTHON_VERSION"

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust is not installed"
    echo "Install from: https://rustup.rs/"
    exit 1
fi

RUST_VERSION=$(rustc --version)
echo "✓ Found $RUST_VERSION"

# Install maturin
echo ""
echo "Installing maturin..."
pip3 install maturin

# Install test dependencies
echo ""
echo "Installing test dependencies..."
pip3 install pytest pytest-cov

# Build in development mode
echo ""
echo "Building Python extension in development mode..."
maturin develop --release

# Run tests
echo ""
echo "Running tests..."
pytest python/tests/ -v

echo ""
echo "=== Setup Complete ==="
echo ""
echo "Development commands:"
echo "  maturin develop          # Rebuild after code changes"
echo "  pytest python/tests/     # Run tests"
echo "  maturin build --release  # Build wheel"
echo ""
echo "See PYTHON_DEVELOPMENT.md for more details"
