CREATE EXTENSION IF NOT EXISTS timescaledb;

-- Immutable source of truth; Redis is a delivery mechanism, not the audit ledger.
CREATE TABLE audit_events (
    sequence BIGSERIAL UNIQUE NOT NULL,
    event_id UUID PRIMARY KEY,
    occurred_at TIMESTAMPTZ NOT NULL,
    schema_version SMALLINT NOT NULL CHECK (schema_version = 1),
    envelope JSONB NOT NULL
);

CREATE FUNCTION deny_audit_mutation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'audit_events is append-only';
END;
$$;
CREATE TRIGGER audit_events_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON audit_events FOR EACH STATEMENT EXECUTE FUNCTION deny_audit_mutation();

CREATE TABLE event_outbox (
    event_id UUID PRIMARY KEY REFERENCES audit_events(event_id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    published_at TIMESTAMPTZ
);
CREATE INDEX pending_outbox ON event_outbox(created_at) WHERE published_at IS NULL;

-- Prices and amounts are decimal, never binary floating-point money.
CREATE TABLE market_candles (
    symbol TEXT NOT NULL,
    timeframe TEXT NOT NULL,
    open_time TIMESTAMPTZ NOT NULL,
    close_time TIMESTAMPTZ NOT NULL CHECK (close_time > open_time),
    open NUMERIC(38,18) NOT NULL CHECK (open > 0),
    high NUMERIC(38,18) NOT NULL,
    low NUMERIC(38,18) NOT NULL CHECK (low > 0),
    close NUMERIC(38,18) NOT NULL CHECK (close > 0),
    volume NUMERIC(38,18) NOT NULL CHECK (volume >= 0),
    source TEXT NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY(symbol, timeframe, open_time),
    CHECK (high >= greatest(open, close, low)),
    CHECK (low <= least(open, close))
);
SELECT create_hypertable('market_candles', 'open_time');
