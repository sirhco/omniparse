# Python Bindings Development Guide

This guide covers the development workflow for the Omniparse Python bindings.

## Prerequisites

- Rust toolchain (1.70 or later)
- Python 3.8 or later
- pip or pip3

## Installation

### 1. Install Maturin

Maturin is the build tool for PyO3-based Python extensions.

```bash
pip install maturin
```

Or using pip3:

```bash
pip3 install maturin
```

### 2. Install Development Dependencies

```bash
pip install pytest pytest-cov
```

## Development Workflow

### Building in Development Mode

The `maturin develop` command builds the extension and installs it in your current Python environment:

```bash
# Build and install in development mode
maturin develop

# Build with release optimizations (faster runtime, slower build)
maturin develop --release
```

This allows you to import and test the module immediately:

```python
import omniparse
result = omniparse.extract_from_path("test_data/document/sample.pdf")
print(result.mime_type)
```

### Running Tests

After building with `maturin develop`, run the Python tests:

```bash
# Run all tests
pytest python/tests/

# Run with verbose output
pytest python/tests/ -v

# Run specific test file
pytest python/tests/test_basic.py

# Run with coverage report
pytest python/tests/ --cov=omniparse --cov-report=html
```

### Development Cycle

1. Make changes to Rust code in `src/python/`
2. Rebuild: `maturin develop`
3. Run tests: `pytest python/tests/`
4. Iterate

### Building Wheels

To build distributable wheels:

```bash
# Build wheel for current platform
maturin build --release

# Build for specific Python versions
maturin build --release --interpreter python3.8 python3.9 python3.10 python3.11 python3.12

# Output will be in target/wheels/
```

### Testing Built Wheels

```bash
# Install from wheel
pip install target/wheels/omniparse-*.whl --force-reinstall

# Test installation
python -c "import omniparse; print(omniparse.supported_mime_types())"
```

## Project Structure

```
omniparse/
├── src/python/          # PyO3 binding code
│   ├── mod.rs          # Module entry point
│   ├── types.rs        # Python type wrappers
│   ├── functions.rs    # Python functions
│   └── errors.rs       # Error conversions
├── python/
│   ├── omniparse/      # Python package
│   │   ├── __init__.py
│   │   └── __init__.pyi
│   └── tests/          # Python tests
│       ├── test_basic.py
│       ├── test_formats.py
│       ├── test_errors.py
│       ├── test_query.py
│       └── test_performance.py
├── pyproject.toml      # Python package configuration
└── Cargo.toml          # Rust package configuration
```

## Troubleshooting

### Import Error After Building

If you get import errors, ensure you've run `maturin develop` after making changes:

```bash
maturin develop --release
```

### Python Version Mismatch

Maturin uses the Python interpreter in your current environment. To target a specific version:

```bash
maturin develop --interpreter python3.10
```

### Clean Build

If you encounter build issues, try a clean build:

```bash
cargo clean
maturin develop --release
```

## CI/CD

The project uses GitHub Actions for automated testing and wheel building. See `.github/workflows/python-bindings.yml` for the CI configuration.

## Publishing

Wheels are automatically built and published to PyPI when a new tag is pushed:

```bash
git tag v0.2.0
git push origin v0.2.0
```

## Additional Resources

- [PyO3 Documentation](https://pyo3.rs/)
- [Maturin Documentation](https://www.maturin.rs/)
- [Python Packaging Guide](https://packaging.python.org/)
