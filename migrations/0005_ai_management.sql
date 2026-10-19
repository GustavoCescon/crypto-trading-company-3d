ALTER TABLE agents
    ADD COLUMN managed_by TEXT,
    ADD COLUMN team_id TEXT REFERENCES company_teams(id),
    ADD COLUMN performance_points NUMERIC(12,2) NOT NULL DEFAULT 0,
    ADD COLUMN penalty_points NUMERIC(12,2) NOT NULL DEFAULT 0;

ALTER TABLE company_settings
    ADD COLUMN managed_cash_reserve_fraction NUMERIC(8,6) NOT NULL DEFAULT 0.70
        CHECK(managed_cash_reserve_fraction BETWEEN 0.70 AND 0.95),
    ADD COLUMN managed_order_budget_brl NUMERIC(38,18) NOT NULL DEFAULT 300
        CHECK(managed_order_budget_brl >= 0);

CREATE TABLE management_reviews (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    model TEXT NOT NULL,
    summary TEXT NOT NULL,
    inputs JSONB NOT NULL,
    recommendations JSONB NOT NULL,
    applied JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX management_reviews_created_at ON management_reviews(created_at DESC);

CREATE TABLE management_penalties (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id TEXT NOT NULL REFERENCES agents(id),
    fill_id UUID NOT NULL UNIQUE REFERENCES fills(id),
    points NUMERIC(12,2) NOT NULL CHECK(points > 0),
    loss_brl NUMERIC(38,18) NOT NULL CHECK(loss_brl < 0),
    reason TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE management_scored_fills (
    fill_id UUID PRIMARY KEY REFERENCES fills(id),
    agent_id TEXT NOT NULL REFERENCES agents(id),
    outcome TEXT NOT NULL CHECK(outcome IN ('WIN','LOSS','BREAKEVEN')),
    points NUMERIC(12,2) NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

SELECT emit_activity('company','ai_management_installed',jsonb_build_object(
    'cash_reserve_fraction',0.70,'real_trading_enabled',false));
