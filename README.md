# Stanx Backend

High-performance backend for a Solana-based prediction market built in Rust because speed isn't optional.

Real-time orderbooks, concurrent WebSocket connections, on-chain event indexing, all running at the latency Solana was designed for.

---

## Why Rust + Solana

Prediction markets live and die by speed. A stale orderbook or a slow fill is a broken product. Rust gives zero-cost abstractions and fearless concurrency. Solana gives sub-second finality and ~400ms block times. Everything here is built around that constraint.

---

## Architecture

```
Solana Program (on-chain)
        │
        │  WebSocket RPC logs
        ▼
  event-listener          ← subscribes to program events, decodes Anchor/Borsh
        │
        ├──► PostgreSQL   ← persistent storage (SQLx, compile-time verified queries)
        │
        └──► Redis        ← pub/sub on `orderbook:market:{id}` channels
                │
        ┌───────┴────────┐
        ▼                ▼
   backend (Axum)     ws (Actix-ws)
   REST API           WebSocket server
   port 3000          real-time orderbook diffs
```

**Data flow:** On-chain event → event-listener decodes → stored in Postgres + diff published to Redis → backend serves REST queries from in-memory cache + Postgres → WebSocket server pushes live diffs to subscribed clients.

---

## Workspace

| Crate | Role |
|---|---|
| `backend` | Axum HTTP server — markets, orderbooks, trades, user orders |
| `ws` | Actix-ws WebSocket server — room-based pub/sub for live updates |
| `db` | SQLx database layer — all Postgres queries and models |
| `common` | Shared types — `OrderbookState`, `OrderbookDiff`, enums |
| `event-listener` | Solana event indexer — decodes on-chain logs, syncs state |

---

## API Endpoints

```
POST   /signup
POST   /signin

GET    /markets
GET    /markets/:market_id
GET    /markets/:market_id/orderbook
GET    /markets/:market_id/trades
GET    /markets/:market_id/orders/:user_pubkey
GET    /user/:user_pubkey/trades
```

---

## Orderbook State

The core data structure is `OrderbookState` — an in-memory snapshot of all four order sides (`yes_bids`, `yes_asks`, `no_bids`, `no_asks`), kept sorted by price, updated via `OrderbookDiff` deltas.

`OrderbookDiff` tracks only what changed (added/removed orders per side) — this is what gets published to Redis and pushed to WebSocket clients. No full snapshots on every update.

---

## Events Indexed

`MarketInitialized` · `OrderPlaced` · `OrderMatched` · `OrderCancelled` · `MarketOrderExecuted` · `TokensSplit` · `TokensMerged` · `WinningSideSet` · `RewardsClaimed` · `MarketClosed` · `FundsClaimed`

---

## Setup

**Prerequisites:** Rust (latest stable), PostgreSQL, Redis, Solana devnet RPC access

```bash
git clone <repo>
cd stanx-backend
cp .env.example .env
```

**.env**
```env
DATABASE_URL=postgres://user:password@localhost/stanx
PROGRAM_ID=<your_solana_program_id>
SOLANA_WS_RPC_URL=wss://api.devnet.solana.com/
REDIS_ADDRESS=127.0.0.1
REDIS_PORT=6379
SECRET_KEY=your_jwt_secret
```

**Run migrations**
```bash
cargo install sqlx-cli
sqlx migrate run --source db/migrations
```

**Start all services**
```bash
# Index on-chain events
cargo run -p event-listener

# REST API
cargo run -p backend

# WebSocket server
cargo run -p ws
```

---

## Tech Stack

| | |
|---|---|
| **Axum** | async HTTP server |
| **Actix-ws** | WebSocket connections |
| **Tokio** | async runtime |
| **SQLx** | compile-time verified Postgres queries |
| **Anchor-lang + Borsh** | Solana event deserialization |
| **Solana-client** | RPC + WebSocket subscriptions |
| **Redis** | orderbook diff pub/sub |
| **Snowflake IDs** | crash-safe unique client identifiers |
| **jsonwebtoken** | JWT auth |
