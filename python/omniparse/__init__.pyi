"""
Type stubs for omniparse Python bindings.

This file provides type hints for IDE support and static type checking.
"""

from typing import Optional, Dict, Any, List, Union

__version__: str

class ExtractionResult:
    """
    Result of a file extraction operation.
    
    Attributes:
        mime_type: The detected MIME type of the file (e.g., "application/pdf")
        content: The extracted text content (str), binary data (bytes), or None
        metadata: Dictionary containing file metadata (title, author, dates, etc.)
        detection_confidence: Confidence score (0.0-1.0) for the MIME type detection
    
    Example:
        >>> result = extract_from_path("document.pdf")
        >>> print(result.mime_type)
        'application/pdf'
        >>> print(result.detection_confidence)
        0.95
    """
    
    @property
    def mime_type(self) -> str:
        """The detected MIME type of the file."""
        ...
    
    @property
    def content(self) -> Optional[Union[str, bytes]]:
        """
        The extracted content from the file.
        
        Returns:
            str for text-based formats (text/plain, application/json, etc.)
            bytes for binary formats (images, etc.)
            None if no content was extracted
        """
        ...
    
    @property
    def metadata(self) -> Dict[str, Any]:
        """
        Metadata extracted from the file.
        
        Common metadata keys include:
        - title: Document title
        - author: Document author
        - created: Creation date (ISO 8601 string)
        - modified: Last modification date (ISO 8601 string)
        - page_count: Number of pages (for documents)
        - word_count: Number of words (for text documents)
        
        Returns:
            Dictionary with string keys and values of various types
            (str, int, float, bool, list)
        """
        ...
    
    @property
    def detection_confidence(self) -> float:
        """
        Confidence score for the MIME type detection.
        
        Returns:
            Float between 0.0 and 1.0, where 1.0 indicates highest confidence
        """
        ...
    
    def __repr__(self) -> str:
        """Return a string representation of the extraction result."""
        ...

def extract_from_path(path: str) -> ExtractionResult:
    """
    Extract content and metadata from a file at the given path.
    
    This function automatically detects the file format and uses the appropriate
    parser to extract text content and metadata.
    
    Args:
        path: Path to the file to extract from
    
    Returns:
        ExtractionResult containing the extracted data
    
    Raises:
        IOError: If the file cannot be read or does not exist
        ValueError: If the file format is unsupported or the file is corrupted
        RuntimeError: If detection fails or extraction is only partially successful
    
    Example:
        >>> result = extract_from_path("document.pdf")
        >>> print(f"Type: {result.mime_type}")
        >>> print(f"Content: {result.content}")
        >>> print(f"Author: {result.metadata.get('author', 'Unknown')}")
    """
    ...

def extract_from_bytes(
    data: bytes,
    mime_hint: Optional[str] = None
) -> ExtractionResult:
    """
    Extract content and metadata from raw bytes.
    
    This function is useful when you have file data in memory rather than
    on disk. You can optionally provide a MIME type hint to improve detection.
    
    Args:
        data: Raw bytes of the file content
        mime_hint: Optional MIME type hint to assist detection
                   (e.g., "application/pdf", "text/plain")
    
    Returns:
        ExtractionResult containing the extracted data
    
    Raises:
        ValueError: If the data format is unsupported or corrupted
        RuntimeError: If detection fails or extraction is only partially successful
    
    Example:
        >>> with open("document.pdf", "rb") as f:
        ...     data = f.read()
        >>> result = extract_from_bytes(data, mime_hint="application/pdf")
        >>> print(result.mime_type)
    """
    ...

def supported_mime_types() -> List[str]:
    """
    Get a list of all supported MIME types.
    
    Returns:
        List of MIME type strings that can be parsed by omniparse
    
    Example:
        >>> formats = supported_mime_types()
        >>> print(f"Supports {len(formats)} formats")
        >>> print("PDF supported:", "application/pdf" in formats)
    """
    ...

def is_mime_supported(mime_type: str) -> bool:
    """
    Check if a specific MIME type is supported.
    
    Args:
        mime_type: MIME type string to check (e.g., "application/pdf")
    
    Returns:
        True if the MIME type is supported, False otherwise
    
    Example:
        >>> if is_mime_supported("application/pdf"):
        ...     result = extract_from_path("document.pdf")
        >>> else:
        ...     print("PDF format not supported")
    """
    ...

__all__: List[str]
