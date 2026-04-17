-- =============================================================================
-- ENUM TYPES
-- =============================================================================
CREATE TYPE order_side AS ENUM ('Buy', 'Sell');
CREATE TYPE token_type AS ENUM ('Yes', 'No');
CREATE TYPE winning_outcome AS ENUM ('OutcomeA', 'OutcomeB', 'Neither');
CREATE TYPE order_status AS ENUM ('open', 'partially_filled', 'filled', 'cancelled');
CREATE TYPE market_status AS ENUM ('active', 'settled', 'closed');

-- =============================================================================
-- 1. MARKETS — materialized state of each market
-- =============================================================================
CREATE TABLE markets (
    market_id       INTEGER PRIMARY KEY,
    authority       VARCHAR(64) NOT NULL,
    settlement_deadline BIGINT NOT NULL,
    collateral_mint VARCHAR(64) NOT NULL,
    outcome_yes_mint VARCHAR(64) NOT NULL,
    outcome_no_mint VARCHAR(64) NOT NULL,
    meta_data_url   TEXT NOT NULL DEFAULT '',
    status          market_status NOT NULL DEFAULT 'active',
    winning_outcome winning_outcome,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- =============================================================================
-- 2. LIVE_ORDERS — materialized orderbook state (the one you query for orderbook)
-- =============================================================================
CREATE TABLE live_orders (
    order_id        BIGINT NOT NULL,
    market_id       INTEGER NOT NULL REFERENCES markets(market_id),
    user_pubkey     VARCHAR(64) NOT NULL,
    side            order_side NOT NULL,
    token_type      token_type NOT NULL,
    price           BIGINT NOT NULL,
    original_quantity BIGINT NOT NULL,
    remaining_quantity BIGINT NOT NULL,
    status          order_status NOT NULL DEFAULT 'open',
    placed_at       BIGINT NOT NULL,  -- on-chain timestamp
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (market_id, order_id)
);

-- Fast orderbook lookups: all open orders for a market, sorted by price
CREATE INDEX idx_live_orders_book ON live_orders (market_id, token_type, side, price)
    WHERE status IN ('open', 'partially_filled');
CREATE INDEX idx_live_orders_user ON live_orders (user_pubkey, market_id);

-- =============================================================================
-- 4. TRADE HISTORY — denormalized for fast queries (filled from OrderMatched)
-- =============================================================================
CREATE TABLE trades (
    id              SERIAL PRIMARY KEY,
    signature       VARCHAR(128) NOT NULL,
    market_id       INTEGER NOT NULL,
    maker_order_id  BIGINT NOT NULL,
    taker_side      order_side NOT NULL,
    taker           VARCHAR(64) NOT NULL,
    maker           VARCHAR(64) NOT NULL,
    token_type      token_type NOT NULL,
    price           BIGINT NOT NULL,
    quantity        BIGINT NOT NULL,
    event_timestamp BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_trades_market ON trades (market_id, event_timestamp DESC);
CREATE INDEX idx_trades_user ON trades (taker, event_timestamp DESC);
CREATE INDEX idx_trades_maker ON trades (maker, event_timestamp DESC);

-- =============================================================================
-- 5. INDEXER CURSOR — tracks last processed slot/signature for crash recovery
-- =============================================================================
CREATE TABLE indexer_cursor (
    id              INTEGER PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    last_signature  VARCHAR(128) NOT NULL,
    last_slot       BIGINT NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);


-- =============================================================================
-- 3. EVENT LOG TABLES — immutable audit trail of every blockchain event
-- =============================================================================

-- 3a. Market lifecycle events
CREATE TABLE event_market_initialized (
    id              SERIAL PRIMARY KEY,
    signature       VARCHAR(128) NOT NULL,
    slot            BIGINT NOT NULL,
    market_id       INTEGER NOT NULL,
    authority       VARCHAR(64) NOT NULL,
    settlement_deadline BIGINT NOT NULL,
    collateral_mint VARCHAR(64) NOT NULL,
    outcome_yes_mint VARCHAR(64) NOT NULL,
    outcome_no_mint VARCHAR(64) NOT NULL,
    meta_data_url   TEXT NOT NULL DEFAULT '',
    event_timestamp BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(signature, market_id)
);

CREATE TABLE event_market_closed (
    id              SERIAL PRIMARY KEY,
    signature       VARCHAR(128) NOT NULL,
    slot            BIGINT NOT NULL,
    market_id       INTEGER NOT NULL,
    authority       VARCHAR(64) NOT NULL,
    event_timestamp BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(signature, market_id)
);

CREATE TABLE event_winning_side_set (
    id              SERIAL PRIMARY KEY,
    signature       VARCHAR(128) NOT NULL,
    slot            BIGINT NOT NULL,
    market_id       INTEGER NOT NULL,
    winning_outcome winning_outcome NOT NULL,
    authority       VARCHAR(64) NOT NULL,
    event_timestamp BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(signature, market_id)
);

CREATE TABLE event_metadata_updated (
    id              SERIAL PRIMARY KEY,
    signature       VARCHAR(128) NOT NULL,
    slot            BIGINT NOT NULL,
    market_id       INTEGER NOT NULL,
    authority       VARCHAR(64) NOT NULL,
    new_metadata_url TEXT NOT NULL,
    event_timestamp BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(signature, market_id)
);

-- 3b. Order events
CREATE TABLE event_order_placed (
    id              SERIAL PRIMARY KEY,
    signature       VARCHAR(128) NOT NULL,
    slot            BIGINT NOT NULL,
    market_id       INTEGER NOT NULL,
    order_id        BIGINT NOT NULL,
    user_pubkey     VARCHAR(64) NOT NULL,
    side            order_side NOT NULL,
    token_type      token_type NOT NULL,
    price           BIGINT NOT NULL,
    quantity        BIGINT NOT NULL,
    event_timestamp BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(signature, order_id)
);

CREATE TABLE event_order_matched (
    id              SERIAL PRIMARY KEY,
    signature       VARCHAR(128) NOT NULL,
    slot            BIGINT NOT NULL,
    market_id       INTEGER NOT NULL,
    maker_order_id  BIGINT NOT NULL,
    taker_side      order_side NOT NULL,
    taker           VARCHAR(64) NOT NULL,
    maker           VARCHAR(64) NOT NULL,
    token_type      token_type NOT NULL,
    price           BIGINT NOT NULL,
    quantity        BIGINT NOT NULL,
    event_timestamp BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(signature, maker_order_id)
);

CREATE INDEX idx_event_order_matched_price
ON event_order_matched (market_id, token_type, event_timestamp ASC);


CREATE TABLE event_order_cancelled (
    id              SERIAL PRIMARY KEY,
    signature       VARCHAR(128) NOT NULL,
    slot            BIGINT NOT NULL,
    market_id       INTEGER NOT NULL,
    order_id        BIGINT NOT NULL,
    user_pubkey     VARCHAR(64) NOT NULL,
    side            order_side NOT NULL,
    token_type      token_type NOT NULL,
    remaining_quantity BIGINT NOT NULL,
    event_timestamp BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(signature, order_id)
);

CREATE TABLE event_market_order_executed (
    id              SERIAL PRIMARY KEY,
    signature       VARCHAR(128) NOT NULL,
    slot            BIGINT NOT NULL,
    market_id       INTEGER NOT NULL,
    user_pubkey     VARCHAR(64) NOT NULL,
    side            order_side NOT NULL,
    token_type      token_type NOT NULL,
    initial_quantity BIGINT NOT NULL,
    filled_quantity  BIGINT NOT NULL,
    orders_matched  BIGINT NOT NULL,
    event_timestamp BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(signature)
);

-- 3c. Token events
CREATE TABLE event_tokens_split (
    id              SERIAL PRIMARY KEY,
    signature       VARCHAR(128) NOT NULL,
    slot            BIGINT NOT NULL,
    market_id       INTEGER NOT NULL,
    user_pubkey     VARCHAR(64) NOT NULL,
    amount          BIGINT NOT NULL,
    event_timestamp BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(signature)
);

CREATE TABLE event_tokens_merged (
    id              SERIAL PRIMARY KEY,
    signature       VARCHAR(128) NOT NULL,
    slot            BIGINT NOT NULL,
    market_id       INTEGER NOT NULL,
    user_pubkey     VARCHAR(64) NOT NULL,
    amount          BIGINT NOT NULL,
    event_timestamp BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(signature)
);

-- 3d. Claim events
CREATE TABLE event_rewards_claimed (
    id              SERIAL PRIMARY KEY,
    signature       VARCHAR(128) NOT NULL,
    slot            BIGINT NOT NULL,
    market_id       INTEGER NOT NULL,
    user_pubkey     VARCHAR(64) NOT NULL,
    collateral_amount BIGINT NOT NULL,
    yes_tokens_burned BIGINT NOT NULL,
    no_tokens_burned  BIGINT NOT NULL,
    event_timestamp BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(signature)
);

CREATE TABLE event_funds_claimed (
    id              SERIAL PRIMARY KEY,
    signature       VARCHAR(128) NOT NULL,
    slot            BIGINT NOT NULL,
    market_id       INTEGER NOT NULL,
    user_pubkey     VARCHAR(64) NOT NULL,
    collateral_amount BIGINT NOT NULL,
    yes_amount      BIGINT NOT NULL,
    no_amount       BIGINT NOT NULL,
    event_timestamp BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(signature)
);
