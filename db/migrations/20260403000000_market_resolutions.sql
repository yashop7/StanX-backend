-- Stores the oracle-computed outcome for a market after its deadline passes.
-- The market creator reads this and uses it to build the SetWinner tx in their wallet.
CREATE TABLE market_resolutions (
    market_id        INTEGER PRIMARY KEY REFERENCES markets(market_id),
    outcome          TEXT    NOT NULL,   -- 'OutcomeA' (YES wins) or 'OutcomeB' (NO wins)
    actual_value     BIGINT  NOT NULL,   -- e.g. actual view count at resolution time
    threshold        BIGINT  NOT NULL,   -- the target that was set
    metric           TEXT    NOT NULL,   -- 'viewCount', 'likeCount', 'commentCount'
    video_id         TEXT    NOT NULL,
    resolved_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
