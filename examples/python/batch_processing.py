#!/usr/bin/env python3
"""
Batch Processing Example for Omniparse Python Bindings

This example demonstrates:
- Concurrent processing using ThreadPoolExecutor
- Processing multiple files efficiently
- Performance comparison between sequential and parallel processing
- Progress tracking and error handling in batch operations
"""

import omniparse
import time
import sys
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import List, Tuple, Optional


def process_file(file_path: str) -> Tuple[str, Optional[dict], Optional[str]]:
    """
    Process a single file and return results.
    
    Returns:
        Tuple of (file_path, result_summary, error_message)
    """
    try:
        result = omniparse.extract_from_path(file_path)
        
        summary = {
            'mime_type': result.mime_type,
            'confidence': result.detection_confidence,
            'content_length': len(result.content) if result.content else 0,
            'metadata_fields': len(result.metadata) if result.metadata else 0,
        }
        
        return (file_path, summary, None)
        
    except Exception as e:
        return (file_path, None, str(e))


def process_sequential(files: List[str]) -> List[Tuple[str, Optional[dict], Optional[str]]]:
    """Process files sequentially."""
    results = []
    for file_path in files:
        results.append(process_file(file_path))
    return results


def process_parallel(files: List[str], max_workers: int = 4) -> List[Tuple[str, Optional[dict], Optional[str]]]:
    """Process files in parallel using ThreadPoolExecutor."""
    results = []
    
    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        # Submit all tasks
        future_to_file = {executor.submit(process_file, file_path): file_path 
                         for file_path in files}
        
        # Collect results as they complete
        for future in as_completed(future_to_file):
            results.append(future.result())
    
    return results


def process_parallel_with_progress(files: List[str], max_workers: int = 4) -> List[Tuple[str, Optional[dict], Optional[str]]]:
    """Process files in parallel with progress tracking."""
    results = []
    total = len(files)
    completed = 0
    
    print(f"Processing {total} files with {max_workers} workers...")
    
    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        # Submit all tasks
        future_to_file = {executor.submit(process_file, file_path): file_path 
                         for file_path in files}
        
        # Collect results as they complete
        for future in as_completed(future_to_file):
            result = future.result()
            results.append(result)
            completed += 1
            
            # Display progress
            file_path, summary, error = result
            status = "✓" if summary else "✗"
            print(f"[{completed}/{total}] {status} {Path(file_path).name}")
    
    return results


def display_results(results: List[Tuple[str, Optional[dict], Optional[str]]], duration: float):
    """Display processing results summary."""
    successful = sum(1 for _, summary, _ in results if summary is not None)
    failed = len(results) - successful
    
    print(f"\n{'='*60}")
    print(f"Processing Summary")
    print('='*60)
    print(f"Total files: {len(results)}")
    print(f"Successful: {successful}")
    print(f"Failed: {failed}")
    print(f"Duration: {duration:.2f} seconds")
    print(f"Throughput: {len(results)/duration:.2f} files/second")
    
    if failed > 0:
        print(f"\nFailed files:")
        for file_path, _, error in results:
            if error:
                print(f"  ✗ {Path(file_path).name}: {error}")
    
    # Display some successful results
    print(f"\nSuccessful extractions (sample):")
    count = 0
    for file_path, summary, _ in results:
        if summary and count < 5:
            print(f"  ✓ {Path(file_path).name}")
            print(f"    MIME: {summary['mime_type']}")
            print(f"    Confidence: {summary['confidence']:.2%}")
            print(f"    Content length: {summary['content_length']} chars")
            print(f"    Metadata fields: {summary['metadata_fields']}")
            count += 1


def collect_test_files() -> List[str]:
    """Collect all available test files."""
    test_dirs = [
        "test_data/document",
        "test_data/text",
        "test_data/image",
        "test_data/archive",
    ]
    
    files = []
    for test_dir in test_dirs:
        dir_path = Path(test_dir)
        if dir_path.exists():
            for file_path in dir_path.glob("*"):
                if file_path.is_file() and not file_path.name.startswith('.'):
                    files.append(str(file_path))
    
    return files


def main():
    """Main function demonstrating batch processing."""
    
    print("Omniparse Python Bindings - Batch Processing Example\n")
    
    # Collect test files
    files = collect_test_files()
    
    if not files:
        print("No test files found. Please ensure test_data directory exists.")
        sys.exit(1)
    
    print(f"Found {len(files)} test files\n")
    
    # Example 1: Sequential processing
    print(f"{'='*60}")
    print("Example 1: Sequential Processing")
    print('='*60)
    
    start_time = time.time()
    results_seq = process_sequential(files)
    duration_seq = time.time() - start_time
    
    display_results(results_seq, duration_seq)
    
    # Example 2: Parallel processing (4 workers)
    print(f"\n{'='*60}")
    print("Example 2: Parallel Processing (4 workers)")
    print('='*60)
    
    start_time = time.time()
    results_par = process_parallel(files, max_workers=4)
    duration_par = time.time() - start_time
    
    display_results(results_par, duration_par)
    
    # Calculate speedup
    speedup = duration_seq / duration_par if duration_par > 0 else 0
    print(f"\nSpeedup: {speedup:.2f}x faster than sequential")
    
    # Example 3: Parallel processing with progress tracking
    print(f"\n{'='*60}")
    print("Example 3: Parallel Processing with Progress Tracking")
    print('='*60)
    
    start_time = time.time()
    results_progress = process_parallel_with_progress(files, max_workers=4)
    duration_progress = time.time() - start_time
    
    print(f"\nCompleted in {duration_progress:.2f} seconds")
    
    # Example 4: Different worker counts
    print(f"\n{'='*60}")
    print("Example 4: Performance Comparison (Different Worker Counts)")
    print('='*60)
    
    worker_counts = [1, 2, 4, 8]
    
    for workers in worker_counts:
        start_time = time.time()
        results = process_parallel(files, max_workers=workers)
        duration = time.time() - start_time
        throughput = len(results) / duration
        
        print(f"Workers: {workers:2d} | Duration: {duration:6.2f}s | Throughput: {throughput:6.2f} files/s")
    
    # Example 5: Processing specific file types
    print(f"\n{'='*60}")
    print("Example 5: Processing Specific File Types")
    print('='*60)
    
    # Filter for document files
    doc_files = [f for f in files if 'document' in f]
    
    if doc_files:
        print(f"\nProcessing {len(doc_files)} document files...")
        start_time = time.time()
        results_docs = process_parallel(doc_files, max_workers=4)
        duration_docs = time.time() - start_time
        display_results(results_docs, duration_docs)
    
    print(f"\n{'='*60}")
    print("Batch processing examples complete!")
    print('='*60)


if __name__ == "__main__":
    main()
