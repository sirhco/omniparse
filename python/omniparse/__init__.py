"""
Omniparse - A Python library for detecting and extracting metadata, text, and content from various file formats.

This package provides Python bindings to the Omniparse Rust library, offering high-performance
file parsing and metadata extraction capabilities.
"""

try:
    from omniparse._omniparse import (
        extract_from_path,
        extract_from_bytes,
        supported_mime_types,
        is_mime_supported,
        ExtractionResult,
    )
except ImportError:
    # Fallback for development mode before building
    raise ImportError(
        "The omniparse native module is not available. "
        "Please build the package using 'maturin develop' or install from PyPI."
    )

__version__ = "0.2.0"

__all__ = [
    "extract_from_path",
    "extract_from_bytes",
    "supported_mime_types",
    "is_mime_supported",
    "ExtractionResult",
]
