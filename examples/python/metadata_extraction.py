#!/usr/bin/env python3
"""
Metadata Extraction Example for Omniparse Python Bindings

This example demonstrates:
- Metadata-only extraction patterns
- Accessing different metadata types (text, numbers, dates, lists)
- Organizing and displaying metadata
- Metadata-based file analysis and categorization
"""

import omniparse
import sys
from pathlib import Path
from typing import Dict, Any, List
from collections import defaultdict


def extract_metadata(file_path: str) -> Dict[str, Any]:
    """
    Extract metadata from a file without processing content.
    
    Returns:
        Dictionary containing metadata or empty dict on error
    """
    try:
        result = omniparse.extract_from_path(file_path)
        return result.metadata if result.metadata else {}
    except Exception as e:
        print(f"Error extracting metadata from {file_path}: {e}", file=sys.stderr)
        return {}


def display_metadata(file_path: str, metadata: Dict[str, Any]):
    """Display metadata in a formatted way."""
    print(f"\n{'='*60}")
    print(f"File: {Path(file_path).name}")
    print('='*60)
    
    if not metadata:
        print("No metadata available")
        return
    
    # Group metadata by type
    text_fields = {}
    numeric_fields = {}
    date_fields = {}
    list_fields = {}
    other_fields = {}
    
    for key, value in metadata.items():
        if isinstance(value, str):
            # Check if it looks like a date
            if any(date_indicator in key.lower() for date_indicator in ['date', 'time', 'created', 'modified']):
                date_fields[key] = value
            else:
                text_fields[key] = value
        elif isinstance(value, (int, float)):
            numeric_fields[key] = value
        elif isinstance(value, list):
            list_fields[key] = value
        else:
            other_fields[key] = value
    
    # Display grouped metadata
    if text_fields:
        print("\n📝 Text Fields:")
        for key, value in text_fields.items():
            # Truncate long values
            value_str = str(value)
            if len(value_str) > 80:
                value_str = value_str[:80] + "..."
            print(f"  {key}: {value_str}")
    
    if numeric_fields:
        print("\n🔢 Numeric Fields:")
        for key, value in numeric_fields.items():
            print(f"  {key}: {value}")
    
    if date_fields:
        print("\n📅 Date/Time Fields:")
        for key, value in date_fields.items():
            print(f"  {key}: {value}")
    
    if list_fields:
        print("\n📋 List Fields:")
        for key, value in list_fields.items():
            print(f"  {key}: {value}")
    
    if other_fields:
        print("\n🔧 Other Fields:")
        for key, value in other_fields.items():
            print(f"  {key}: {value}")


def analyze_metadata_coverage(files: List[str]) -> Dict[str, int]:
    """
    Analyze which metadata fields are present across multiple files.
    
    Returns:
        Dictionary mapping field names to occurrence counts
    """
    field_counts = defaultdict(int)
    
    for file_path in files:
        metadata = extract_metadata(file_path)
        for key in metadata.keys():
            field_counts[key] += 1
    
    return dict(field_counts)


def categorize_by_metadata(files: List[str]) -> Dict[str, List[str]]:
    """
    Categorize files based on their metadata characteristics.
    
    Returns:
        Dictionary mapping categories to file lists
    """
    categories = {
        'has_author': [],
        'has_title': [],
        'has_dates': [],
        'has_page_count': [],
        'rich_metadata': [],  # 5+ fields
        'minimal_metadata': [],  # 1-2 fields
        'no_metadata': [],
    }
    
    for file_path in files:
        metadata = extract_metadata(file_path)
        
        if not metadata:
            categories['no_metadata'].append(file_path)
            continue
        
        # Check for specific fields
        if any('author' in key.lower() for key in metadata.keys()):
            categories['has_author'].append(file_path)
        
        if any('title' in key.lower() for key in metadata.keys()):
            categories['has_title'].append(file_path)
        
        if any(date_key in key.lower() for key in metadata.keys() 
               for date_key in ['date', 'time', 'created', 'modified']):
            categories['has_dates'].append(file_path)
        
        if any('page' in key.lower() for key in metadata.keys()):
            categories['has_page_count'].append(file_path)
        
        # Categorize by metadata richness
        field_count = len(metadata)
        if field_count >= 5:
            categories['rich_metadata'].append(file_path)
        elif field_count <= 2:
            categories['minimal_metadata'].append(file_path)
    
    return categories


def extract_specific_fields(files: List[str], fields: List[str]) -> Dict[str, Dict[str, Any]]:
    """
    Extract specific metadata fields from multiple files.
    
    Args:
        files: List of file paths
        fields: List of field names to extract (case-insensitive partial match)
    
    Returns:
        Dictionary mapping file paths to extracted field values
    """
    results = {}
    
    for file_path in files:
        metadata = extract_metadata(file_path)
        extracted = {}
        
        for field in fields:
            # Find matching keys (case-insensitive partial match)
            for key, value in metadata.items():
                if field.lower() in key.lower():
                    extracted[key] = value
        
        if extracted:
            results[file_path] = extracted
    
    return results


def main():
    """Main function demonstrating metadata extraction patterns."""
    
    print("Omniparse Python Bindings - Metadata Extraction Example\n")
    
    # Collect test files
    test_files = []
    test_dirs = ["test_data/document", "test_data/text", "test_data/image"]
    
    for test_dir in test_dirs:
        dir_path = Path(test_dir)
        if dir_path.exists():
            for file_path in dir_path.glob("*"):
                if file_path.is_file() and not file_path.name.startswith('.'):
                    test_files.append(str(file_path))
    
    if not test_files:
        print("No test files found. Please ensure test_data directory exists.")
        sys.exit(1)
    
    print(f"Found {len(test_files)} test files\n")
    
    # Example 1: Extract and display metadata for each file
    print(f"{'='*60}")
    print("Example 1: Individual File Metadata")
    print('='*60)
    
    for file_path in test_files[:5]:  # Show first 5 files
        metadata = extract_metadata(file_path)
        display_metadata(file_path, metadata)
    
    # Example 2: Analyze metadata coverage
    print(f"\n{'='*60}")
    print("Example 2: Metadata Field Coverage Analysis")
    print('='*60)
    
    field_counts = analyze_metadata_coverage(test_files)
    
    if field_counts:
        print(f"\nMetadata fields found across {len(test_files)} files:")
        print(f"{'Field Name':<30} {'Occurrences':<15} {'Coverage':<10}")
        print('-' * 60)
        
        # Sort by occurrence count
        for field, count in sorted(field_counts.items(), key=lambda x: x[1], reverse=True):
            coverage = (count / len(test_files)) * 100
            print(f"{field:<30} {count:<15} {coverage:>6.1f}%")
    else:
        print("No metadata fields found")
    
    # Example 3: Categorize files by metadata
    print(f"\n{'='*60}")
    print("Example 3: File Categorization by Metadata")
    print('='*60)
    
    categories = categorize_by_metadata(test_files)
    
    for category, files in categories.items():
        if files:
            print(f"\n{category.replace('_', ' ').title()}: {len(files)} files")
            for file_path in files[:3]:  # Show first 3 in each category
                print(f"  - {Path(file_path).name}")
            if len(files) > 3:
                print(f"  ... and {len(files) - 3} more")
    
    # Example 4: Extract specific fields
    print(f"\n{'='*60}")
    print("Example 4: Extract Specific Metadata Fields")
    print('='*60)
    
    # Look for author and title information
    fields_of_interest = ['author', 'title', 'creator', 'subject']
    specific_metadata = extract_specific_fields(test_files, fields_of_interest)
    
    if specific_metadata:
        print(f"\nFiles with author/title information:")
        for file_path, metadata in specific_metadata.items():
            print(f"\n  {Path(file_path).name}:")
            for key, value in metadata.items():
                value_str = str(value)
                if len(value_str) > 60:
                    value_str = value_str[:60] + "..."
                print(f"    {key}: {value_str}")
    else:
        print("\nNo files found with author/title information")
    
    # Example 5: Metadata-only extraction pattern
    print(f"\n{'='*60}")
    print("Example 5: Metadata-Only Extraction Pattern")
    print('='*60)
    
    print("\nDemonstrating efficient metadata-only extraction:")
    print("(Useful when you only need file properties, not content)\n")
    
    for file_path in test_files[:3]:
        try:
            result = omniparse.extract_from_path(file_path)
            
            print(f"File: {Path(file_path).name}")
            print(f"  MIME Type: {result.mime_type}")
            print(f"  Detection Confidence: {result.detection_confidence:.2%}")
            print(f"  Metadata Fields: {len(result.metadata) if result.metadata else 0}")
            
            # Access metadata without processing content
            if result.metadata:
                # Example: Get file size if available
                size_fields = [k for k in result.metadata.keys() if 'size' in k.lower()]
                if size_fields:
                    print(f"  Size Info: {result.metadata[size_fields[0]]}")
                
                # Example: Get creation date if available
                date_fields = [k for k in result.metadata.keys() if 'created' in k.lower() or 'date' in k.lower()]
                if date_fields:
                    print(f"  Date Info: {result.metadata[date_fields[0]]}")
            
            print()
            
        except Exception as e:
            print(f"Error: {e}\n")
    
    # Example 6: Metadata comparison
    print(f"{'='*60}")
    print("Example 6: Compare Metadata Across File Types")
    print('='*60)
    
    # Group files by extension
    by_extension = defaultdict(list)
    for file_path in test_files:
        ext = Path(file_path).suffix.lower()
        by_extension[ext].append(file_path)
    
    print("\nMetadata richness by file type:")
    for ext, files in sorted(by_extension.items()):
        if not ext:
            continue
        
        total_fields = 0
        file_count = 0
        
        for file_path in files:
            metadata = extract_metadata(file_path)
            if metadata:
                total_fields += len(metadata)
                file_count += 1
        
        avg_fields = total_fields / file_count if file_count > 0 else 0
        print(f"  {ext:10s}: {avg_fields:5.1f} avg metadata fields ({file_count} files)")
    
    print(f"\n{'='*60}")
    print("Metadata extraction examples complete!")
    print('='*60)


if __name__ == "__main__":
    main()
