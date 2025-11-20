"""
Basic integration tests for omniparse Python bindings.

Tests core functionality including extract_from_path, extract_from_bytes,
and result structure validation across multiple file formats.
"""

import pytest
import omniparse


class TestExtractFromPath:
    """Tests for extract_from_path function."""
    
    def test_extract_pdf(self):
        """Test extraction from PDF file."""
        result = omniparse.extract_from_path("test_data/document/sample.pdf")
        
        assert isinstance(result, omniparse.ExtractionResult)
        assert result.mime_type == "application/pdf"
        assert result.detection_confidence > 0.5
        assert isinstance(result.content, str)
        assert isinstance(result.metadata, dict)
        assert len(result.content) > 0
    
    def test_extract_json(self):
        """Test extraction from JSON file."""
        result = omniparse.extract_from_path("test_data/text/sample.json")
        
        assert result.mime_type == "application/json"
        assert result.detection_confidence > 0.5
        assert isinstance(result.content, str)
        assert isinstance(result.metadata, dict)
    
    def test_extract_csv(self):
        """Test extraction from CSV file."""
        result = omniparse.extract_from_path("test_data/text/sample.csv")
        
        assert result.mime_type == "text/csv"
        assert result.detection_confidence > 0.5
        assert isinstance(result.content, str)
        assert isinstance(result.metadata, dict)
    
    def test_extract_plain_text(self):
        """Test extraction from plain text file."""
        result = omniparse.extract_from_path("test_data/text/sample.txt")
        
        assert result.mime_type == "text/plain"
        assert result.detection_confidence > 0.5
        assert isinstance(result.content, str)
        assert isinstance(result.metadata, dict)
    
    def test_extract_xml(self):
        """Test extraction from XML file."""
        result = omniparse.extract_from_path("test_data/text/sample.xml")
        
        assert result.mime_type in ["application/xml", "text/xml"]
        assert result.detection_confidence > 0.5
        assert isinstance(result.content, str)
        assert isinstance(result.metadata, dict)
    
    def test_extract_docx(self):
        """Test extraction from DOCX file."""
        result = omniparse.extract_from_path("test_data/document/sample.docx")
        
        assert result.mime_type == "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        assert result.detection_confidence > 0.5
        assert isinstance(result.content, str)
        assert isinstance(result.metadata, dict)


class TestExtractFromBytes:
    """Tests for extract_from_bytes function."""
    
    def test_extract_from_bytes_json(self):
        """Test extraction from JSON bytes."""
        with open("test_data/text/sample.json", "rb") as f:
            data = f.read()
        
        result = omniparse.extract_from_bytes(data)
        
        assert isinstance(result, omniparse.ExtractionResult)
        assert result.mime_type == "application/json"
        assert result.detection_confidence > 0.5
        assert isinstance(result.content, str)
        assert isinstance(result.metadata, dict)
    
    def test_extract_from_bytes_with_hint(self):
        """Test extraction from bytes with MIME type hint."""
        with open("test_data/text/sample.csv", "rb") as f:
            data = f.read()
        
        result = omniparse.extract_from_bytes(data, mime_hint="text/csv")
        
        assert result.mime_type == "text/csv"
        assert isinstance(result.content, str)
        assert isinstance(result.metadata, dict)
    
    def test_extract_from_bytes_pdf(self):
        """Test extraction from PDF bytes."""
        with open("test_data/document/sample.pdf", "rb") as f:
            data = f.read()
        
        result = omniparse.extract_from_bytes(data)
        
        assert result.mime_type == "application/pdf"
        assert result.detection_confidence > 0.5
        assert isinstance(result.content, str)
    
    def test_extract_from_bytes_plain_text(self):
        """Test extraction from plain text bytes."""
        with open("test_data/text/sample.txt", "rb") as f:
            data = f.read()
        
        result = omniparse.extract_from_bytes(data)
        
        assert result.mime_type == "text/plain"
        assert isinstance(result.content, str)


class TestResultStructure:
    """Tests for ExtractionResult structure and properties."""
    
    def test_result_has_all_properties(self):
        """Test that result has all expected properties."""
        result = omniparse.extract_from_path("test_data/text/sample.json")
        
        # Check all properties exist
        assert hasattr(result, "mime_type")
        assert hasattr(result, "content")
        assert hasattr(result, "metadata")
        assert hasattr(result, "detection_confidence")
    
    def test_mime_type_is_string(self):
        """Test that mime_type is a string."""
        result = omniparse.extract_from_path("test_data/text/sample.txt")
        assert isinstance(result.mime_type, str)
        assert len(result.mime_type) > 0
    
    def test_content_types(self):
        """Test that content can be str, bytes, or None."""
        result = omniparse.extract_from_path("test_data/text/sample.txt")
        assert result.content is None or isinstance(result.content, (str, bytes))
    
    def test_metadata_is_dict(self):
        """Test that metadata is a dictionary."""
        result = omniparse.extract_from_path("test_data/text/sample.json")
        assert isinstance(result.metadata, dict)
    
    def test_detection_confidence_range(self):
        """Test that detection_confidence is between 0 and 1."""
        result = omniparse.extract_from_path("test_data/text/sample.txt")
        assert isinstance(result.detection_confidence, float)
        assert 0.0 <= result.detection_confidence <= 1.0
    
    def test_result_repr(self):
        """Test that result has a readable string representation."""
        result = omniparse.extract_from_path("test_data/text/sample.txt")
        repr_str = repr(result)
        
        assert isinstance(repr_str, str)
        assert "ExtractionResult" in repr_str
        assert result.mime_type in repr_str


class TestMultipleFormats:
    """Tests extraction across multiple file formats."""
    
    @pytest.mark.parametrize("file_path,expected_mime", [
        ("test_data/text/sample.txt", "text/plain"),
        ("test_data/text/sample.json", "application/json"),
        ("test_data/text/sample.csv", "text/csv"),
        ("test_data/text/sample.xml", "application/xml"),
        ("test_data/document/sample.pdf", "application/pdf"),
    ])
    def test_multiple_text_formats(self, file_path, expected_mime):
        """Test extraction from various text-based formats."""
        result = omniparse.extract_from_path(file_path)
        
        # XML can be detected as either application/xml or text/xml
        if expected_mime == "application/xml":
            assert result.mime_type in ["application/xml", "text/xml"]
        else:
            assert result.mime_type == expected_mime
        
        assert result.detection_confidence > 0.0
        assert isinstance(result.content, str)
        assert isinstance(result.metadata, dict)
