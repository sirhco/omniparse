"""
Pytest configuration and fixtures for omniparse tests.
"""

import pytest
import os


@pytest.fixture(scope="session")
def test_data_dir():
    """Provide the path to the test data directory."""
    return "test_data"


@pytest.fixture(scope="session")
def sample_files(test_data_dir):
    """Provide paths to sample test files."""
    return {
        "text": {
            "plain": os.path.join(test_data_dir, "text", "sample.txt"),
            "json": os.path.join(test_data_dir, "text", "sample.json"),
            "csv": os.path.join(test_data_dir, "text", "sample.csv"),
            "xml": os.path.join(test_data_dir, "text", "sample.xml"),
        },
        "document": {
            "pdf": os.path.join(test_data_dir, "document", "sample.pdf"),
            "docx": os.path.join(test_data_dir, "document", "sample.docx"),
            "odt": os.path.join(test_data_dir, "document", "sample.odt"),
        },
        "image": {
            "png": os.path.join(test_data_dir, "image", "sample.png"),
            "jpg": os.path.join(test_data_dir, "image", "sample.jpg"),
            "tiff": os.path.join(test_data_dir, "image", "sample.tiff"),
        },
        "archive": {
            "zip": os.path.join(test_data_dir, "archive", "sample.zip"),
            "tar": os.path.join(test_data_dir, "archive", "sample.tar"),
        },
    }
