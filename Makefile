.DEFAULT_GOAL := help

VENV_PYTHON := $(abspath ../..)/bin/python
PYTHON ?= $(VENV_PYTHON)
PIP := $(PYTHON) -m pip
PYTEST := $(PYTHON) -m pytest

.PHONY: help
help: ## List available tasks.
	@echo "Show all available Make tasks and descriptions."
	@grep -E '^[a-zA-Z0-9_.-]+:.*## ' $(MAKEFILE_LIST) | sed -E 's/:.*## /\t- /'

.PHONY: install-dev
install-dev: ## Install editable package with development dependencies.
	@echo "Install project and development dependencies in editable mode."
	@$(PIP) install -e '.[dev]'

.PHONY: fmt
fmt: ## Run Rust formatter check.
	@echo "Check Rust formatting with rustfmt."
	@cargo fmt --check

.PHONY: test-rust
test-rust: ## Run Rust tests.
	@echo "Run Rust unit and snapshot tests."
	@cargo test

.PHONY: test-python
test-python: ## Run Python tests (uses pytest defaults from pyproject.toml).
	@echo "Run Python tests with coverage and JUnit outputs."
	@$(PYTEST)

.PHONY: test
test: ## Run all tests.
	@echo "Run Rust and Python test suites."
	@$(MAKE) test-rust
	@$(MAKE) test-python

.PHONY: snapshot-update
snapshot-update: ## Refresh Rust insta snapshots.
	@echo "Update Rust insta snapshots and run tests."
	@INSTA_UPDATE=always cargo test

.PHONY: coverage-python
coverage-python: ## Run Python coverage generation.
	@echo "Generate Python coverage artifacts configured in pytest addopts."
	@$(PYTEST)

.PHONY: coverage-rust
coverage-rust: ## Run Rust coverage check (requires cargo-llvm-cov).
	@echo "Generate Rust coverage summary and lcov artifact."
	@cargo llvm-cov --workspace --all-features --summary-only --lcov --output-path rust-coverage.lcov

.PHONY: coverage
coverage: ## Run Python and Rust coverage tasks.
	@echo "Run Python and Rust coverage commands."
	@$(MAKE) coverage-python
	@$(MAKE) coverage-rust

.PHONY: ci
ci: ## Run local CI-equivalent checks.
	@echo "Run format checks and all tests."
	@$(MAKE) fmt
	@$(MAKE) test
	@$(MAKE) coverage-rust

# @$(MAKE) coverage-python

.PHONY: clean
clean: ## Remove generated coverage and test artifacts.
	@echo "Clean generated test and coverage files."
	@rm -f coverage.xml coverage.txt pytest-junit.xml rust-coverage.lcov rust-coverage.txt
