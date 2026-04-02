-- event_market_order_executed was created before filled_quantity was added
-- to the schema. This migration adds the missing column to the live table.
ALTER TABLE event_market_order_executed
    ADD COLUMN IF NOT EXISTS filled_quantity BIGINT NOT NULL DEFAULT 0;
