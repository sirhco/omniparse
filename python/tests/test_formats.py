"""
Format-specific integration tests for omniparse Python bindings.

Tests extraction for each supported format category and verifies
format-specific metadata extraction.
"""

import pytest
import omniparse


class TestTextFormats:
    """Tests for text-based format extraction."""
    
    def test_plain_text_extraction(self):
        """Test plain text file extraction."""
        result = omniparse.extract_from_path("test_data/text/sample.txt")
        
        assert result.mime_type == "text/plain"
        assert isinstance(result.content, str)
        assert len(result.content) > 0
        assert result.detection_confidence > 0.5
    
    def test_json_extraction(self):
        """Test JSON file extraction and metadata."""
        result = omniparse.extract_from_path("test_data/text/sample.json")
        
        assert result.mime_type == "application/json"
        assert isinstance(result.content, str)
        assert result.detection_confidence > 0.5
        
        # JSON files should have content
        assert len(result.content) > 0
    
    def test_csv_extraction(self):
        """Test CSV file extraction and metadata."""
        result = omniparse.extract_from_path("test_data/text/sample.csv")
        
        assert result.mime_type == "text/csv"
        assert isinstance(result.content, str)
        assert result.detection_confidence > 0.5
        
        # CSV should have row/column metadata
        metadata = result.metadata
        assert isinstance(metadata, dict)
    
    def test_xml_extraction(self):
        """Test XML file extraction."""
        result = omniparse.extract_from_path("test_data/text/sample.xml")
        
        assert result.mime_type in ["application/xml", "text/xml"]
        assert isinstance(result.content, str)
        assert result.detection_confidence > 0.5
        assert len(result.content) > 0
    
    def test_minimal_json(self):
        """Test minimal JSON file extraction."""
        result = omniparse.extract_from_path("test_data/text/minimal.json")
        
        assert result.mime_type == "application/json"
        assert isinstance(result.content, str)
    
    def test_minimal_csv(self):
        """Test minimal CSV file extraction."""
        result = omniparse.extract_from_path("test_data/text/minimal.csv")
        
        assert result.mime_type == "text/csv"
        assert isinstance(result.content, str)


class TestDocumentFormats:
    """Tests for document format extraction."""
    
    def test_pdf_extraction(self):
        """Test PDF document extraction and metadata."""
        result = omniparse.extract_from_path("test_data/document/sample.pdf")
        
        assert result.mime_type == "application/pdf"
        assert isinstance(result.content, str)
        assert result.detection_confidence > 0.5
        
        # PDF metadata may include title, author, page count, etc.
        metadata = result.metadata
        assert isinstance(metadata, dict)
    
    def test_docx_extraction(self):
        """Test DOCX document extraction and metadata."""
        result = omniparse.extract_from_path("test_data/document/sample.docx")
        
        assert result.mime_type == "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        assert isinstance(result.content, str)
        assert result.detection_confidence > 0.5
        
        # DOCX metadata may include title, author, created date, etc.
        metadata = result.metadata
        assert isinstance(metadata, dict)
    
    def test_odt_extraction(self):
        """Test ODT document extraction and metadata."""
        result = omniparse.extract_from_path("test_data/document/sample.odt")
        
        assert result.mime_type == "application/vnd.oasis.opendocument.text"
        assert isinstance(result.content, str)
        assert result.detection_confidence > 0.5
        
        # ODT metadata may include title, author, etc.
        metadata = result.metadata
        assert isinstance(metadata, dict)
    
    def test_empty_pdf(self):
        """Test empty PDF file extraction."""
        result = omniparse.extract_from_path("test_data/document/empty.pdf")
        
        assert result.mime_type == "application/pdf"
        assert result.detection_confidence > 0.5
        # Empty PDF may have empty or minimal content
        assert result.content is None or isinstance(result.content, str)


class TestImageFormats:
    """Tests for image format extraction."""
    
    def test_png_extraction(self):
        """Test PNG image extraction and metadata."""
        result = omniparse.extract_from_path("test_data/image/sample.png")
        
        assert result.mime_type == "image/png"
        assert result.detection_confidence > 0.5
        
        # Images may have binary content or None
        assert result.content is None or isinstance(result.content, (str, bytes))
        
        # Image metadata may include dimensions, color depth, etc.
        metadata = result.metadata
        assert isinstance(metadata, dict)
    
    def test_jpeg_extraction(self):
        """Test JPEG image extraction and metadata."""
        result = omniparse.extract_from_path("test_data/image/sample.jpg")
        
        assert result.mime_type == "image/jpeg"
        assert result.detection_confidence > 0.5
        
        # JPEG metadata may include EXIF data
        metadata = result.metadata
        assert isinstance(metadata, dict)
    
    def test_tiff_extraction(self):
        """Test TIFF image extraction and metadata."""
        result = omniparse.extract_from_path("test_data/image/sample.tiff")
        
        assert result.mime_type == "image/tiff"
        assert result.detection_confidence > 0.5
        
        metadata = result.metadata
        assert isinstance(metadata, dict)
    
    def test_empty_png(self):
        """Test empty PNG file extraction."""
        result = omniparse.extract_from_path("test_data/image/empty.png")
        
        assert result.mime_type == "image/png"
        assert result.detection_confidence > 0.5


class TestArchiveFormats:
    """Tests for archive format extraction."""
    
    def test_zip_extraction(self):
        """Test ZIP archive extraction and metadata."""
        result = omniparse.extract_from_path("test_data/archive/sample.zip")
        
        assert result.mime_type == "application/zip"
        assert result.detection_confidence > 0.5
        
        # Archive metadata may include file list, compression info
        metadata = result.metadata
        assert isinstance(metadata, dict)
    
    def test_tar_extraction(self):
        """Test TAR archive extraction and metadata."""
        result = omniparse.extract_from_path("test_data/archive/sample.tar")
        
        assert result.mime_type in ["application/x-tar", "application/tar"]
        assert result.detection_confidence > 0.5
        
        metadata = result.metadata
        assert isinstance(metadata, dict)
    
    def test_empty_zip(self):
        """Test empty ZIP archive extraction."""
        result = omniparse.extract_from_path("test_data/archive/empty.zip")
        
        assert result.mime_type == "application/zip"
        assert result.detection_confidence > 0.5


class TestMetadataExtraction:
    """Tests for format-specific metadata extraction."""
    
    def test_metadata_types(self):
        """Test that metadata values have correct Python types."""
        result = omniparse.extract_from_path("test_data/document/sample.pdf")
        
        metadata = result.metadata
        
        # Metadata values should be Python native types
        for key, value in metadata.items():
            assert isinstance(key, str)
            assert isinstance(value, (str, int, float, bool, list, type(None)))
    
    def test_metadata_list_values(self):
        """Test that metadata list values are properly converted."""
        result = omniparse.extract_from_path("test_data/text/sample.json")
        
        metadata = result.metadata
        
        # Check that any list values are properly converted
        for value in metadata.values():
            if isinstance(value, list):
                for item in value:
                    assert isinstance(item, (str, int, float, bool, type(None)))
    
    def test_empty_metadata(self):
        """Test files with minimal or no metadata."""
        result = omniparse.extract_from_path("test_data/text/empty.txt")
        
        # Even empty files should have a metadata dict
        assert isinstance(result.metadata, dict)


class TestFormatCategories:
    """Tests extraction across format categories."""
    
    @pytest.mark.parametrize("file_path,category", [
        ("test_data/text/sample.txt", "text"),
        ("test_data/text/sample.json", "text"),
        ("test_data/text/sample.csv", "text"),
        ("test_data/document/sample.pdf", "document"),
        ("test_data/document/sample.docx", "document"),
        ("test_data/image/sample.png", "image"),
        ("test_data/image/sample.jpg", "image"),
        ("test_data/archive/sample.zip", "archive"),
    ])
    def test_category_extraction(self, file_path, category):
        """Test that files from each category can be extracted."""
        result = omniparse.extract_from_path(file_path)
        
        assert isinstance(result, omniparse.ExtractionResult)
        assert isinstance(result.mime_type, str)
        assert len(result.mime_type) > 0
        assert result.detection_confidence > 0.0
        assert isinstance(result.metadata, dict)
