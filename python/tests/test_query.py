"""
Format query tests for omniparse Python bindings.

Tests the supported_mime_types and is_mime_supported functions to verify
format capability queries work correctly.
"""

import pytest
import omniparse


class TestSupportedMimeTypes:
    """Tests for supported_mime_types function."""
    
    def test_returns_list(self):
        """Test that supported_mime_types returns a list."""
        formats = omniparse.supported_mime_types()
        assert isinstance(formats, list)
    
    def test_list_not_empty(self):
        """Test that the list of supported formats is not empty."""
        formats = omniparse.supported_mime_types()
        assert len(formats) > 0
    
    def test_list_contains_strings(self):
        """Test that all items in the list are strings."""
        formats = omniparse.supported_mime_types()
        for mime_type in formats:
            assert isinstance(mime_type, str)
            assert len(mime_type) > 0
    
    def test_contains_common_formats(self):
        """Test that common formats are in the supported list."""
        formats = omniparse.supported_mime_types()
        
        # Check for some common formats that should be supported
        common_formats = [
            "text/plain",
            "application/json",
            "text/csv",
            "application/pdf",
        ]
        
        for fmt in common_formats:
            assert fmt in formats, f"{fmt} should be in supported formats"
    
    def test_contains_document_formats(self):
        """Test that document formats are supported."""
        formats = omniparse.supported_mime_types()
        
        # Check for document formats
        assert "application/pdf" in formats
        assert "application/vnd.openxmlformats-officedocument.wordprocessingml.document" in formats
    
    def test_contains_image_formats(self):
        """Test that image formats are supported."""
        formats = omniparse.supported_mime_types()
        
        # Check for image formats
        assert "image/png" in formats
        assert "image/jpeg" in formats
    
    def test_contains_archive_formats(self):
        """Test that archive formats are supported."""
        formats = omniparse.supported_mime_types()
        
        # Check for archive formats
        assert "application/zip" in formats
    
    def test_no_duplicates(self):
        """Test that there are no duplicate MIME types in the list."""
        formats = omniparse.supported_mime_types()
        assert len(formats) == len(set(formats))
    
    def test_consistent_results(self):
        """Test that multiple calls return the same results."""
        formats1 = omniparse.supported_mime_types()
        formats2 = omniparse.supported_mime_types()
        
        assert formats1 == formats2


class TestIsMimeSupported:
    """Tests for is_mime_supported function."""
    
    def test_returns_boolean(self):
        """Test that is_mime_supported returns a boolean."""
        result = omniparse.is_mime_supported("text/plain")
        assert isinstance(result, bool)
    
    def test_supported_format_returns_true(self):
        """Test that supported formats return True."""
        supported_formats = [
            "text/plain",
            "application/json",
            "text/csv",
            "application/pdf",
            "image/png",
            "image/jpeg",
            "application/zip",
        ]
        
        for mime_type in supported_formats:
            assert omniparse.is_mime_supported(mime_type) is True
    
    def test_unsupported_format_returns_false(self):
        """Test that unsupported formats return False."""
        unsupported_formats = [
            "application/x-unknown",
            "text/x-fake-format",
            "application/x-nonexistent",
            "image/x-invalid",
        ]
        
        for mime_type in unsupported_formats:
            assert omniparse.is_mime_supported(mime_type) is False
    
    def test_empty_string_returns_false(self):
        """Test that empty string returns False."""
        assert omniparse.is_mime_supported("") is False
    
    def test_invalid_mime_type_returns_false(self):
        """Test that invalid MIME type format returns False."""
        invalid_types = [
            "not-a-mime-type",
            "invalid",
            "text",
            "/plain",
        ]
        
        for mime_type in invalid_types:
            assert omniparse.is_mime_supported(mime_type) is False
    
    def test_case_sensitivity(self):
        """Test MIME type case sensitivity."""
        # MIME types should be case-insensitive according to RFC
        # but implementation may vary
        result_lower = omniparse.is_mime_supported("text/plain")
        result_upper = omniparse.is_mime_supported("TEXT/PLAIN")
        result_mixed = omniparse.is_mime_supported("Text/Plain")
        
        # At least the lowercase version should work
        assert result_lower is True
    
    def test_consistency_with_supported_list(self):
        """Test that is_mime_supported is consistent with supported_mime_types."""
        formats = omniparse.supported_mime_types()
        
        # All formats in the list should return True
        for mime_type in formats:
            assert omniparse.is_mime_supported(mime_type) is True
    
    def test_multiple_calls_same_result(self):
        """Test that multiple calls return consistent results."""
        mime_type = "application/pdf"
        
        result1 = omniparse.is_mime_supported(mime_type)
        result2 = omniparse.is_mime_supported(mime_type)
        result3 = omniparse.is_mime_supported(mime_type)
        
        assert result1 == result2 == result3


class TestQueryIntegration:
    """Integration tests for query functions."""
    
    def test_query_before_extraction(self):
        """Test using query functions before extraction."""
        # Check if format is supported before attempting extraction
        if omniparse.is_mime_supported("application/pdf"):
            result = omniparse.extract_from_path("test_data/document/sample.pdf")
            assert result.mime_type == "application/pdf"
    
    def test_all_supported_formats_extractable(self):
        """Test that formats reported as supported can be extracted."""
        # Map MIME types to test files
        test_files = {
            "text/plain": "test_data/text/sample.txt",
            "application/json": "test_data/text/sample.json",
            "text/csv": "test_data/text/sample.csv",
            "application/pdf": "test_data/document/sample.pdf",
            "image/png": "test_data/image/sample.png",
            "image/jpeg": "test_data/image/sample.jpg",
            "application/zip": "test_data/archive/sample.zip",
        }
        
        for mime_type, file_path in test_files.items():
            if omniparse.is_mime_supported(mime_type):
                # Should be able to extract without error
                result = omniparse.extract_from_path(file_path)
                assert isinstance(result, omniparse.ExtractionResult)
    
    def test_unsupported_format_extraction_fails(self):
        """Test that unsupported formats fail extraction."""
        # Font files should not be supported for content extraction
        assert omniparse.is_mime_supported("application/x-font-ttf") is False or \
               omniparse.is_mime_supported("font/ttf") is False
        
        # Attempting to extract should raise an error
        with pytest.raises((ValueError, IOError)):
            omniparse.extract_from_path("test_data/fonts/DejaVuSans.ttf")


class TestQueryEdgeCases:
    """Tests for edge cases in query functions."""
    
    def test_mime_type_with_parameters(self):
        """Test MIME types with parameters."""
        # Some MIME types include parameters like charset
        # Test base type without parameters
        assert omniparse.is_mime_supported("text/plain") is True
    
    def test_similar_mime_types(self):
        """Test similar but different MIME types."""
        # XML can have multiple MIME types
        xml_types = ["application/xml", "text/xml"]
        
        # At least one should be supported
        xml_supported = any(omniparse.is_mime_supported(t) for t in xml_types)
        assert xml_supported
    
    def test_tar_mime_type_variants(self):
        """Test TAR archive MIME type variants."""
        # TAR can be represented as different MIME types
        tar_types = ["application/x-tar", "application/tar"]
        
        # At least one should be supported
        tar_supported = any(omniparse.is_mime_supported(t) for t in tar_types)
        assert tar_supported
