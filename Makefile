.PHONY: all build start stop status logs clean help

# Default target
all: help

# Show help options
help:
	@echo "Void Grid 3v3 CTF Server Control Panel"
	@echo "======================================"
	@echo "Usage:"
	@echo "  make build      - Compile the server in release mode"
	@echo "  make start      - Build and start the server in the background"
	@echo "  make stop       - Stop the running server"
	@echo "  make status     - Check if the server is running"
	@echo "  make logs       - Tail the server logs"
	@echo "  make clean      - Clean build artifacts and logs"

# Compile the server in release mode
build:
	@echo "Compiling Void Grid server..."
	cargo build --release

# Start the server in the background
start: build
	@if [ -f server.pid ] && kill -0 $$(cat server.pid) 2>/dev/null; then \
		echo "Server is already running (PID: $$(cat server.pid))"; \
	else \
		echo "Starting server..."; \
		nohup ./target/release/voidgrid > server.log 2>&1 & echo $$! > server.pid; \
		sleep 1.5; \
		if kill -0 $$(cat server.pid) 2>/dev/null; then \
			echo "Server started successfully (PID: $$(cat server.pid))"; \
			echo "Access it at http://localhost:8082/"; \
			echo "Logs are being written to server.log"; \
		else \
			echo "Failed to start server. Check server.log for details."; \
			rm -f server.pid; \
			exit 1; \
		fi; \
	fi

# Stop the server
stop:
	@if [ -f server.pid ]; then \
		PID=$$(cat server.pid); \
		echo "Stopping server (PID: $$PID)..."; \
		kill -15 $$PID 2>/dev/null || true; \
		for i in 1 2 3 4 5; do \
			if ! kill -0 $$PID 2>/dev/null; then \
				break; \
			fi; \
			sleep 1; \
		done; \
		if kill -0 $$PID 2>/dev/null; then \
			echo "Server did not stop gracefully, forcing shutdown..."; \
			kill -9 $$PID 2>/dev/null || true; \
		fi; \
		rm -f server.pid; \
		echo "Server stopped."; \
	else \
		echo "No server.pid found. Checking for any running voidgrid processes..."; \
		if pkill -f target/release/voidgrid; then \
			echo "Stopped running processes."; \
		else \
			echo "No running server found."; \
		fi; \
	fi

# Check server status
status:
	@if [ -f server.pid ] && kill -0 $$(cat server.pid) 2>/dev/null; then \
		echo "Server is RUNNING (PID: $$(cat server.pid))"; \
		echo "URL: http://localhost:8082/"; \
	else \
		echo "Server is STOPPED"; \
	fi

# Tail server logs
logs:
	@if [ -f server.log ]; then \
		tail -n 100 -f server.log; \
	else \
		echo "No log file found (server.log). Start the server first."; \
	fi

# Clean build artifacts and log files
clean:
	@echo "Cleaning up..."
	rm -f server.pid server.log
	cargo clean
	@echo "Cleanup complete."
