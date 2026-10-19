CREATE TABLE fx_quotes (
    pair TEXT PRIMARY KEY CHECK(pair='USDTBRL'),
    price NUMERIC(38,18) NOT NULL CHECK(price>0),
    observed_at TIMESTAMPTZ NOT NULL,
    source TEXT NOT NULL
);

ALTER TABLE fills ADD COLUMN account_currency TEXT NOT NULL DEFAULT 'USDT' CHECK(account_currency IN ('BRL','USDT'));
ALTER TABLE fills ADD COLUMN quote_currency TEXT NOT NULL DEFAULT 'USDT' CHECK(quote_currency='USDT');
ALTER TABLE fills ADD COLUMN fx_rate NUMERIC(38,18) NOT NULL DEFAULT 1 CHECK(fx_rate>0);

CREATE TABLE company_settings (
    id INTEGER PRIMARY KEY CHECK(id=1),
    base_currency TEXT NOT NULL CHECK(base_currency='BRL'),
    initial_capital NUMERIC(38,18) NOT NULL CHECK(initial_capital=1000),
    max_active_agents INTEGER NOT NULL CHECK(max_active_agents BETWEEN 1 AND 30),
    max_active_teams INTEGER NOT NULL CHECK(max_active_teams BETWEEN 1 AND 8),
    hires_per_management_cycle INTEGER NOT NULL CHECK(hires_per_management_cycle BETWEEN 1 AND 3),
    management_review_minutes INTEGER NOT NULL CHECK(management_review_minutes=60),
    max_positions INTEGER NOT NULL CHECK(max_positions BETWEEN 1 AND 5),
    max_total_exposure_fraction NUMERIC(8,6) NOT NULL CHECK(max_total_exposure_fraction BETWEEN 0 AND 0.30),
    real_trading_enabled BOOLEAN NOT NULL DEFAULT FALSE CHECK(real_trading_enabled=FALSE),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO company_settings(id,base_currency,initial_capital,max_active_agents,max_active_teams,
    hires_per_management_cycle,management_review_minutes,max_positions,max_total_exposure_fraction)
VALUES(1,'BRL',1000,30,8,3,60,5,0.30);

CREATE FUNCTION enforce_agent_cap() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM pg_advisory_xact_lock(8457,1);
    IF TG_OP='INSERT' AND NOT EXISTS(SELECT 1 FROM agents WHERE id=NEW.id)
       AND (SELECT count(*) FROM agents)>= (SELECT max_active_agents FROM company_settings WHERE id=1) THEN
        RAISE EXCEPTION 'Limite configurado de 30 agentes atingido';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER company_agent_limit BEFORE INSERT ON agents FOR EACH ROW EXECUTE FUNCTION enforce_agent_cap();

CREATE TABLE company_departments (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO company_departments(id,name) VALUES
 ('executive','Diretoria Executiva'),('market','Inteligência de Mercado'),
 ('news','Investigação e Notícias'),('technical','Análise Técnica'),
 ('quant','Pesquisa Quantitativa'),('committee','Comitê de Trading'),
 ('risk','Gestão de Risco'),('execution','Execução'),('audit','Auditoria e Performance');
CREATE TABLE company_teams (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    department_id TEXT NOT NULL REFERENCES company_departments(id),
    state TEXT NOT NULL DEFAULT 'ACTIVE' CHECK(state IN ('ACTIVE','PAUSED','RETIRED')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE FUNCTION enforce_team_cap() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM pg_advisory_xact_lock(8457,2);
    IF NEW.state='ACTIVE' AND (TG_OP='INSERT' OR OLD.state<>'ACTIVE')
       AND (SELECT count(*) FROM company_teams WHERE state='ACTIVE') >=
           (SELECT max_active_teams FROM company_settings WHERE id=1) THEN
        RAISE EXCEPTION 'Limite configurado de 8 equipes ativas atingido';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER company_team_limit BEFORE INSERT OR UPDATE OF state ON company_teams
FOR EACH ROW EXECUTE FUNCTION enforce_team_cap();

SELECT emit_activity('company','brl_configuration_created',jsonb_build_object(
    'base_currency','BRL','initial_capital',1000,'max_active_agents',30,
    'max_active_teams',8,'hires_per_management_cycle',3,'management_review_minutes',60,
    'max_positions',5,'max_total_exposure_fraction',0.30,'real_trading_enabled',false));
