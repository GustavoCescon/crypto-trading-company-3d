CREATE TABLE company_runtime (
    id INTEGER PRIMARY KEY CHECK(id=1),
    state TEXT NOT NULL CHECK(state IN ('STOPPED','STARTING','RUNNING','STOPPING','ERROR')),
    shutdown_policy TEXT NOT NULL DEFAULT 'CLOSE_SIMULATED' CHECK(shutdown_policy='CLOSE_SIMULATED'),
    message TEXT NOT NULL,
    requested_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- Preserve running legacy positions. Empty installations require an explicit start.
INSERT INTO company_runtime(id,state,message)
SELECT 1, CASE WHEN EXISTS(SELECT 1 FROM positions) THEN 'RUNNING' ELSE 'STOPPED' END,
       'Controle operacional inicializado; execução exclusivamente fictícia.';

CREATE FUNCTION guard_company_entry() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    -- Same first lock as risk, execution and lifecycle transitions.
    PERFORM id FROM wallet WHERE id=1 FOR UPDATE;
    IF NEW.side='BUY' AND NEW.status IN ('PENDING','APPROVED','FILLED')
       AND NOT EXISTS(SELECT 1 FROM company_runtime WHERE id=1 AND state='RUNNING') THEN
        RAISE EXCEPTION 'Empresa não está em operação; novas compras bloqueadas';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER company_entry_gate BEFORE INSERT OR UPDATE ON intents
FOR EACH ROW EXECUTE FUNCTION guard_company_entry();

SELECT emit_activity('company','lifecycle_initialized',jsonb_build_object('simulated',true));
