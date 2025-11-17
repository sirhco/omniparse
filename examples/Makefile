.PHONY: help server client test clean

help:
	@echo "Omniparse Web Service Examples"
	@echo "==============================="
	@echo ""
	@echo "Available commands:"
	@echo "  make server  - Start the web service"
	@echo "  make client  - Run the client example (requires server running)"
	@echo "  make test    - Run the test script (requires server running)"
	@echo "  make clean   - Clean build artifacts"
	@echo ""
	@echo "Quick start:"
	@echo "  1. make server    (in one terminal)"
	@echo "  2. make test      (in another terminal)"

server:
	@echo "🚀 Starting Omniparse web service..."
	@cargo run --example web_service

client:
	@echo "🔌 Running client example..."
	@cargo run --example web_client

test:
	@echo "🧪 Running tests..."
	@bash test_web_service.sh

clean:
	@echo "🧹 Cleaning build artifacts..."
	@cargo clean
