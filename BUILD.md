# Build Guide

This guide covers how to build ichat from source for development or production.

## Prerequisites

| Tool | Version | Notes |
| :--- | :--- | :--- |
| Rust | ≥1.89 | MSRV is 1.89 |
| Node.js | ≥20 | For frontend |
| pnpm | ≥9 | Package manager |
| sqlite3 | - | For local development |
| sea-orm-cli | ≤1 | Database migration |
| typeshare | ≥1.13 | API codegen |

## Development Setup

### 1. Install Frontend Dependencies

```bash
cd frontend
pnpm install
```

### 2. Run Backend with Dev Features

Use `cargo xtask` from the `backend` directory:

```bash
cd backend
cargo xtask run
```

This runs the backend on `http://127.0.0.1:8001` with development features enabled.

### 2a. External Auth With New API

If you want ichat login and registration to use a separate New API deployment, configure that on the backend only:

```bash
export NEWAPI_AUTH_BASE="https://newapi.example.com"
```

Optional settings:

```bash
export NEWAPI_AUTH_BEARER="optional-service-token"
export NEWAPI_AUTH_USER_HEADER="New-Api-User"
```

Important:

- The browser still calls only ichat at `/api/...`
- ichat backend calls New API server-to-server
- This avoids direct browser-to-New-API CORS requirements

### 3. Build and Run Together

```bash
cd backend
cargo xtask run-with-build
```

This builds the frontend first, then runs the backend.

## Production Build

### Build Frontend Only

```bash
cd frontend
pnpm build
```

Output is in `frontend/build`.

### Build Full Application

```bash
cd backend
cargo xtask build
```

This builds the frontend and then compiles the Rust backend in release mode.

The final binary is located at `backend/target/release/backend`.

## Database Migrations

Run migrations from the `backend` directory:

```bash
cd backend
cargo xtask fresh     # Drop all tables and recreate
cargo xtask refresh  # Refresh database
```

## Code Generation

### Generate TypeScript Types

```bash
cd backend
cargo xtask gen-ts
```

Generates TypeScript types from Rust structs using typeshare.

If `cargo xtask gen-ts` fails with a typeshare parsing error, fix the Rust types first. In the current codebase, `typeshare` may reject enums in `backend/protocol/src/lib.rs` that are not tagged for export.

### Generate SeaORM Entities

```bash
cd backend
cargo xtask gen-entity
```

Requires `sea-orm-cli`:
```bash
cargo install sea-orm-cli
```

### Generate License File

```bash
cd backend
cargo xtask gen-license
```

Generates `THIRDPARTY.toml` for dependency licenses.

## Testing

### Backend Tests

```bash
cd backend
cargo test
cargo test --release  # Faster tests
```

### Frontend Tests

```bash
cd frontend
pnpm test       # Run tests
pnpm check      # TypeScript check
pnpm lint       # Format check
```

## Code Formatting

### Backend

```bash
cargo +nightly fmt
```

### Frontend

```bash
cd frontend
pnpm format
```
