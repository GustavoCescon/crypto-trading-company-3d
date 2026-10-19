-- Intentional destructive reset requested by the owner.
-- Preserves schema, SQLx migration history, and company configuration/departments.
BEGIN;
LOCK TABLE wallet IN ACCESS EXCLUSIVE MODE;
ALTER TABLE audit_events DISABLE TRIGGER audit_events_immutable;
ALTER TABLE fills DISABLE TRIGGER fills_immutable;
ALTER TABLE decisions DISABLE TRIGGER decisions_immutable;
TRUNCATE TABLE audit_events,market_candles,agents,market_symbols,quotes,rankings,
    decisions,wallet,positions,intents,fills,equity_history,news,backtests,company_teams,
    company_runtime,fx_quotes RESTART IDENTITY CASCADE;
INSERT INTO wallet(id,initial_balance,cash,peak_equity,day_start_equity,limits)
VALUES (1,1000,1000,1000,1000,
    '{"initial_balance":"1000","order_budget":"300","max_asset_fraction":"0.15","max_total_fraction":"0.30","daily_loss_fraction":"0.03","max_drawdown_fraction":"0.1","stop_loss_fraction":"0.025","take_profit_fraction":"0.04","max_positions":5,"max_signal_age_seconds":120,"fee_bps":"10","spread_bps":"2","slippage_bps":"3"}'::jsonb);
INSERT INTO company_runtime(id,state,message)
VALUES (1,'STOPPED','Empresa reiniciada do zero com R$ 1.000 de capital fictício.');
ALTER TABLE audit_events ENABLE TRIGGER audit_events_immutable;
ALTER TABLE fills ENABLE TRIGGER fills_immutable;
ALTER TABLE decisions ENABLE TRIGGER decisions_immutable;
SELECT emit_activity('owner','database_reset',jsonb_build_object(
    'capital_brl',1000,'execution','simulated','historical_data_cleared',true,
    'configuration_preserved',true));
COMMIT;
