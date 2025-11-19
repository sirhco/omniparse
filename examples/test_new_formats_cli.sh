#!/bin/bash
# Test script for CLI with new format support
# This script demonstrates CLI usage with all newly added formats

set -e

echo "==================================="
echo "Testing Omniparse CLI with New Formats"
echo "==================================="
echo ""

# Build the CLI
echo "Building omniparse CLI..."
cargo build --release --quiet
CLI="./target/release/omniparse"

echo ""
echo "==================================="
echo "1. HTML Format Tests"
echo "==================================="
echo ""

echo "Test 1.1: Basic HTML extraction"
$CLI test_data/text/sample.html | head -20
echo ""

echo "Test 1.2: HTML with JSON output"
$CLI --format json test_data/text/sample.html | head -15
echo ""

echo "Test 1.3: HTML metadata only"
$CLI --metadata-only test_data/text/sample.html
echo ""

echo "==================================="
echo "2. CSS Format Tests"
echo "==================================="
echo ""

echo "Test 2.1: Basic CSS extraction"
$CLI test_data/text/sample.css | head -20
echo ""

echo "Test 2.2: CSS with JSON output"
$CLI --format json --metadata-only test_data/text/sample.css
echo ""

echo "==================================="
echo "3. RTF Format Tests"
echo "==================================="
echo ""

echo "Test 3.1: Basic RTF extraction"
$CLI test_data/text/sample.rtf
echo ""

echo "Test 3.2: RTF metadata only"
$CLI --metadata-only test_data/text/sample.rtf
echo ""

echo "==================================="
echo "4. XLSX Format Tests"
echo "==================================="
echo ""

echo "Test 4.1: Basic XLSX extraction"
$CLI test_data/document/sample.xlsx | head -25
echo ""

echo "Test 4.2: XLSX metadata only"
$CLI --metadata-only test_data/document/sample.xlsx
echo ""

echo "==================================="
echo "5. PPTX Format Tests"
echo "==================================="
echo ""

echo "Test 5.1: Basic PPTX extraction"
$CLI test_data/document/sample.pptx | head -30
echo ""

echo "Test 5.2: PPTX with JSON output"
$CLI --format json --metadata-only test_data/document/sample.pptx
echo ""

echo "==================================="
echo "6. ODS Format Tests"
echo "==================================="
echo ""

echo "Test 6.1: Basic ODS extraction"
$CLI test_data/document/sample.ods | head -25
echo ""

echo "==================================="
echo "7. ODP Format Tests"
echo "==================================="
echo ""

echo "Test 7.1: Basic ODP extraction"
$CLI test_data/document/sample.odp | head -30
echo ""

echo "==================================="
echo "8. Type Detection Tests"
echo "==================================="
echo ""

echo "Test 8.1: Detect multiple new formats"
$CLI --detect-only test_data/text/sample.html test_data/text/sample.css test_data/document/sample.xlsx test_data/document/sample.pptx
echo ""

echo "==================================="
echo "9. Parallel Processing Tests"
echo "==================================="
echo ""

echo "Test 9.1: Process multiple new formats in parallel"
$CLI --parallel --verbose test_data/text/sample.html test_data/text/sample.css test_data/document/sample.xlsx test_data/document/sample.pptx 2>&1 | grep -E "(Processing|Successfully|File:|MIME Type:)" | head -20
echo ""

echo "==================================="
echo "10. Mixed Format Batch Processing"
echo "==================================="
echo ""

echo "Test 10.1: Process all text formats"
$CLI --parallel --format json --metadata-only test_data/text/sample.html test_data/text/sample.css test_data/text/sample.rtf 2>&1 | grep -E '"mime_type"' | head -10
echo ""

echo "==================================="
echo "All CLI tests completed successfully!"
echo "==================================="
