#!/usr/bin/env python3
"""
Script to validate Python documentation completeness
"""
import sys
import subprocess
from pathlib import Path


def check_readme():
    """Check README_PYTHON.md exists and has required sections"""
    print("1. Checking README_PYTHON.md...")
    
    readme_path = Path("README_PYTHON.md")
    if not readme_path.exists():
        print("  ✗ README_PYTHON.md not found")
        return False
    
    content = readme_path.read_text()
    
    required_sections = [
        "# Omniparse Python Bindings",
        "## Features",
        "## Installation",
        "## Quick Start",
        "## API Reference",
        "## Error Handling",
        "## Performance",
        "## Examples",
    ]
    
    missing = []
    for section in required_sections:
        if section not in content:
            missing.append(section)
    
    if missing:
        print(f"  ✗ Missing sections: {', '.join(missing)}")
        return False
    
    print(f"  ✓ README_PYTHON.md complete ({len(content)} chars)")
    return True


def check_type_stubs():
    """Check type stubs exist and are valid"""
    print("\n2. Checking type stubs...")
    
    stub_path = Path("python/omniparse/__init__.pyi")
    if not stub_path.exists():
        print("  ✗ __init__.pyi not found")
        return False
    
    content = stub_path.read_text()
    
    required_items = [
        "class ExtractionResult",
        "def extract_from_path",
        "def extract_from_bytes",
        "def supported_mime_types",
        "def is_mime_supported",
    ]
    
    missing = []
    for item in required_items:
        if item not in content:
            missing.append(item)
    
    if missing:
        print(f"  ✗ Missing items: {', '.join(missing)}")
        return False
    
    print(f"  ✓ Type stubs complete ({len(content)} chars)")
    
    # Check py.typed marker
    marker_path = Path("python/omniparse/py.typed")
    if not marker_path.exists():
        print("  ✗ py.typed marker not found")
        return False
    
    print("  ✓ py.typed marker present")
    return True


def check_examples():
    """Check example scripts exist"""
    print("\n3. Checking example scripts...")
    
    examples = [
        "examples/python/basic_usage.py",
        "examples/python/batch_processing.py",
        "examples/python/metadata_extraction.py",
    ]
    
    all_exist = True
    for example in examples:
        path = Path(example)
        if path.exists():
            print(f"  ✓ {example}")
        else:
            print(f"  ✗ {example} not found")
            all_exist = False
    
    return all_exist


def check_mypy():
    """Check type stubs work with mypy"""
    print("\n4. Checking mypy compatibility...")
    
    try:
        result = subprocess.run(
            ["python", "-m", "mypy", "examples/python/basic_usage.py", "--no-error-summary"],
            capture_output=True,
            text=True,
            timeout=30
        )
        
        if result.returncode == 0:
            print("  ✓ mypy validation passed")
            return True
        else:
            print(f"  ✗ mypy validation failed:")
            print(result.stdout)
            print(result.stderr)
            return False
    except subprocess.TimeoutExpired:
        print("  ✗ mypy check timed out")
        return False
    except FileNotFoundError:
        print("  ⚠ mypy not installed (skipping)")
        return True


def check_docstrings():
    """Check that Python module has docstrings"""
    print("\n5. Checking docstrings...")
    
    try:
        import omniparse
        
        # Check module docstring
        if not omniparse.__doc__:
            print("  ✗ Module missing docstring")
            return False
        
        # Check function docstrings
        functions = [
            omniparse.extract_from_path,
            omniparse.extract_from_bytes,
            omniparse.supported_mime_types,
            omniparse.is_mime_supported,
        ]
        
        missing = []
        for func in functions:
            if not func.__doc__:
                missing.append(func.__name__)
        
        if missing:
            print(f"  ✗ Missing docstrings: {', '.join(missing)}")
            return False
        
        # Check class docstring
        if not omniparse.ExtractionResult.__doc__:
            print("  ✗ ExtractionResult missing docstring")
            return False
        
        print("  ✓ All docstrings present")
        return True
    except ImportError as e:
        print(f"  ✗ Failed to import omniparse: {e}")
        return False


def main():
    """Run all validation checks"""
    print("=" * 80)
    print("Python Documentation Validation")
    print("=" * 80)
    
    checks = [
        check_readme,
        check_type_stubs,
        check_examples,
        check_mypy,
        check_docstrings,
    ]
    
    results = []
    for check in checks:
        try:
            results.append(check())
        except Exception as e:
            print(f"  ✗ Check failed with error: {e}")
            results.append(False)
    
    print("\n" + "=" * 80)
    passed = sum(results)
    total = len(results)
    
    if all(results):
        print(f"✓ All {total} validation checks passed!")
        return 0
    else:
        print(f"✗ {total - passed}/{total} checks failed")
        return 1


if __name__ == '__main__':
    sys.exit(main())
