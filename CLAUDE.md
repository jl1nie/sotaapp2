# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SOTAApp2 is a Rust backend web service for managing amateur radio award programs (SOTA - Summits on the Air, POTA - Parks on the Air). It provides APIs for activation alerts, spots, summit/park databases, and APRS real-time data integration.

## Build and Development Commands

This project uses `cargo-make` for task automation. Install with: `cargo install cargo-make`

### Common Commands

```bash
# Build
cargo make build              # Build the project
cargo make run                # Run the application locally

# Testing
cargo make test               # Run all tests with cargo-nextest (--test-threads=1)

# Linting and formatting
cargo make fmt                # Format code
cargo make clippy             # Run clippy on all targets

# Development
cargo make watch              # Watch mode: runs fmt, clippy, test on changes

# Database migrations (SQLite)
cargo make migrate run        # Run migrations
cargo make migrate add <name> # Add new migration

# Docker
cargo make compose-build-app  # Build Docker image
cargo make run-in-docker      # Run app in Docker
cargo make compose-down       # Stop Docker containers
```

### Running a Single Test

```bash
cargo nextest run <test_name> --workspace --test-threads=1
# Or for a specific package:
cargo nextest run --package domain <test_name>
```

## Architecture

The project follows **Hexagonal Architecture** with 6 layers:

```
API → Service → Domain ← Adapter
         ↓         ↓
      Registry  Common
```

### Layer Structure

- **api/** - REST endpoints (Axum), authentication middleware (Firebase Auth), request/response DTOs
- **service/** - Business logic, use cases (`UserService`, `AdminService`, `AdminPeriodicService`)
- **domain/** - Core entities and value objects (`Activation`, `SOTAReference`, `POTAReference`, `AprsLog`)
- **adapter/** - External integrations: SQLite/PostgreSQL (SQLx), APRS-IS, external APIs (SOTA/POTA/geomag)
- **registry/** - Dependency injection using Shaku framework
- **common/** - Shared utilities, configuration, error types

### Key Patterns

- **Dependency direction**: Always inward (API → Service → Domain ← Adapter)
- **Trait-based abstractions**: Services and repositories are defined as traits
- **DI container**: Shaku for runtime dependency resolution
- **Error handling**: `anyhow` + `thiserror`

## Database

- Primary: SQLite (migrations in `adapter/migrations/sqlite/`)
- Alternative: PostgreSQL with PostGIS (migrations in `adapter/migrations/postgis/`)
- Root `migrations/` symlinks to SQLite migrations

## Key Dependencies

- Web framework: Axum 0.8
- Database: SQLx with SQLite/PostgreSQL
- Auth: Firebase Authentication
- DI: Shaku
- Async runtime: Tokio
- APRS parsing: aprs-message (custom fork)
- Scheduled tasks: tokio-cron-scheduler

## Code Style

- Rust 2021 edition with rustfmt 2024 edition style
- Use type-safe wrappers for domain identifiers (e.g., `SummitCode(String)`)
- Prefer trait abstractions over concrete implementations in service layer
