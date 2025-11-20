"""
Error handling tests for omniparse Python bindings.

Tests error conditions including IOError for missing files, ValueError for
unsupported/corrupted formats, and error message preservation.
"""

import pytest
import omniparse


class TestIOErrors:
    """Tests for IO-related errors."""
    
    def test_nonexistent_file(self):
        """Test that nonexistent file raises IOError."""
        with pytest.raises(IOError) as exc_info:
            omniparse.extract_from_path("nonexistent_file.txt")
        
        # Error message should mention the file issue
        error_msg = str(exc_info.value)
        assert len(error_msg) > 0
    
    def test_nonexistent_path(self):
        """Test that invalid path raises IOError."""
        with pytest.raises(IOError):
            omniparse.extract_from_path("/invalid/path/to/file.pdf")
    
    def test_directory_instead_of_file(self):
        """Test that directory path raises IOError."""
        with pytest.raises(IOError):
            omniparse.extract_from_path("test_data")


class TestValueErrors:
    """Tests for ValueError exceptions."""
    
    def test_unsupported_format_from_path(self):
        """Test that unsupported format raises ValueError."""
        # Create a file with unsupported extension/content
        # Using a font file which should not be supported for content extraction
        with pytest.raises(ValueError) as exc_info:
            omniparse.extract_from_path("test_data/fonts/DejaVuSans.ttf")
        
        error_msg = str(exc_info.value)
        # Error should mention unsupported format or similar
        assert len(error_msg) > 0
    
    def test_unsupported_format_from_bytes(self):
        """Test that unsupported format in bytes raises ValueError."""
        # Random binary data that doesn't match any format
        invalid_data = b"\x00\x01\x02\x03\x04\x05\x06\x07"
        
        with pytest.raises(ValueError):
            omniparse.extract_from_bytes(invalid_data)
    
    def test_corrupted_json(self):
        """Test that corrupted JSON file raises ValueError."""
        with pytest.raises(ValueError) as exc_info:
            omniparse.extract_from_path("test_data/text/invalid.json")
        
        error_msg = str(exc_info.value)
        assert len(error_msg) > 0
    
    def test_corrupted_bytes(self):
        """Test that corrupted file data raises ValueError."""
        # Take valid PDF header but truncate it
        with open("test_data/document/sample.pdf", "rb") as f:
            data = f.read(100)  # Only first 100 bytes - corrupted
        
        with pytest.raises(ValueError):
            omniparse.extract_from_bytes(data, mime_hint="application/pdf")
    
    def test_empty_bytes(self):
        """Test that empty bytes raises ValueError."""
        with pytest.raises(ValueError):
            omniparse.extract_from_bytes(b"")


class TestErrorMessages:
    """Tests for error message preservation and clarity."""
    
    def test_io_error_message_contains_context(self):
        """Test that IOError messages contain useful context."""
        with pytest.raises(IOError) as exc_info:
            omniparse.extract_from_path("missing_file.pdf")
        
        error_msg = str(exc_info.value)
        # Should contain some context about the error
        assert len(error_msg) > 10
        assert isinstance(error_msg, str)
    
    def test_unsupported_format_message(self):
        """Test that unsupported format error has clear message."""
        try:
            omniparse.extract_from_path("test_data/fonts/DejaVuSans.ttf")
            pytest.fail("Expected ValueError for unsupported format")
        except ValueError as e:
            error_msg = str(e)
            # Message should indicate unsupported format
            assert len(error_msg) > 0
            assert isinstance(error_msg, str)
    
    def test_corrupted_file_message(self):
        """Test that corrupted file error has descriptive message."""
        try:
            omniparse.extract_from_path("test_data/text/invalid.json")
            pytest.fail("Expected ValueError for corrupted file")
        except ValueError as e:
            error_msg = str(e)
            # Message should provide some detail about the issue
            assert len(error_msg) > 0
            assert isinstance(error_msg, str)
    
    def test_error_message_types(self):
        """Test that error messages are strings."""
        errors_to_test = [
            (IOError, lambda: omniparse.extract_from_path("nonexistent.txt")),
            (ValueError, lambda: omniparse.extract_from_bytes(b"\x00\x01\x02")),
        ]
        
        for expected_error, func in errors_to_test:
            with pytest.raises(expected_error) as exc_info:
                func()
            
            assert isinstance(str(exc_info.value), str)
            assert len(str(exc_info.value)) > 0


class TestErrorRecovery:
    """Tests for error recovery and handling."""
    
    def test_error_does_not_crash_subsequent_calls(self):
        """Test that errors don't affect subsequent valid calls."""
        # First call should fail
        with pytest.raises(IOError):
            omniparse.extract_from_path("nonexistent.txt")
        
        # Second call should succeed
        result = omniparse.extract_from_path("test_data/text/sample.txt")
        assert result.mime_type == "text/plain"
    
    def test_multiple_errors_in_sequence(self):
        """Test handling multiple errors in sequence."""
        # Multiple failed calls should all raise appropriate errors
        with pytest.raises(IOError):
            omniparse.extract_from_path("missing1.txt")
        
        with pytest.raises(IOError):
            omniparse.extract_from_path("missing2.txt")
        
        with pytest.raises(ValueError):
            omniparse.extract_from_bytes(b"\x00\x01")
        
        # Valid call should still work
        result = omniparse.extract_from_path("test_data/text/sample.json")
        assert result.mime_type == "application/json"


class TestExceptionTypes:
    """Tests for correct exception type mapping."""
    
    def test_io_error_type(self):
        """Test that file access issues raise IOError."""
        with pytest.raises(IOError):
            omniparse.extract_from_path("nonexistent.pdf")
        
        # Should not raise other exception types
        try:
            omniparse.extract_from_path("nonexistent.pdf")
        except IOError:
            pass  # Expected
        except ValueError:
            pytest.fail("Should raise IOError, not ValueError")
        except RuntimeError:
            pytest.fail("Should raise IOError, not RuntimeError")
    
    def test_value_error_type(self):
        """Test that format issues raise ValueError."""
        with pytest.raises(ValueError):
            omniparse.extract_from_bytes(b"\x00\x01\x02")
        
        # Should not raise other exception types
        try:
            omniparse.extract_from_bytes(b"\x00\x01\x02")
        except ValueError:
            pass  # Expected
        except IOError:
            pytest.fail("Should raise ValueError, not IOError")
        except RuntimeError:
            pytest.fail("Should raise ValueError, not RuntimeError")
