.PHONY: help build run test vet lint check bench gacp

# Default target - show help
.DEFAULT_GOAL := help

## Help:
help: ## Show this help message
	@printf "\n\033[1mrust-hello-world\033[0m\n"
	@printf "A Rust hello-world web server with Axum\n"
	@printf "\n\033[1mUsage:\033[0m make \033[36m<target>\033[0m\n"
	@awk 'BEGIN {FS = ":.*##"; section=""} \
		/^## [A-Za-z]/ { section=substr($$0, 4); next } \
		/^[a-zA-Z_-]+:.*##/ { \
			if (section != "") { printf "\n\033[1m%s\033[0m\n", section; section="" } \
			printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2 \
		}' $(MAKEFILE_LIST)
	@printf "\n"

## Dev:
build: ## Build all packages
	cargo build

run: ## Run the web server (listens on http://localhost:3000)
	cargo run

## Quality:
test: ## Run all tests
	cargo test

vet: ## Run cargo check (fast compile without codegen)
	cargo check

lint: ## Run clippy
	cargo clippy -- -D warnings

check: build vet test ## Build, check, and test

## Git:
gacp: ## Git add, commit, push (Usage: make gacp M="type(scope): message")
	git add -A && git commit -m "$(M)" && git push
