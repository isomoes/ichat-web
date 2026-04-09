SHELL := /bin/sh

FRONTEND_DIR := frontend
BACKEND_DIR := backend

.PHONY: help install build-frontend dev dev-build build fresh refresh \
	gen-ts gen-entity gen-license test test-backend test-backend-release \
	test-frontend check-frontend lint-frontend fmt fmt-backend fmt-frontend

help:
	@printf '%s\n' \
		'ichat build targets:' \
		'  make install              Install frontend dependencies' \
		'  make dev                  Run backend and frontend dev servers' \
		'  make dev-build            Build frontend, then run backend' \
		'  make build-frontend       Build frontend only' \
		'  make build                Build full application' \
		'  make fresh                Drop and recreate database' \
		'  make refresh              Refresh database' \
		'  make gen-ts               Generate TypeScript types' \
		'  make gen-entity           Generate SeaORM entities' \
		'  make gen-license          Generate backend license file' \
		'  make test                 Run backend and frontend tests/checks' \
		'  make fmt                  Format backend and frontend'

install:
	pnpm install --dir $(FRONTEND_DIR)

build-frontend:
	pnpm build --dir $(FRONTEND_DIR)

dev:
	@set -eu; \
	pnpm --dir $(FRONTEND_DIR) dev & \
	frontend_pid=$$!; \
	trap 'kill "'"'$$frontend_pid'"'" 2>/dev/null || true' INT TERM EXIT; \
	cd $(BACKEND_DIR) && cargo xtask run

dev-build:
	cd $(BACKEND_DIR) && cargo xtask run-with-build

build:
	cd $(BACKEND_DIR) && cargo xtask build

fresh:
	cd $(BACKEND_DIR) && cargo xtask fresh

refresh:
	cd $(BACKEND_DIR) && cargo xtask refresh

gen-ts:
	cd $(BACKEND_DIR) && cargo xtask gen-ts

gen-entity:
	cd $(BACKEND_DIR) && cargo xtask gen-entity

gen-license:
	cd $(BACKEND_DIR) && cargo xtask gen-license

test: test-backend test-frontend check-frontend lint-frontend

test-backend:
	cargo test --manifest-path $(BACKEND_DIR)/Cargo.toml

test-backend-release:
	cargo test --release --manifest-path $(BACKEND_DIR)/Cargo.toml

test-frontend:
	pnpm test --dir $(FRONTEND_DIR)

check-frontend:
	pnpm check --dir $(FRONTEND_DIR)

lint-frontend:
	pnpm lint --dir $(FRONTEND_DIR)

fmt: fmt-backend fmt-frontend

fmt-backend:
	cargo +nightly fmt --manifest-path $(BACKEND_DIR)/Cargo.toml --all

fmt-frontend:
	pnpm format --dir $(FRONTEND_DIR)
