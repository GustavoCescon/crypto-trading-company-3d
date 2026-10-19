CREATE TABLE ai_runtime_state (
    id INTEGER PRIMARY KEY CHECK(id=1),
    model TEXT NOT NULL,
    memory_mb BIGINT NOT NULL DEFAULT 0 CHECK(memory_mb>=0),
    cpu_cores INTEGER NOT NULL DEFAULT 1 CHECK(cpu_cores>0),
    max_agents INTEGER NOT NULL DEFAULT 23 CHECK(max_agents BETWEEN 1 AND 30),
    scanner_symbols INTEGER NOT NULL DEFAULT 1 CHECK(scanner_symbols BETWEEN 1 AND 8),
    review_minutes INTEGER NOT NULL DEFAULT 30 CHECK(review_minutes BETWEEN 10 AND 60),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE strategy_policies (
    strategy TEXT PRIMARY KEY CHECK(strategy IN ('moving_average','rsi','macd','bollinger','momentum','breakout')),
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    weight NUMERIC(8,6) NOT NULL DEFAULT 1 CHECK(weight BETWEEN 0 AND 2),
    rationale TEXT NOT NULL DEFAULT 'Peso inicial',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO strategy_policies(strategy,weight) VALUES
 ('moving_average',0.30),('rsi',0.10),('macd',0.15),('bollinger',0.10),('momentum',0.25),('breakout',0.10);

ALTER TABLE company_teams ADD COLUMN manager_agent_id TEXT REFERENCES agents(id) ON DELETE SET NULL;
ALTER TABLE agents DROP CONSTRAINT agents_state_check;
ALTER TABLE agents ADD CONSTRAINT agents_state_check CHECK(state IN ('IDLE','ANALYZING','MEETING','APPROVING','EXECUTING','MONITORING','BLOCKED','ERROR','RETIRED'));
CREATE OR REPLACE FUNCTION enforce_agent_cap() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM pg_advisory_xact_lock(8457,1);
    IF TG_OP='INSERT' AND NOT EXISTS(SELECT 1 FROM agents WHERE id=NEW.id)
       AND NEW.state<>'RETIRED'
       AND (SELECT count(*) FROM agents WHERE state<>'RETIRED')>=
           (SELECT max_active_agents FROM company_settings WHERE id=1) THEN
        RAISE EXCEPTION 'Dynamic active-agent capacity reached';
    END IF;
    RETURN NEW;
END;
$$;

SELECT emit_activity('company','agent_autonomy_enabled',jsonb_build_object(
    'agent_council','ollama','hardware_adaptive','true','risk_limits','deterministic'));
