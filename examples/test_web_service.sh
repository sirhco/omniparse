#!/bin/bash
# Test script for the Omniparse web service example
#
# Usage:
#   1. Start the server: cargo run --example web_service
#   2. Run this script in another terminal: bash examples/test_web_service.sh

set -e

BASE_URL="http://localhost:3000"

echo "🧪 Testing Omniparse Web Service"
echo "================================="
echo ""

# Test health check
echo "1️⃣  Testing health check..."
curl -s "$BASE_URL/health" | jq '.'
echo ""

# Test root endpoint
echo "2️⃣  Testing root endpoint..."
curl -s "$BASE_URL/"
echo ""
echo ""

# Test JSON file parsing
echo "3️⃣  Testing JSON file parsing..."
curl -s -X POST -F "file=@test_data/text/sample.json" "$BASE_URL/parse" | jq '.'
echo ""

# Test CSV file parsing
echo "4️⃣  Testing CSV file parsing..."
curl -s -X POST -F "file=@test_data/text/sample.csv" "$BASE_URL/parse" | jq '.'
echo ""

# Test metadata-only mode
echo "5️⃣  Testing metadata-only mode..."
curl -s -X POST -F "file=@test_data/text/sample.json" "$BASE_URL/parse?metadata_only=true" | jq '.'
echo ""

# Test file type detection
echo "6️⃣  Testing file type detection..."
curl -s -X POST -F "file=@test_data/document/sample.docx" "$BASE_URL/detect" | jq '.'
echo ""

# Test DOCX file parsing
echo "7️⃣  Testing DOCX file parsing..."
curl -s -X POST -F "file=@test_data/document/sample.docx" "$BASE_URL/parse" | jq '.'
echo ""

# Test TAR archive parsing
echo "8️⃣  Testing TAR archive parsing..."
curl -s -X POST -F "file=@test_data/archive/sample.tar" "$BASE_URL/parse" | jq '.'
echo ""

# Test error handling - missing file
echo "9️⃣  Testing error handling (missing file)..."
curl -s -X POST "$BASE_URL/parse" | jq '.'
echo ""

echo "✅ All tests completed!"
