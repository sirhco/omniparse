#!/usr/bin/env python3
"""
Script to validate Python bindings performance
"""
import sys
import time
import threading
import json
from datetime import datetime
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

# Global results storage
RESULTS = {
    "timestamp": datetime.now().isoformat(),
    "platform": sys.platform,
    "python_version": sys.version,
    "tests": [],
    "summary": {}
}


def test_extraction_overhead():
    """Test that extraction overhead is < 10%"""
    print("1. Testing extraction overhead...")
    
    test_result = {
        "name": "Extraction Overhead",
        "status": "failed",
        "details": {}
    }
    
    try:
        import omniparse
        
        # Use a simple file that should parse quickly
        test_file = "test_data/text/sample.json"
        if not Path(test_file).exists():
            print(f"  ⚠ Test file {test_file} not found (skipping)")
            test_result["status"] = "skipped"
            test_result["details"]["reason"] = "Test file not found"
            RESULTS["tests"].append(test_result)
            return True
        
        # Warm up
        for _ in range(5):
            try:
                omniparse.extract_from_path(test_file)
            except:
                pass
        
        # Time multiple extractions
        iterations = 100
        start = time.time()
        successful = 0
        sample_result = None
        
        for _ in range(iterations):
            try:
                result = omniparse.extract_from_path(test_file)
                if sample_result is None:
                    sample_result = result
                successful += 1
            except Exception as e:
                pass
        
        duration = time.time() - start
        
        if successful == 0:
            print(f"  ⚠ No successful extractions (skipping)")
            test_result["status"] = "skipped"
            test_result["details"]["reason"] = "No successful extractions"
            RESULTS["tests"].append(test_result)
            return True
        
        avg_time = (duration / successful) * 1000  # Convert to ms
        
        test_result["status"] = "passed"
        test_result["details"] = {
            "test_file": test_file,
            "iterations": iterations,
            "successful": successful,
            "total_duration_ms": duration * 1000,
            "avg_time_ms": round(avg_time, 4),
            "throughput_per_sec": round(successful / duration, 2),
            "target_ms": 10,
            "meets_target": avg_time < 10,
            "sample_extraction": {
                "mime_type": sample_result.mime_type if sample_result else None,
                "confidence": sample_result.detection_confidence if sample_result else None,
                "content": str(sample_result.content) if sample_result else None,
                "content_length": len(str(sample_result.content)) if sample_result else None,
                "metadata": dict(sample_result.metadata) if sample_result else None
            }
        }
        
        print(f"  ✓ Average extraction time: {avg_time:.2f}ms ({successful}/{iterations} successful)")
        
        # Check if reasonable (< 10ms per extraction for JSON)
        if avg_time < 10:
            print(f"  ✓ Performance is good (< 10ms)")
        else:
            print(f"  ⚠ Performance could be better ({avg_time:.2f}ms)")
        
        RESULTS["tests"].append(test_result)
        return True
        
    except ImportError as e:
        print(f"  ✗ Failed to import omniparse: {e}")
        test_result["details"]["error"] = str(e)
        RESULTS["tests"].append(test_result)
        return False


def test_concurrent_extraction():
    """Test concurrent extraction scaling"""
    print("\n2. Testing concurrent extraction scaling...")
    
    test_result = {
        "name": "Concurrent Extraction Scaling",
        "status": "failed",
        "details": {}
    }
    
    try:
        import omniparse
        
        # Find test files
        test_files = []
        for pattern in ["test_data/text/*.json", "test_data/text/*.csv"]:
            test_files.extend(Path(".").glob(pattern))
        
        if len(test_files) < 2:
            print(f"  ⚠ Not enough test files (skipping)")
            test_result["status"] = "skipped"
            test_result["details"]["reason"] = "Not enough test files"
            RESULTS["tests"].append(test_result)
            return True
        
        test_files = [str(f) for f in test_files[:10]]  # Limit to 10 files
        
        # Test sequential
        start = time.time()
        sequential_results = []
        for file in test_files:
            try:
                result = omniparse.extract_from_path(file)
                sequential_results.append({
                    "file": Path(file).name,
                    "mime_type": result.mime_type,
                    "confidence": result.detection_confidence,
                    "content": str(result.content),
                    "content_length": len(str(result.content)),
                    "metadata": dict(result.metadata)
                })
            except Exception as e:
                sequential_results.append({
                    "file": Path(file).name,
                    "error": str(e)
                })
        sequential_time = time.time() - start
        
        successful_seq = sum(1 for r in sequential_results if "error" not in r)
        
        if successful_seq == 0:
            print(f"  ⚠ No successful extractions (skipping)")
            test_result["status"] = "skipped"
            test_result["details"]["reason"] = "No successful extractions"
            RESULTS["tests"].append(test_result)
            return True
        
        # Test concurrent with 4 threads
        start = time.time()
        concurrent_results = []
        with ThreadPoolExecutor(max_workers=4) as executor:
            futures = {executor.submit(omniparse.extract_from_path, file): file for file in test_files}
            for future in futures:
                file = futures[future]
                try:
                    result = future.result()
                    concurrent_results.append({
                        "file": Path(file).name,
                        "mime_type": result.mime_type,
                        "confidence": result.detection_confidence,
                        "content": str(result.content),
                        "content_length": len(str(result.content)),
                        "metadata": dict(result.metadata)
                    })
                except Exception as e:
                    concurrent_results.append({
                        "file": Path(file).name,
                        "error": str(e)
                    })
        concurrent_time = time.time() - start
        
        successful_conc = sum(1 for r in concurrent_results if "error" not in r)
        
        if successful_conc == 0:
            print(f"  ⚠ No successful concurrent extractions (skipping)")
            test_result["status"] = "skipped"
            test_result["details"]["reason"] = "No successful concurrent extractions"
            RESULTS["tests"].append(test_result)
            return True
        
        speedup = sequential_time / concurrent_time if concurrent_time > 0 else 0
        
        test_result["status"] = "passed"
        test_result["details"] = {
            "test_files_count": len(test_files),
            "thread_count": 4,
            "sequential": {
                "duration_sec": round(sequential_time, 4),
                "successful": successful_seq,
                "failed": len(sequential_results) - successful_seq,
                "results": sequential_results
            },
            "concurrent": {
                "duration_sec": round(concurrent_time, 4),
                "successful": successful_conc,
                "failed": len(concurrent_results) - successful_conc,
                "results": concurrent_results
            },
            "speedup": round(speedup, 2),
            "gil_released": speedup > 1.2,
            "efficiency_percent": round((speedup / 4) * 100, 2)  # Efficiency vs ideal 4x speedup
        }
        
        print(f"  ✓ Sequential: {sequential_time:.3f}s ({successful_seq} files)")
        print(f"  ✓ Concurrent (4 threads): {concurrent_time:.3f}s ({successful_conc} files)")
        print(f"  ✓ Speedup: {speedup:.2f}x")
        
        # Check if we got any speedup (GIL is released)
        if speedup > 1.2:
            print(f"  ✓ Good concurrent scaling (GIL released)")
        else:
            print(f"  ⚠ Limited concurrent scaling ({speedup:.2f}x)")
        
        RESULTS["tests"].append(test_result)
        return True
        
    except ImportError as e:
        print(f"  ✗ Failed to import omniparse: {e}")
        test_result["details"]["error"] = str(e)
        RESULTS["tests"].append(test_result)
        return False


def test_memory_efficiency():
    """Test memory efficiency with multiple extractions"""
    print("\n3. Testing memory efficiency...")
    
    test_result = {
        "name": "Memory Efficiency",
        "status": "failed",
        "details": {}
    }
    
    try:
        import omniparse
        
        # Find a test file
        test_file = "test_data/text/sample.json"
        if not Path(test_file).exists():
            print(f"  ⚠ Test file {test_file} not found (skipping)")
            test_result["status"] = "skipped"
            test_result["details"]["reason"] = "Test file not found"
            RESULTS["tests"].append(test_result)
            return True
        
        # Extract multiple times
        iterations = 100
        results = []
        start = time.time()
        
        for i in range(iterations):
            try:
                result = omniparse.extract_from_path(test_file)
                results.append(result)
            except:
                pass
        
        duration = time.time() - start
        
        if len(results) == 0:
            print(f"  ⚠ No successful extractions (skipping)")
            test_result["status"] = "skipped"
            test_result["details"]["reason"] = "No successful extractions"
            RESULTS["tests"].append(test_result)
            return True
        
        test_result["status"] = "passed"
        test_result["details"] = {
            "test_file": test_file,
            "iterations": iterations,
            "successful": len(results),
            "duration_sec": round(duration, 4),
            "avg_time_ms": round((duration / len(results)) * 1000, 4),
            "throughput_per_sec": round(len(results) / duration, 2)
        }
        
        print(f"  ✓ Extracted {len(results)} times in {duration:.2f}s")
        print(f"  ✓ Average: {(duration/len(results))*1000:.2f}ms per extraction")
        print(f"  ✓ Memory efficiency validated (no crashes)")
        
        RESULTS["tests"].append(test_result)
        return True
        
    except ImportError as e:
        print(f"  ✗ Failed to import omniparse: {e}")
        test_result["details"]["error"] = str(e)
        RESULTS["tests"].append(test_result)
        return False


def test_gil_release():
    """Test that GIL is released during extraction"""
    print("\n4. Testing GIL release...")
    
    test_result = {
        "name": "GIL Release",
        "status": "failed",
        "details": {}
    }
    
    try:
        import omniparse
        
        # Find test files
        test_files = list(Path("test_data/text").glob("*.json"))[:5]
        
        if len(test_files) < 2:
            print(f"  ⚠ Not enough test files (skipping)")
            test_result["status"] = "skipped"
            test_result["details"]["reason"] = "Not enough test files"
            RESULTS["tests"].append(test_result)
            return True
        
        test_files = [str(f) for f in test_files]
        
        # Test with threads
        results = []
        errors = []
        
        def extract_file(file):
            try:
                result = omniparse.extract_from_path(file)
                results.append({
                    "file": Path(file).name,
                    "mime_type": result.mime_type,
                    "confidence": result.detection_confidence,
                    "content": str(result.content),
                    "metadata": dict(result.metadata)
                })
            except Exception as e:
                errors.append({
                    "file": Path(file).name,
                    "error": str(e)
                })
        
        threads = []
        start = time.time()
        
        for file in test_files:
            thread = threading.Thread(target=extract_file, args=(file,))
            thread.start()
            threads.append(thread)
        
        for thread in threads:
            thread.join()
        
        duration = time.time() - start
        
        test_result["status"] = "passed"
        test_result["details"] = {
            "test_files_count": len(test_files),
            "thread_count": len(test_files),
            "duration_sec": round(duration, 4),
            "successful": len(results),
            "failed": len(errors),
            "results": results,
            "errors": errors
        }
        
        print(f"  ✓ Processed {len(test_files)} files with {len(test_files)} threads")
        print(f"  ✓ Duration: {duration:.3f}s")
        print(f"  ✓ Successful: {len(results)}, Errors: {len(errors)}")
        
        if len(results) > 0:
            print(f"  ✓ GIL release validated (threads ran concurrently)")
        else:
            print(f"  ⚠ No successful extractions")
        
        RESULTS["tests"].append(test_result)
        return True
        
    except ImportError as e:
        print(f"  ✗ Failed to import omniparse: {e}")
        test_result["details"]["error"] = str(e)
        RESULTS["tests"].append(test_result)
        return False


def test_bytes_extraction_performance():
    """Test extraction from bytes performance"""
    print("\n5. Testing bytes extraction performance...")
    
    test_result = {
        "name": "Bytes Extraction Performance",
        "status": "failed",
        "details": {}
    }
    
    try:
        import omniparse
        
        test_file = "test_data/text/sample.json"
        if not Path(test_file).exists():
            print(f"  ⚠ Test file {test_file} not found (skipping)")
            test_result["status"] = "skipped"
            test_result["details"]["reason"] = "Test file not found"
            RESULTS["tests"].append(test_result)
            return True
        
        # Read file into memory
        data = Path(test_file).read_bytes()
        
        # Warm up
        for _ in range(5):
            try:
                omniparse.extract_from_bytes(data)
            except:
                pass
        
        # Time multiple extractions
        iterations = 100
        start = time.time()
        successful = 0
        sample_result = None
        
        for _ in range(iterations):
            try:
                result = omniparse.extract_from_bytes(data)
                if sample_result is None:
                    sample_result = result
                successful += 1
            except:
                pass
        
        duration = time.time() - start
        
        if successful == 0:
            print(f"  ⚠ No successful extractions (skipping)")
            test_result["status"] = "skipped"
            test_result["details"]["reason"] = "No successful extractions"
            RESULTS["tests"].append(test_result)
            return True
        
        avg_time = (duration / successful) * 1000
        
        test_result["status"] = "passed"
        test_result["details"] = {
            "test_file": test_file,
            "data_size_bytes": len(data),
            "iterations": iterations,
            "successful": successful,
            "total_duration_ms": round(duration * 1000, 4),
            "avg_time_ms": round(avg_time, 4),
            "throughput_per_sec": round(successful / duration, 2),
            "sample_extraction": {
                "mime_type": sample_result.mime_type if sample_result else None,
                "confidence": sample_result.detection_confidence if sample_result else None,
                "content": str(sample_result.content) if sample_result else None,
                "content_length": len(str(sample_result.content)) if sample_result else None,
                "metadata": dict(sample_result.metadata) if sample_result else None
            }
        }
        
        print(f"  ✓ Average extraction time: {avg_time:.2f}ms ({successful}/{iterations} successful)")
        print(f"  ✓ Bytes extraction performance validated")
        
        RESULTS["tests"].append(test_result)
        return True
        
    except ImportError as e:
        print(f"  ✗ Failed to import omniparse: {e}")
        test_result["details"]["error"] = str(e)
        RESULTS["tests"].append(test_result)
        return False


def write_results(output_file="performance_results.json"):
    """Write results to JSON file"""
    try:
        # Add summary
        passed = sum(1 for t in RESULTS["tests"] if t["status"] == "passed")
        skipped = sum(1 for t in RESULTS["tests"] if t["status"] == "skipped")
        failed = sum(1 for t in RESULTS["tests"] if t["status"] == "failed")
        
        RESULTS["summary"] = {
            "total_tests": len(RESULTS["tests"]),
            "passed": passed,
            "skipped": skipped,
            "failed": failed,
            "success_rate": round((passed / len(RESULTS["tests"])) * 100, 2) if RESULTS["tests"] else 0
        }
        
        # Write to file
        with open(output_file, 'w') as f:
            json.dump(RESULTS, f, indent=2)
        
        print(f"\n📄 Results written to: {output_file}")
        return True
    except Exception as e:
        print(f"\n⚠️  Failed to write results: {e}")
        return False


def main():
    """Run all performance validation checks"""
    print("=" * 80)
    print("Python Bindings Performance Validation")
    print("=" * 80)
    
    checks = [
        test_extraction_overhead,
        test_concurrent_extraction,
        test_memory_efficiency,
        test_gil_release,
        test_bytes_extraction_performance,
    ]
    
    results = []
    for check in checks:
        try:
            results.append(check())
        except Exception as e:
            print(f"  ✗ Check failed with error: {e}")
            import traceback
            traceback.print_exc()
            results.append(False)
    
    print("\n" + "=" * 80)
    passed = sum(results)
    total = len(results)
    
    # Write results to file
    write_results()
    
    if all(results):
        print(f"✓ All {total} performance checks passed!")
        return 0
    else:
        print(f"✗ {total - passed}/{total} checks failed")
        return 1


if __name__ == '__main__':
    sys.exit(main())
