# Rust Backend Workspace Template

A production-ready Rust workspace template for building scalable backends. Ships with a **REST API server**, a **WebSocket server**, a **database layer**, and a **shared common crate** — all wired together in a single Cargo workspace.

---

## Workspace Structure

```
.
├── Cargo.toml          # Workspace root — declares all members
├── backend/            # HTTP REST API server (Actix-web)
├── db/                 # Database layer (SQLx + PostgreSQL)
├── ws/                 # WebSocket server (Actix-ws)
└── common/             # Shared types and utilities
```

---

## Crates

### `db` — Database Layer

**Stack:** `sqlx` (PostgreSQL, async, compile-time checked queries)

This crate owns everything related to the database. It exposes a `Db` struct that wraps a SQLx connection pool and implements all database operations as methods.

**What's included:**
- `Db::new()` — initialises a PostgreSQL connection pool from the `DATABASE_URL` env variable
- `models/user.rs` — User model with `create_user` and `get_user_by_username` queries using `sqlx::query_as!`

**How to extend:**
- Add a new file under `db/src/models/` for each domain entity (e.g. `post.rs`, `room.rs`)
- Implement `impl Db { ... }` blocks in each model file for that entity's queries
- Register the new module in `db/src/models/mod.rs`

**Key dependencies:**
| Crate | Purpose |
|---|---|
| `sqlx` | Async, compile-time verified SQL queries |
| `serde` | Serialise/deserialise DB structs |
| `anyhow` | Ergonomic error handling |
| `dotenvy` | Load `.env` file for `DATABASE_URL` |

**Environment variable required:**
```
DATABASE_URL=postgres://user:password@localhost:5432/dbname
```

---

### `backend` — HTTP REST API Server

**Stack:** `actix-web`, `jsonwebtoken`, `uuid`

The HTTP server. It imports the `db` crate directly and uses it as shared application state via `web::Data<Db>`.

**What's included:**
- `POST /signup` — creates a new user (hashed password recommended before storing)
- `POST /signin` — validates credentials and returns a signed JWT
- `middleware.rs` — a custom `FromRequest` extractor (`JwtClaims`) that validates the `Authorization` header on any protected route

**How to add a protected route:**
```rust
// In your route handler, just add JwtClaims as a parameter
async fn my_protected_route(claims: JwtClaims, db: web::Data<Db>) -> impl Responder {
    let user_id = &claims.0.sub; // extracted from the JWT
    // ...
}
```

**How to extend:**
- Add new route files under `backend/src/routes/`
- Register them in `backend/src/routes/mod.rs`
- Mount them in `main.rs` with `.service(...)`

**Key dependencies:**
| Crate | Purpose |
|---|---|
| `actix-web` | HTTP framework |
| `jsonwebtoken` | JWT sign & verify |
| `uuid` | Generate unique IDs |
| `serde` / `serde_json` | JSON request/response |
| `anyhow` | Error handling |
| `dotenvy` | Load `.env` |
| `db` | Internal workspace crate |

**Environment variables required:**
```
DATABASE_URL=postgres://user:password@localhost:5432/dbname
SECRET_KEY=your_jwt_secret_here
```

Server binds on `0.0.0.0:3000` by default.

---

### `ws` — WebSocket Server

**Stack:** `actix-web`, `actix-ws`, `tokio`, `snowflake`

A fully working WebSocket server with a **room-based pub/sub system**. Clients can connect, join rooms, send messages to rooms, and leave rooms.

**What's included:**

#### `room_manager.rs` — Connection & Room Manager
- `RoomManager` — holds all active connections (`clients`) and room subscriptions (`subscriptions`)
- Each connected client is identified by a `ProcessUniqueId` (snowflake ID — crash-safe, unique across the process lifetime)
- Each client has a `tokio::mpsc::Sender<String>` channel for sending messages back to them
- `RoomManager::broadcast(room_id, message)` — fans out a message to every client subscribed to a room

#### `types.rs` — WebSocket Message Protocol
Incoming messages are a JSON-serialised enum:

```json
// Join a room
{ "JoinRoom": "room-id-here" }

// Leave a room
{ "LeaveRoom": "room-id-here" }

// Send a message to a room
{ "Message": { "room_id": "room-id-here", "message": "hello!" } }
```

**How to extend:**
- Add new variants to the `Message` enum in `ws/src/types.rs` to handle new event types
- Add new manager logic in `room_manager.rs` (e.g. track user metadata, limit room sizes, persist messages)
- Integrate the `db` crate to persist room history

**Key dependencies:**
| Crate | Purpose |
|---|---|
| `actix-web` | HTTP upgrade to WebSocket |
| `actix-ws` | WebSocket session management |
| `tokio` | Async runtime + `mpsc` channels |
| `snowflake` | Process-unique IDs for clients |
| `serde` / `serde_json` | JSON message parsing |

Server binds on `0.0.0.0:8080` by default (configure in `ws/src/main.rs`).

---

### `common` — Shared Types & Utilities

A library crate for anything that needs to be shared across `backend`, `ws`, or `db` without creating circular dependencies.

**When to use this crate:**
- Shared error types
- Shared request/response DTOs used by both the HTTP and WebSocket servers
- Utility functions (e.g. password hashing, token generation helpers)
- Constants

**How to use it in another crate:**
```toml
# In backend/Cargo.toml or ws/Cargo.toml
[dependencies]
common = { path = "../common" }
```

---

## Getting Started

### Prerequisites
- Rust (stable, 2024 edition) — install via [rustup](https://rustup.rs)
- PostgreSQL running locally (or a connection string to a remote instance)

### 1. Clone and configure environment

Create a `.env` file inside the `backend/` directory:
```
DATABASE_URL=postgres://user:password@localhost:5432/mydb
SECRET_KEY=supersecretjwtkey
```

### 2. Run database migrations

This template uses raw SQL with SQLx. Create your tables manually or use `sqlx-cli`:
```bash
cargo install sqlx-cli
sqlx migrate run --source db/migrations
```

Example initial migration for the `users` table:
```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username TEXT NOT NULL UNIQUE,
    password TEXT NOT NULL
);
```

### 3. Run the HTTP API server
```bash
cargo run -p backend
```

### 4. Run the WebSocket server
```bash
cargo run -p ws
```

### 5. Build the entire workspace
```bash
cargo build --workspace
```

---

## Architecture Overview

```
┌─────────────────────────────────────────────┐
│                  Clients                    │
│         (Browser / Mobile / CLI)            │
└────────────┬──────────────────┬─────────────┘
             │ HTTP REST        │ WebSocket
             ▼                  ▼
┌────────────────┐   ┌──────────────────────────┐
│   backend/     │   │         ws/              │
│                │   │                          │
│  Actix-web     │   │  Actix-ws + RoomManager  │
│  JWT Auth      │   │  Room-based pub/sub       │
│  REST Routes   │   │  Snowflake client IDs     │
└───────┬────────┘   └──────────────────────────┘
        │
        ▼
┌────────────────┐
│     db/        │
│                │
│  SQLx + PG     │
│  Typed queries │
│  User model    │
└────────────────┘
        ▲
┌────────────────┐
│   common/      │
│  Shared types  │
│  Shared utils  │
└────────────────┘
```

---

## Recommended Extensions to This Template

| Feature | Where to add | Suggested crates |
|---|---|---|
| Password hashing | `db/` or `common/` | `argon2`, `bcrypt` |
| Rate limiting | `backend/src/middleware.rs` | `actix-governor` |
| Database migrations | `db/` | `sqlx-cli` |
| Logging / tracing | All crates | `tracing`, `tracing-actix-web` |
| Redis caching | `db/` or new `cache/` crate | `redis` |
| Email service | New `mailer/` crate | `lettre` |
| Config management | `common/` | `config` |
| WS auth (JWT on connect) | `ws/src/main.rs` | reuse `jsonwebtoken` from `backend` |
| Room persistence | `ws/` + `db/` | Add room queries to `db/src/models/` |

---

## License

MIT
# Rust-workspaces-Template
# StanX-backend
