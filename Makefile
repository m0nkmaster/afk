# afk Makefile
# Development and testing commands

.PHONY: help install test lint format typecheck quality sandbox-setup sandbox-run sandbox-reset sandbox-status

SANDBOX_DIR := examples/sandbox

help:
	@echo "afk development commands:"
	@echo ""
	@echo "  Development:"
	@echo "    make install      Install package in dev mode"
	@echo "    make test         Run tests with coverage"
	@echo "    make lint         Run linter (ruff check)"
	@echo "    make format       Format code (ruff format)"
	@echo "    make typecheck    Run type checker (mypy)"
	@echo "    make quality      Run all quality checks"
	@echo ""
	@echo "  Sandbox (integration testing):"
	@echo "    make sandbox-setup   Initialise the sandbox project"
	@echo "    make sandbox-run     Run afk go in the sandbox"
	@echo "    make sandbox-reset   Reset sandbox to initial state"
	@echo "    make sandbox-status  Show sandbox state"

# =============================================================================
# Development
# =============================================================================

install:
	pip install -e ".[dev]"

test:
	pytest

lint:
	ruff check .

format:
	ruff format .

typecheck:
	mypy src/afk

quality: lint typecheck test

# =============================================================================
# Sandbox - Integration Test Harness
# =============================================================================

sandbox-setup:
	@echo "Setting up sandbox..."
	@cd $(SANDBOX_DIR) && \
		if [ ! -d .git ]; then \
			git init && \
			git add -A && \
			git commit -m "Initial sandbox scaffold"; \
		fi
	@cd $(SANDBOX_DIR) && \
		pip install -e ".[dev]" --quiet
	@cd $(SANDBOX_DIR) && \
		afk init --yes
	@cd $(SANDBOX_DIR) && \
		afk prd parse PRD.md --stdout > /dev/null && \
		echo "Run: cd $(SANDBOX_DIR) && afk prd parse PRD.md --copy"
	@echo ""
	@echo "Sandbox ready! Next steps:"
	@echo "  1. cd $(SANDBOX_DIR)"
	@echo "  2. afk prd parse PRD.md --copy  # Then paste to AI to create prd.json"
	@echo "  3. afk go 3                     # Run 3 iterations"
	@echo ""
	@echo "Or run: make sandbox-run"

sandbox-run:
	@echo "Running afk in sandbox..."
	cd $(SANDBOX_DIR) && afk go 3

sandbox-reset:
	@echo "Resetting sandbox..."
	@cd $(SANDBOX_DIR) && \
		if [ -d .git ]; then \
			git checkout . && \
			git clean -fd; \
		fi
	@rm -rf $(SANDBOX_DIR)/.afk
	@rm -f $(SANDBOX_DIR)/tasks.json
	@echo "Sandbox reset to initial state."

sandbox-status:
	@echo "Sandbox status:"
	@echo ""
	@if [ -d $(SANDBOX_DIR)/.afk ]; then \
		echo "  .afk directory: exists"; \
		if [ -f $(SANDBOX_DIR)/.afk/prd.json ]; then \
			echo "  prd.json: exists"; \
		else \
			echo "  prd.json: missing (run 'afk prd parse PRD.md')"; \
		fi; \
		if [ -f $(SANDBOX_DIR)/.afk/progress.json ]; then \
			echo "  progress.json: exists"; \
		else \
			echo "  progress.json: no session started"; \
		fi; \
	else \
		echo "  .afk directory: not initialised (run 'make sandbox-setup')"; \
	fi
	@echo ""
	@if [ -d $(SANDBOX_DIR)/.git ]; then \
		echo "  Git status:"; \
		cd $(SANDBOX_DIR) && git status --short | head -10; \
	else \
		echo "  Git: not initialised"; \
	fi
	@echo ""
	@echo "  Tests:"
	@cd $(SANDBOX_DIR) && pytest --tb=no -q 2>/dev/null || echo "  (some tests failing - expected)"
