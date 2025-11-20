#!/usr/bin/env python3
"""
Basic Usage Example for Omniparse Python Bindings

This example demonstrates:
- Extracting content from various file formats
- Accessing content and metadata
- Proper error handling
"""

import omniparse
import sys
from pathlib import Path


def extract_and_display(file_path: str):
    """Extract content from a file and display the results."""
    print(f"\n{'='*60}")
    print(f"Processing: {file_path}")
    print('='*60)
    
    try:
        # Extract content from file
        result = omniparse.extract_from_path(file_path)
        
        # Display MIME type and confidence
        print(f"MIME Type: {result.mime_type}")
        print(f"Detection Confidence: {result.detection_confidence:.2%}")
        
        # Display content (truncated if too long)
        if result.content:
            content_str = str(result.content)
            if len(content_str) > 200:
                print(f"\nContent (first 200 chars):\n{content_str[:200]}...")
            else:
                print(f"\nContent:\n{content_str}")
        else:
            print("\nContent: (empty)")
        
        # Display metadata
        if result.metadata:
            print(f"\nMetadata ({len(result.metadata)} fields):")
            for key, value in result.metadata.items():
                # Truncate long values
                value_str = str(value)
                if len(value_str) > 100:
                    value_str = value_str[:100] + "..."
                print(f"  {key}: {value_str}")
        else:
            print("\nMetadata: (none)")
            
    except IOError as e:
        print(f"❌ File access error: {e}", file=sys.stderr)
    except ValueError as e:
        print(f"❌ Format or parsing error: {e}", file=sys.stderr)
    except RuntimeError as e:
        print(f"❌ Processing error: {e}", file=sys.stderr)
    except Exception as e:
        print(f"❌ Unexpected error: {e}", file=sys.stderr)


def main():
    """Main function demonstrating various file format extractions."""
    
    # Check if omniparse is available
    print("Omniparse Python Bindings - Basic Usage Example")
    print(f"Supported formats: {len(omniparse.supported_mime_types())} MIME types")
    
    # Example 1: PDF document
    pdf_path = "test_data/document/sample.pdf"
    if Path(pdf_path).exists():
        extract_and_display(pdf_path)
    else:
        print(f"\nSkipping {pdf_path} (not found)")
    
    # Example 2: JSON file
    json_path = "test_data/text/sample.json"
    if Path(json_path).exists():
        extract_and_display(json_path)
    else:
        print(f"\nSkipping {json_path} (not found)")
    
    # Example 3: CSV file
    csv_path = "test_data/text/sample.csv"
    if Path(csv_path).exists():
        extract_and_display(csv_path)
    else:
        print(f"\nSkipping {csv_path} (not found)")
    
    # Example 4: Plain text
    txt_path = "test_data/text/sample.txt"
    if Path(txt_path).exists():
        extract_and_display(txt_path)
    else:
        print(f"\nSkipping {txt_path} (not found)")
    
    # Example 5: DOCX document
    docx_path = "test_data/document/sample.docx"
    if Path(docx_path).exists():
        extract_and_display(docx_path)
    else:
        print(f"\nSkipping {docx_path} (not found)")
    
    # Example 6: Image file
    image_path = "test_data/image/sample.jpg"
    if Path(image_path).exists():
        extract_and_display(image_path)
    else:
        print(f"\nSkipping {image_path} (not found)")
    
    # Example 7: Error handling - non-existent file
    print(f"\n{'='*60}")
    print("Testing error handling with non-existent file")
    print('='*60)
    extract_and_display("nonexistent_file.pdf")
    
    # Example 8: Check format support
    print(f"\n{'='*60}")
    print("Checking format support")
    print('='*60)
    formats_to_check = [
        "application/pdf",
        "application/json",
        "text/csv",
        "text/plain",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "image/jpeg",
        "application/x-unknown-format"
    ]
    
    for mime_type in formats_to_check:
        supported = omniparse.is_mime_supported(mime_type)
        status = "✓ Supported" if supported else "✗ Not supported"
        print(f"{status}: {mime_type}")
    
    print(f"\n{'='*60}")
    print("Example complete!")
    print('='*60)


if __name__ == "__main__":
    main()
