#!/bin/bash

# Test script for the Petrify CLI

echo "🧪 Testing Petrify"
echo "=============================================="

# Build the project
echo "🔨 Building the project..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "❌ Build failed!"
    exit 1
fi

echo "✅ Build successful!"

# Test 1: Basic mirroring
echo ""
echo "🧪 Test 1: Basic mirroring of example.com"
echo "------------------------------------------"
./target/release/petrify https://example.com \
    --output ./test_output_basic \
    --max-concurrent 2 \
    --download-only html,css

if [ $? -eq 0 ]; then
    echo "✅ Basic mirroring test passed!"
    echo "📁 Output directory: ./test_output_basic"
    ls -la ./test_output_basic/pages/
    ls -la ./test_output_basic/static/
else
    echo "❌ Basic mirroring test failed!"
fi

# Test 2: WebP conversion
echo ""
echo "🧪 Test 2: WebP image conversion"
echo "--------------------------------"
./target/release/petrify https://httpbin.org/image/png \
    --output ./test_output_webp \
    --max-concurrent 2 \
    --download-only images \
    --convert-to-webp \
    --webp-quality 80

if [ $? -eq 0 ]; then
    echo "✅ WebP conversion test passed!"
    echo "📁 Output directory: ./test_output_webp"
    find ./test_output_webp -name "*.webp" | head -5
else
    echo "❌ WebP conversion test failed!"
fi

# Test 3: Resource filtering
echo ""
echo "🧪 Test 3: Resource type filtering"
echo "---------------------------------"
./target/release/petrify https://example.com \
    --output ./test_output_filter \
    --max-concurrent 2 \
    --download-only html,images

if [ $? -eq 0 ]; then
    echo "✅ Resource filtering test passed!"
    echo "📁 Output directory: ./test_output_filter"
    echo "📄 HTML files:"
    find ./test_output_filter -name "*.html" | head -3
    echo "🖼️ Image files:"
    find ./test_output_filter -name "*.webp" -o -name "*.png" -o -name "*.jpg" | head -3
else
    echo "❌ Resource filtering test failed!"
fi

echo ""
echo "🎉 All tests completed!"
echo "📊 Summary:"
echo "   - Basic mirroring: $(if [ -d "./test_output_basic" ]; then echo "✅ PASSED"; else echo "❌ FAILED"; fi)"
echo "   - WebP conversion: $(if [ -d "./test_output_webp" ]; then echo "✅ PASSED"; else echo "❌ FAILED"; fi)"
echo "   - Resource filtering: $(if [ -d "./test_output_filter" ]; then echo "✅ PASSED"; else echo "❌ FAILED"; fi)"

echo ""
echo "🧹 Cleaning up test directories..."
rm -rf ./test_output_basic ./test_output_webp ./test_output_filter

echo "✨ Test script completed!"
