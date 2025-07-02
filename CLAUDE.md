# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Development Commands

### Build and Run
```bash
# Build and run the entire stack
docker compose up --build

# Run just the relay service
docker compose up nosrelay

# Build Rust components locally
cd event_deleter
cargo build --release
```

### Testing
```bash
# Run all tests
./run_tests.sh

# Run integration tests
./run_integration_tests.sh

# Run Rust tests
cd event_deleter
cargo test --release --lib

# Run Deno plugin tests
./strfry/plugins/tests/run_deno_tests.sh
```

### Linting and Formatting
```bash
# Rust
cargo fmt
cargo clippy

# TypeScript/Deno
deno fmt
deno lint
```

## Architecture Overview

nosrelay is a Nostr relay implementation that combines:

1. **strfry** - Core relay (C++) handling WebSocket connections and event storage (LMDB)
2. **Event Deleter** - Rust services for spam cleaning and vanish request processing
3. **Policy Plugins** - Deno/TypeScript plugins for request filtering and policy enforcement
4. **Redis** - Message queue for vanish requests

### Key Components

**strfry Relay**
- Config: `strfry/config/strfry.conf`
- Port: 7777
- Database: LMDB
- Implements NIPs: 1, 2, 4, 9, 11, 12, 16, 20, 22, 28, 33, 40, 62

**Event Deleter (Rust)**
- `spam_cleaner` - Deletes policy-violating events from stdin
- `vanish_subscriber` - Processes NIP-62 vanish requests from Redis

**Policy Pipeline (TypeScript)**
- `policies.ts` - Main pipeline: rate limiting, anti-duplication, hellthread protection
- `broadcast_vanish_requests.ts` - Broadcasts vanish requests to Redis
- `nos_policy.ts` - nos.social specific policies

### Policy Rules
- Rate limiting: 20 req/min with 2-day ban, 10 req/min soft limit
- Event size: Max 192KB normalized JSON
- Time restrictions: Events between 3 years old and 15 minutes future
- Hellthread limit: 100 participants
- Duplicate prevention: 1-day TTL

### Environment Variables
- `RELAY_URL` - WebSocket URL (e.g., `wss://example.com`)
- `REDIS_URL` - Redis connection (e.g., `redis://redis:6379`)