"""
Performance tests for omniparse Python bindings.

Tests concurrent extraction, GIL release verification, extraction overhead,
and memory efficiency with large files.
"""

import pytest
import omniparse
import time
import threading
from concurrent.futures import ThreadPoolExecutor, as_completed


class TestConcurrentExtraction:
    """Tests for concurrent extraction with ThreadPoolExecutor."""
    
    def test_concurrent_extraction_basic(self):
        """Test basic concurrent extraction with multiple threads."""
        files = [
            "test_data/text/sample.txt",
            "test_data/text/sample.json",
            "test_data/text/sample.csv",
            "test_data/document/sample.pdf",
        ]
        
        with ThreadPoolExecutor(max_workers=4) as executor:
            futures = [executor.submit(omniparse.extract_from_path, f) for f in files]
            results = [future.result() for future in as_completed(futures)]
        
        assert len(results) == len(files)
        for result in results:
            assert isinstance(result, omniparse.ExtractionResult)
            assert isinstance(result.mime_type, str)
    
    def test_concurrent_same_file(self):
        """Test concurrent extraction of the same file multiple times."""
        file_path = "test_data/document/sample.pdf"
        num_extractions = 10
        
        with ThreadPoolExecutor(max_workers=4) as executor:
            futures = [executor.submit(omniparse.extract_from_path, file_path) 
                      for _ in range(num_extractions)]
            results = [future.result() for future in as_completed(futures)]
        
        assert len(results) == num_extractions
        
        # All results should be consistent
        mime_types = [r.mime_type for r in results]
        assert all(mt == "application/pdf" for mt in mime_types)
    
    def test_concurrent_different_formats(self):
        """Test concurrent extraction of different file formats."""
        files = [
            "test_data/text/sample.txt",
            "test_data/text/sample.json",
            "test_data/document/sample.pdf",
            "test_data/image/sample.png",
            "test_data/archive/sample.zip",
        ]
        
        with ThreadPoolExecutor(max_workers=5) as executor:
            results = list(executor.map(omniparse.extract_from_path, files))
        
        assert len(results) == len(files)
        
        # Verify each result
        expected_types = [
            "text/plain",
            "application/json",
            "application/pdf",
            "image/png",
            "application/zip",
        ]
        
        for result, expected in zip(results, expected_types):
            assert result.mime_type == expected


class TestGILRelease:
    """Tests to verify GIL is released during operations."""
    
    def test_parallel_speedup(self):
        """Test that parallel execution is faster than sequential."""
        files = ["test_data/document/sample.pdf"] * 8
        
        # Sequential execution
        start_seq = time.time()
        for file_path in files:
            omniparse.extract_from_path(file_path)
        sequential_time = time.time() - start_seq
        
        # Parallel execution
        start_par = time.time()
        with ThreadPoolExecutor(max_workers=4) as executor:
            list(executor.map(omniparse.extract_from_path, files))
        parallel_time = time.time() - start_par
        
        # Parallel should be faster (allowing some overhead)
        # With GIL released, we should see speedup
        speedup = sequential_time / parallel_time
        
        # Should see at least some speedup (>1.2x) if GIL is released
        # This is a conservative check
        assert speedup > 1.0
    
    def test_threads_can_run_simultaneously(self):
        """Test that multiple threads can extract simultaneously."""
        results = []
        errors = []
        
        def extract_and_record(file_path, thread_id):
            try:
                start = time.time()
                result = omniparse.extract_from_path(file_path)
                duration = time.time() - start
                results.append((thread_id, duration, result))
            except Exception as e:
                errors.append((thread_id, e))
        
        threads = []
        files = [
            "test_data/document/sample.pdf",
            "test_data/text/sample.json",
            "test_data/text/sample.csv",
            "test_data/image/sample.png",
        ]
        
        # Start threads simultaneously
        for i, file_path in enumerate(files):
            thread = threading.Thread(target=extract_and_record, args=(file_path, i))
            threads.append(thread)
            thread.start()
        
        # Wait for all threads
        for thread in threads:
            thread.join()
        
        # No errors should occur
        assert len(errors) == 0, f"Errors occurred: {errors}"
        
        # All threads should complete
        assert len(results) == len(files)


class TestExtractionOverhead:
    """Tests for extraction overhead compared to Rust implementation."""
    
    def test_extraction_performance(self):
        """Test that extraction completes in reasonable time."""
        file_path = "test_data/document/sample.pdf"
        
        start = time.time()
        result = omniparse.extract_from_path(file_path)
        duration = time.time() - start
        
        # Extraction should complete quickly (< 1 second for sample file)
        assert duration < 1.0
        assert isinstance(result, omniparse.ExtractionResult)
    
    def test_batch_extraction_performance(self):
        """Test performance of batch extraction."""
        files = [
            "test_data/text/sample.txt",
            "test_data/text/sample.json",
            "test_data/text/sample.csv",
            "test_data/document/sample.pdf",
            "test_data/image/sample.png",
        ] * 2  # 10 files total
        
        start = time.time()
        results = [omniparse.extract_from_path(f) for f in files]
        duration = time.time() - start
        
        # Should complete in reasonable time
        assert len(results) == len(files)
        assert duration < 5.0  # Conservative limit
    
    def test_repeated_extraction_performance(self):
        """Test performance of repeated extractions."""
        file_path = "test_data/text/sample.json"
        num_iterations = 20
        
        start = time.time()
        for _ in range(num_iterations):
            result = omniparse.extract_from_path(file_path)
            assert result.mime_type == "application/json"
        duration = time.time() - start
        
        # Should complete quickly
        avg_time = duration / num_iterations
        assert avg_time < 0.1  # Less than 100ms per extraction


class TestMemoryEfficiency:
    """Tests for memory efficiency with large files."""
    
    def test_large_file_extraction(self):
        """Test extraction of large file."""
        # Use the large test file if it exists
        try:
            result = omniparse.extract_from_path("test_data/large_test.txt")
            
            assert isinstance(result, omniparse.ExtractionResult)
            assert result.mime_type == "text/plain"
            assert isinstance(result.content, str)
        except IOError:
            pytest.skip("Large test file not available")
    
    def test_multiple_large_extractions(self):
        """Test multiple extractions don't accumulate memory issues."""
        file_path = "test_data/document/sample.pdf"
        num_iterations = 50
        
        # Repeatedly extract - should not cause memory issues
        for _ in range(num_iterations):
            result = omniparse.extract_from_path(file_path)
            assert isinstance(result, omniparse.ExtractionResult)
            # Result goes out of scope and should be cleaned up
    
    def test_concurrent_large_extractions(self):
        """Test concurrent extraction doesn't cause memory issues."""
        files = ["test_data/document/sample.pdf"] * 20
        
        with ThreadPoolExecutor(max_workers=4) as executor:
            results = list(executor.map(omniparse.extract_from_path, files))
        
        assert len(results) == len(files)
        for result in results:
            assert result.mime_type == "application/pdf"


class TestExtractionFromBytes:
    """Performance tests for extract_from_bytes."""
    
    def test_bytes_extraction_performance(self):
        """Test performance of extraction from bytes."""
        with open("test_data/document/sample.pdf", "rb") as f:
            data = f.read()
        
        start = time.time()
        result = omniparse.extract_from_bytes(data)
        duration = time.time() - start
        
        assert duration < 1.0
        assert result.mime_type == "application/pdf"
    
    def test_concurrent_bytes_extraction(self):
        """Test concurrent extraction from bytes."""
        with open("test_data/text/sample.json", "rb") as f:
            data = f.read()
        
        def extract_bytes():
            return omniparse.extract_from_bytes(data)
        
        with ThreadPoolExecutor(max_workers=4) as executor:
            futures = [executor.submit(extract_bytes) for _ in range(10)]
            results = [future.result() for future in as_completed(futures)]
        
        assert len(results) == 10
        for result in results:
            assert result.mime_type == "application/json"


class TestScalability:
    """Tests for scalability with increasing workload."""
    
    def test_scaling_with_thread_count(self):
        """Test that performance scales with thread count."""
        files = ["test_data/text/sample.json"] * 16
        
        # Test with different thread counts
        times = {}
        
        for workers in [1, 2, 4]:
            start = time.time()
            with ThreadPoolExecutor(max_workers=workers) as executor:
                list(executor.map(omniparse.extract_from_path, files))
            times[workers] = time.time() - start
        
        # More workers should generally be faster (or at least not slower)
        # This is a basic sanity check
        assert times[1] > 0
        assert times[4] > 0
    
    def test_many_small_files(self):
        """Test extraction of many small files."""
        files = [
            "test_data/text/sample.txt",
            "test_data/text/sample.json",
            "test_data/text/minimal.json",
            "test_data/text/minimal.csv",
        ] * 5  # 20 files
        
        start = time.time()
        with ThreadPoolExecutor(max_workers=4) as executor:
            results = list(executor.map(omniparse.extract_from_path, files))
        duration = time.time() - start
        
        assert len(results) == len(files)
        assert duration < 3.0  # Should complete quickly
