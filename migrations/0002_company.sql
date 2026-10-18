CREATE TABLE agents (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, role TEXT NOT NULL, room TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'IDLE' CHECK(state IN ('IDLE','ANALYZING','MEETING','APPROVING','EXECUTING','MONITORING','BLOCKED','ERROR')),
    detail JSONB NOT NULL DEFAULT '{}', heartbeat TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(), tasks BIGINT NOT NULL DEFAULT 0
);
CREATE TABLE market_symbols (
    symbol TEXT PRIMARY KEY, base_asset TEXT NOT NULL, eligible BOOLEAN NOT NULL,
    filters JSONB NOT NULL, updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE quotes (
    symbol TEXT PRIMARY KEY REFERENCES market_symbols(symbol), price NUMERIC(38,18) NOT NULL CHECK(price>0),
    bid NUMERIC(38,18) NOT NULL, ask NUMERIC(38,18) NOT NULL,
    observed_at TIMESTAMPTZ NOT NULL, source TEXT NOT NULL,
    CHECK(bid>0 AND ask>=bid)
);
CREATE TABLE rankings (
    symbol TEXT PRIMARY KEY REFERENCES market_symbols(symbol), score DOUBLE PRECISION NOT NULL,
    data JSONB NOT NULL, updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE decisions (
    id UUID PRIMARY KEY, symbol TEXT NOT NULL, candle_time TIMESTAMPTZ NOT NULL,
    action TEXT NOT NULL CHECK(action IN ('BUY','SELL','HOLD')), score DOUBLE PRECISION NOT NULL,
    rules_version TEXT NOT NULL, detail JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(symbol,candle_time,rules_version)
);
CREATE TABLE wallet (
    id INTEGER PRIMARY KEY CHECK(id=1), initial_balance NUMERIC(38,18) NOT NULL,
    cash NUMERIC(38,18) NOT NULL CHECK(cash>=0), realized_pnl NUMERIC(38,18) NOT NULL DEFAULT 0,
    fees NUMERIC(38,18) NOT NULL DEFAULT 0, peak_equity NUMERIC(38,18) NOT NULL,
    day_start_equity NUMERIC(38,18) NOT NULL, trading_day DATE NOT NULL DEFAULT CURRENT_DATE,
    limits JSONB NOT NULL, updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE positions (
    symbol TEXT PRIMARY KEY, quantity NUMERIC(38,18) NOT NULL CHECK(quantity>0),
    cost NUMERIC(38,18) NOT NULL CHECK(cost>0), opened_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    strategy TEXT NOT NULL
);
CREATE TABLE intents (
    id UUID PRIMARY KEY, decision_id UUID REFERENCES decisions(id), symbol TEXT NOT NULL,
    side TEXT NOT NULL CHECK(side IN ('BUY','SELL')), status TEXT NOT NULL DEFAULT 'PENDING'
    CHECK(status IN ('PENDING','APPROVED','REJECTED','FILLED','EXPIRED')),
    reason TEXT NOT NULL, risk_reason TEXT, quantity NUMERIC(38,18), reserved NUMERIC(38,18) NOT NULL DEFAULT 0 CHECK(reserved>=0),
    approved_at TIMESTAMPTZ, created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL, risk_snapshot JSONB,
    UNIQUE(decision_id), CHECK(status NOT IN ('APPROVED','FILLED') OR (approved_at IS NOT NULL AND quantity>0 AND risk_snapshot IS NOT NULL))
);
CREATE UNIQUE INDEX one_active_intent_per_asset ON intents(symbol) WHERE status IN ('PENDING','APPROVED');
CREATE INDEX pending_intents ON intents(created_at) WHERE status IN ('PENDING','APPROVED');
CREATE TABLE fills (
    id UUID PRIMARY KEY, intent_id UUID UNIQUE NOT NULL REFERENCES intents(id), symbol TEXT NOT NULL,
    side TEXT NOT NULL, quantity NUMERIC(38,18) NOT NULL CHECK(quantity>0),
    price NUMERIC(38,18) NOT NULL CHECK(price>0), reference_price NUMERIC(38,18) NOT NULL,
    fee NUMERIC(38,18) NOT NULL CHECK(fee>=0), slippage_cost NUMERIC(38,18) NOT NULL,
    realized_pnl NUMERIC(38,18) NOT NULL, strategy TEXT NOT NULL,
    simulated BOOLEAN NOT NULL DEFAULT true CHECK(simulated), created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE FUNCTION guard_fill() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM intents WHERE id=NEW.intent_id AND status='APPROVED'
       AND symbol=NEW.symbol AND side=NEW.side AND quantity=NEW.quantity AND expires_at>now()) THEN
        RAISE EXCEPTION 'Fill requires matching valid risk approval';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER require_risk_approval BEFORE INSERT ON fills FOR EACH ROW EXECUTE FUNCTION guard_fill();
CREATE TRIGGER fills_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON fills FOR EACH STATEMENT EXECUTE FUNCTION deny_audit_mutation();
CREATE TRIGGER decisions_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON decisions FOR EACH STATEMENT EXECUTE FUNCTION deny_audit_mutation();
CREATE TABLE equity_history (
    observed_at TIMESTAMPTZ NOT NULL PRIMARY KEY, equity NUMERIC(38,18) NOT NULL,
    cash NUMERIC(38,18) NOT NULL, exposure NUMERIC(38,18) NOT NULL
);
SELECT create_hypertable('equity_history','observed_at');
CREATE TABLE news (
    id TEXT PRIMARY KEY, title TEXT NOT NULL, url TEXT NOT NULL, source TEXT NOT NULL,
    published_at TIMESTAMPTZ, collected_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    assets JSONB NOT NULL, sentiment DOUBLE PRECISION NOT NULL CHECK(sentiment BETWEEN -1 AND 1),
    analysis JSONB NOT NULL
);
CREATE TABLE backtests (
    id UUID PRIMARY KEY, symbol TEXT NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    report JSONB NOT NULL, dataset_hash TEXT NOT NULL
);
CREATE FUNCTION emit_activity(producer TEXT, kind TEXT, data JSONB) RETURNS UUID LANGUAGE plpgsql AS $$
DECLARE eid UUID := gen_random_uuid(); stamp TIMESTAMPTZ := clock_timestamp();
BEGIN
    INSERT INTO audit_events(event_id,occurred_at,schema_version,envelope)
    VALUES(eid,stamp,1,jsonb_build_object('schema_version',1,'event_id',eid,'correlation_id',COALESCE(data->>'decision_id',data->>'intent_id',eid::text),
        'occurred_at',stamp,'producer',producer,'payload',jsonb_build_object('type','activity','kind',kind,'data',data)));
    INSERT INTO event_outbox(event_id) VALUES(eid);
    RETURN eid;
END;
$$;
