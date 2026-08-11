.PHONY: help dev run build test test-pg migrate migrate-pg schema lint lint-pg fmt check docker-up docker-down gacp

# Default target - show help
.DEFAULT_GOAL := help

## Help:
help: ## Show this help message
	@printf "\n\033[1mrust-web-starter\033[0m\n"
	@printf "Axum + Diesel web service starter\n"
	@printf "\n\033[1mUsage:\033[0m make \033[36m<target>\033[0m\n"
	@awk 'BEGIN {FS = ":.*##"; section=""} \
		/^## [A-Za-z]/ { section=substr($$0, 4); next } \
		/^[a-zA-Z_-]+:.*##/ { \
			if (section != "") { printf "\n\033[1m%s\033[0m\n", section; section="" } \
			printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2 \
		}' $(MAKEFILE_LIST)
	@printf "\n"

## Dev:
dev: ## Hot reload: rebuild and restart the server on save (bacon)
	bacon run

run: ## Run the web server (listens on http://localhost:4563)
	cargo run

build: ## Build all packages
	cargo build

## Database:
migrate: ## Run SQLite migrations
	diesel migration run --migration-dir migrations/sqlite

migrate-pg: ## Run PostgreSQL migrations
	diesel migration run --migration-dir migrations/postgres

schema: ## Generate schema.rs from database
	diesel print-schema > src/db/schema.rs

## Testing:
test: ## Run all tests (SQLite backend)
	cargo test

# Without a DATABASE_URL the postgres-compiled test binary gets handed a sqlite path and
# every test fails with "invalid connection string", so start a throwaway server instead.
PG_TEST_URL ?= postgres://postgres@localhost:55432/postgres

test-pg: ## Run all tests (PostgreSQL). Starts a throwaway server unless DATABASE_URL is set
ifdef DATABASE_URL
	cargo test --no-default-features --features postgres
else
	@docker rm -f rws-test-pg >/dev/null 2>&1 || true
	@docker run -d --rm --name rws-test-pg -e POSTGRES_HOST_AUTH_METHOD=trust \
		-p 55432:5432 postgres:17-alpine >/dev/null
	@for i in $$(seq 1 60); do docker exec rws-test-pg pg_isready -q 2>/dev/null && break; sleep 1; done
	@DATABASE_URL=$(PG_TEST_URL) cargo test --no-default-features --features postgres; \
		status=$$?; docker rm -f rws-test-pg >/dev/null 2>&1; exit $$status
endif

## Quality:
fmt: ## Format code (cargo fmt)
	cargo fmt --all

lint: ## Lint code (SQLite + PostgreSQL)
	cargo clippy --all-targets -- -D warnings
	cargo clippy --all-targets --no-default-features --features postgres -- -D warnings

lint-pg: ## Lint code (PostgreSQL backend only)
	cargo clippy --all-targets --no-default-features --features postgres -- -D warnings

check: ## Format check + lint + test (both backends)
	cargo fmt --all -- --check
	cargo clippy --all-targets -- -D warnings
	cargo clippy --all-targets --no-default-features --features postgres -- -D warnings
	cargo test
	$(MAKE) test-pg

## Docker:
docker-up: ## Start services via docker-compose
	docker compose up --build

docker-down: ## Stop and remove services
	docker compose down -v

## Git:
gacp: ## Git add, commit, push (Usage: make gacp M="type(scope): message")
	git add -A && git commit -m "$(M)" && git push
