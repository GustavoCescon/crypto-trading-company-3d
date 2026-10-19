import json
import os
import re
import time
import uuid
import unicodedata

import httpx
import psycopg
from psycopg.types.json import Jsonb

DATABASE_URL = os.environ["DATABASE_URL"]
OLLAMA_URL = os.getenv("OLLAMA_URL", "http://ollama:11434").rstrip("/")
MODEL = "llama3.2:1b"
MODEL_TIMEOUT = max(300, int(os.getenv("OLLAMA_TIMEOUT", "900")))
MODEL_CONTEXT = 2048
DEPARTMENTS = {"executive", "market", "news", "technical", "quant", "committee", "risk", "execution", "audit"}
STRATEGIES = {"moving_average", "rsi", "macd", "bollinger", "momentum", "breakout"}


def hardware_profile(client):
    """Select a locally installed model that fits the host's visible RAM/CPU budget."""
    memory_mb = 0
    try:
        with open("/proc/meminfo", encoding="ascii") as host_memory:
            for line in host_memory:
                if line.startswith("MemAvailable:"):
                    memory_mb = int(line.split()[1]) // 1024
                    break
    except (OSError, ValueError):
        pass
    cores = max(1, os.cpu_count() or 1)
    try:
        with open("/sys/fs/cgroup/memory.max", encoding="ascii") as limit_file:
            limit = limit_file.read().strip()
            if limit.isdigit():
                memory_mb = min(memory_mb or int(limit) // (1024 * 1024), int(limit) // (1024 * 1024))
    except (OSError, ValueError):
        pass
    try:
        with open("/sys/fs/cgroup/cpu.max", encoding="ascii") as quota_file:
            quota, period = quota_file.read().split()
            if quota.isdigit() and int(period) > 0:
                cores = max(1, min(cores, int(quota) // int(period)))
    except (OSError, ValueError):
        pass
    # Keep at least half the memory for the OS, database and trading services.
    max_parameters = max(1, int(memory_mb / 1800)) if memory_mb else 1
    candidates = [("llama3.2:1b", 1), ("qwen2.5:3b", 3), ("qwen2.5:7b", 7), ("qwen2.5:14b", 14)]
    preferred = max((item for item in candidates if item[1] <= max_parameters), key=lambda item: item[1])[0]
    response = client.get(OLLAMA_URL + "/api/tags", timeout=10)
    response.raise_for_status()
    installed = [item.get("name", "") for item in response.json().get("models", [])]
    model = next((name for name in installed if name == preferred or name.startswith(preferred + ":")), None)
    if not model:
        model = preferred
        log("model_pull_started", model=model, memory_mb=memory_mb, cpu_cores=cores)
        response = client.post(OLLAMA_URL + "/api/pull", json={"name": model, "stream": False}, timeout=3600)
        response.raise_for_status()
        log("model_pull_completed", model=model)
    capacity = min(30, max(3, min(cores * 2, max(3, memory_mb // 700))))
    review_minutes = 2
    return model, memory_mb, cores, capacity, review_minutes


def log(event, **data):
    print(json.dumps({"service": "management", "event": event, **data}), flush=True)


def slug(value):
    value = unicodedata.normalize("NFKD", value).encode("ascii", "ignore").decode().lower()
    return re.sub(r"[^a-z0-9_]+", "_", value).strip("_")[:36]


def connect():
    while True:
        try:
            return psycopg.connect(DATABASE_URL, autocommit=True)
        except psycopg.Error as exc:
            log("database_wait", error=type(exc).__name__)
            time.sleep(3)


def register_director(conn, state, detail):
    conn.execute("""INSERT INTO agents(id,name,role,room,state,detail,managed_by)
        VALUES('director','Dara','Diretora executiva de IA','executive',%s,%s,'ollama')
        ON CONFLICT(id) DO UPDATE SET state=EXCLUDED.state,detail=EXCLUDED.detail,
        heartbeat=now(),updated_at=now()""", (state, Jsonb(detail)))


def score_new_fills(conn):
    rows = conn.execute("""SELECT f.id,f.strategy,f.realized_pnl
        FROM fills f LEFT JOIN management_scored_fills s ON s.fill_id=f.id
        WHERE f.side='SELL' AND s.fill_id IS NULL ORDER BY f.created_at LIMIT 100""").fetchall()
    scored = 0
    for fill_id, strategy, pnl in rows:
        if not conn.execute("SELECT 1 FROM agents WHERE id=%s", (strategy,)).fetchone():
            continue
        outcome = "WIN" if pnl > 0 else "LOSS" if pnl < 0 else "BREAKEVEN"
        points = min(5.0, max(1.0, float(abs(pnl)) / 10.0)) if pnl < 0 else (1.0 if pnl > 0 else 0.0)
        inserted = conn.execute("""INSERT INTO management_scored_fills(fill_id,agent_id,outcome,points)
            VALUES(%s,%s,%s,%s) ON CONFLICT(fill_id) DO NOTHING RETURNING fill_id""",
            (fill_id, strategy, outcome, points)).fetchone()
        if not inserted:
            continue
        if pnl < 0:
            conn.execute("INSERT INTO management_penalties(agent_id,fill_id,points,loss_brl,reason) VALUES(%s,%s,%s,%s,%s) ON CONFLICT(fill_id) DO NOTHING",
                         (strategy, fill_id, points, pnl, "Operação encerrada com prejuízo líquido após taxas"))
            conn.execute("UPDATE agents SET penalty_points=penalty_points+%s,updated_at=now() WHERE id=%s", (points, strategy))
            conn.execute("SELECT emit_activity('director','decision_penalty',%s)",
                         (Jsonb({"agent_id": strategy, "fill_id": str(fill_id), "points": points, "loss_brl": str(pnl)}),))
        elif pnl > 0:
            conn.execute("UPDATE agents SET performance_points=performance_points+%s,updated_at=now() WHERE id=%s", (points, strategy))
        scored += 1
    return scored


def bootstrap_teams(conn):
    if conn.execute("SELECT EXISTS(SELECT 1 FROM company_teams)").fetchone()[0]:
        return []
    maximum = conn.execute("SELECT max_active_teams FROM company_settings WHERE id=1").fetchone()[0]
    defaults = [
        ("market_intelligence", "Inteligência de Mercado", "market", ("market", "news")),
        ("research", "Pesquisa e Estratégias", "technical", ("technical", "quant")),
        ("trade_governance", "Comitê e Gestão de Risco", "committee", ("committee", "risk", "execution", "audit", "executive")),
    ]
    created = []
    for ident, name, department, rooms in defaults[:maximum]:
        conn.execute("INSERT INTO company_teams(id,name,department_id,state) VALUES(%s,%s,%s,'ACTIVE')", (ident, name, department))
        conn.execute("UPDATE agents SET team_id=%s WHERE team_id IS NULL AND room=ANY(%s)", (ident, list(rooms)))
        created.append(ident)
    conn.execute("SELECT emit_activity('director','organization_initialized',%s)",
                 (Jsonb({"created_teams": created}),))
    return created


def context(conn):
    settings = conn.execute("SELECT to_jsonb(s)-'id' FROM company_settings s WHERE id=1").fetchone()[0]
    wallet = conn.execute("""SELECT jsonb_build_object('cash_brl',w.cash::text,
        'equity_brl',(w.cash+COALESCE((SELECT sum(p.quantity*q.price*f.price) FROM positions p
        JOIN quotes q USING(symbol) JOIN fx_quotes f ON f.pair='USDTBRL'),0))::text,
        'realized_pnl_brl',w.realized_pnl::text,'fees_brl',w.fees::text,
        'open_positions',(SELECT count(*) FROM positions)) FROM wallet w WHERE id=1""").fetchone()[0]
    agents = [dict(zip(("id", "name", "role", "room", "state", "team_id", "managed_by", "performance_points", "penalty_points"), row))
              for row in conn.execute("""SELECT id,name,role,room,state,team_id,managed_by,performance_points::text,penalty_points::text
              FROM agents WHERE state<>'RETIRED' ORDER BY penalty_points DESC,room,id LIMIT 30""").fetchall()]
    staffing = [dict(zip(("room", "agents"), row)) for row in conn.execute(
        "SELECT room,count(*) FROM agents GROUP BY room ORDER BY room").fetchall()]
    teams = [dict(zip(("id", "name", "department_id", "state", "manager_agent_id", "manager_name"), row))
             for row in conn.execute("""SELECT t.id,t.name,t.department_id,t.state,t.manager_agent_id,a.name
             FROM company_teams t LEFT JOIN agents a ON a.id=t.manager_agent_id ORDER BY t.state,t.name""").fetchall()]
    strategy_policies = [dict(zip(("strategy", "enabled", "weight", "rationale"), row))
                         for row in conn.execute("SELECT strategy,enabled,weight::text,rationale FROM strategy_policies ORDER BY strategy").fetchall()]
    outcomes = [dict(zip(("strategy", "closed", "wins", "realized_pnl_brl"), row))
                for row in conn.execute("""SELECT strategy,count(*) FILTER(WHERE side='SELL') AS closed,
        count(*) FILTER(WHERE side='SELL' AND realized_pnl>0) AS wins,
        COALESCE(sum(realized_pnl) FILTER(WHERE side='SELL'),0)::text AS realized_pnl_brl
        FROM fills GROUP BY strategy ORDER BY sum(realized_pnl) FILTER(WHERE side='SELL') NULLS LAST""").fetchall()]
    return {"simulation_only": True, "real_trading_enabled": False,
            "settings": settings, "wallet": wallet, "agents": agents, "staffing_by_room": staffing,
            "teams": teams, "strategy_policies": strategy_policies, "strategy_outcomes": outcomes}


def call_model(client, data):
    schema = {"type": "object", "properties": {
        "summary": {"type": "string", "maxLength": 180},
        "reserve_fraction": {"type": "number", "minimum": 0.70, "maximum": 0.95},
        "order_budget_brl": {"type": "number", "minimum": 0, "maximum": 300},
        "create_teams": {"type": "array", "items": {"type": "object", "properties": {
            "name": {"type": "string"}, "department_id": {"type": "string", "enum": sorted(DEPARTMENTS)}},
            "required": ["name", "department_id"], "additionalProperties": False}},
        "create_agents": {"type": "array", "items": {"type": "object", "properties": {
            "name": {"type": "string"}, "role": {"type": "string"},
            "room": {"type": "string", "enum": sorted(DEPARTMENTS)}, "team_name": {"type": "string"}},
            "required": ["name", "role", "room", "team_name"], "additionalProperties": False}},
        "assignments": {"type": "array", "items": {"type": "object", "properties": {
            "agent_id": {"type": "string"}, "team_name": {"type": "string"},
            "room": {"type": "string", "enum": sorted(DEPARTMENTS)}},
            "required": ["agent_id", "team_name", "room"], "additionalProperties": False}},
        "fire_agents": {"type": "array", "items": {"type": "string"}},
        "team_managers": {"type": "array", "items": {"type": "object", "properties": {
            "team_id": {"type": "string"}, "manager_agent_id": {"type": "string"}},
            "required": ["team_id", "manager_agent_id"], "additionalProperties": False}},
        "team_states": {"type": "array", "items": {"type": "object", "properties": {
            "team_id": {"type": "string"}, "state": {"type": "string", "enum": ["ACTIVE", "PAUSED", "RETIRED"]}},
            "required": ["team_id", "state"], "additionalProperties": False}},
        "strategy_weights": {"type": "array", "items": {"type": "object", "properties": {
            "strategy": {"type": "string", "enum": sorted(STRATEGIES)},
            "weight": {"type": "number", "minimum": 0, "maximum": 2},
            "enabled": {"type": "boolean"}, "rationale": {"type": "string", "maxLength": 180}},
            "required": ["strategy", "weight", "enabled", "rationale"], "additionalProperties": False}},
        "focus": {"type": "string", "maxLength": 180}},
        "required": ["summary", "reserve_fraction", "order_budget_brl", "focus"],
        "additionalProperties": False}
    system = "Voce e a diretora de uma empresa de trading exclusivamente simulado. Responda em portugues brasileiro. Avalie caixa, resultados liquidos por estrategia e desempenho de cada agente. Pode contratar, aposentar somente agentes com managed_by=director, designar gerentes, pausar ou reativar equipes e ajustar pesos/ativacao das estrategias. Se os resultados mostrarem uma estrategia ou equipe persistentemente ruim, proponha ajuste concreto e explique o motivo. Use arrays vazios quando nenhuma mudanca for necessaria. Preserve caixa; nunca altere limites globais de risco ou auditoria, nem emita ordens. O motor deterministico de risco continua autoridade final. Retorne somente JSON no schema."
    ensure_model(client)
    response = client.post(OLLAMA_URL + "/api/chat", json={
        "model": MODEL, "stream": False, "format": schema,
        "options": {"temperature": 0, "num_ctx": MODEL_CONTEXT, "num_predict": 700},
        "messages": [{"role": "system", "content": system},
                     {"role": "user", "content": json.dumps(data, ensure_ascii=False, default=str)}]
    }, timeout=MODEL_TIMEOUT)
    response.raise_for_status()
    content = response.json()["message"]["content"].strip()
    if content.startswith("```"):
        content = re.sub(r"^```(?:json)?\s*|\s*```$", "", content, flags=re.IGNORECASE)
    try:
        result = json.loads(content)
    except json.JSONDecodeError:
        start, end = content.find("{"), content.rfind("}")
        if start < 0 or end <= start:
            raise
        result = json.loads(content[start:end + 1])
    if isinstance(result, str):
        result = json.loads(result)
    if not isinstance(result, dict):
        raise ValueError("Ollama returned a non-object management response")
    return result


MODEL_READY = False


def ensure_model(client):
    global MODEL_READY, MODEL_CONTEXT
    if MODEL_READY:
        return
    global MODEL
    MODEL, memory_mb, cores, resource_slots, review_minutes = hardware_profile(client)
    MODEL_CONTEXT = 4096 if memory_mb >= 4096 else 2048
    with connect() as conn:
        baseline = conn.execute("SELECT count(*) FROM agents WHERE state<>'RETIRED' AND managed_by IS DISTINCT FROM 'director'").fetchone()[0]
        capacity = min(30, max(20, baseline) + resource_slots)
        # Keep the radar useful on compact machines while remaining serial: the
        # market worker and Ollama each process one asset at a time.
        scanner_symbols = min(8, max(3, resource_slots // 4))
        conn.execute("""INSERT INTO ai_runtime_state(id,model,memory_mb,cpu_cores,max_agents,scanner_symbols,review_minutes)
            VALUES(1,%s,%s,%s,%s,%s,%s) ON CONFLICT(id) DO UPDATE SET model=EXCLUDED.model,
            memory_mb=EXCLUDED.memory_mb,cpu_cores=EXCLUDED.cpu_cores,max_agents=EXCLUDED.max_agents,
            scanner_symbols=EXCLUDED.scanner_symbols,review_minutes=EXCLUDED.review_minutes,updated_at=now()""",
            (MODEL, memory_mb, cores, capacity, scanner_symbols, review_minutes))
        conn.execute("UPDATE company_settings SET max_active_agents=%s,hires_per_management_cycle=%s,updated_at=now() WHERE id=1",
                     (capacity, min(3, max(1, resource_slots // 10))))
    log("hardware_profile_selected", model=MODEL, memory_mb=memory_mb, cpu_cores=cores,
        max_agents=capacity, scanner_symbols=scanner_symbols, review_minutes=review_minutes)
    MODEL_READY = True


def fallback(data):
    cash = float((data.get("wallet") or {}).get("cash_brl", 0))
    return {"summary": "Diretoria em contingência: preservar caixa e manter a estrutura atual.",
            "reserve_fraction": 0.70, "order_budget_brl": min(300, max(0, cash * 0.30)),
            "create_teams": [], "create_agents": [], "assignments": [],
            "fire_agents": [], "team_managers": [], "team_states": [], "strategy_weights": [],
            "focus": "Aguardando o modelo local Ollama."}


def bounded(value, default, low, high):
    try:
        return min(high, max(low, float(value)))
    except (ValueError, TypeError):
        return default


def list_value(plan, key):
    value = plan.get(key, [])
    return value if isinstance(value, list) else []


def apply_plan(conn, plan, data):
    max_agents, max_teams, max_hires = conn.execute(
        "SELECT max_active_agents,max_active_teams,hires_per_management_cycle FROM company_settings WHERE id=1").fetchone()
    reserve = bounded(plan.get("reserve_fraction"), 0.70, 0.70, 0.95)
    configured_cap = float(os.getenv("ORDER_BUDGET", "300"))
    cash = float(data["wallet"].get("cash_brl", 0))
    budget = bounded(plan.get("order_budget_brl"), 0, 0, min(configured_cap, cash * (1 - reserve)))
    conn.execute("UPDATE company_settings SET managed_cash_reserve_fraction=%s,managed_order_budget_brl=%s,updated_at=now() WHERE id=1", (reserve, budget))

    made_teams, made_agents, assigned, fired, managers, team_states, strategies = [], [], [], [], [], [], []
    active_teams = conn.execute("SELECT count(*) FROM company_teams WHERE state='ACTIVE'").fetchone()[0]
    team_ids = {name: ident for ident, name in conn.execute("SELECT id,name FROM company_teams").fetchall()}
    for item in list_value(plan, "team_states")[:8]:
        if not isinstance(item, dict):
            continue
        team_id, state = str(item.get("team_id", "")), str(item.get("state", ""))
        if team_id not in team_ids.values() or state not in {"ACTIVE", "PAUSED", "RETIRED"}:
            continue
        active = conn.execute("SELECT count(*) FROM company_teams WHERE state='ACTIVE'").fetchone()[0]
        limit = conn.execute("SELECT max_active_teams FROM company_settings WHERE id=1").fetchone()[0]
        if state == "ACTIVE" and active >= limit:
            continue
        conn.execute("UPDATE company_teams SET state=%s,updated_at=now() WHERE id=%s", (state, team_id))
        team_states.append({"team_id": team_id, "state": state})
    active_teams = conn.execute("SELECT count(*) FROM company_teams WHERE state='ACTIVE'").fetchone()[0]

    # Retire only employees explicitly hired by the AI director; operational
    # infrastructure workers and the fixed risk/execution agents are protected.
    for agent_id in list_value(plan, "fire_agents")[:30]:
        agent_id = str(agent_id)[:80]
        row = conn.execute("SELECT managed_by,state FROM agents WHERE id=%s", (agent_id,)).fetchone()
        if row and row[0] == "director" and row[1] != "RETIRED" and agent_id != "director":
            conn.execute("UPDATE agents SET state='RETIRED',team_id=NULL,updated_at=now() WHERE id=%s", (agent_id,))
            conn.execute("UPDATE company_teams SET manager_agent_id=NULL WHERE manager_agent_id=%s", (agent_id,))
            fired.append(agent_id)
    for item in list_value(plan, "create_teams")[:max_teams]:
        if active_teams >= max_teams or not isinstance(item, dict):
            break
        name = str(item.get("name", "")).strip()[:80]
        department = str(item.get("department_id", "executive"))
        ident = slug(name)
        if not name or not ident or department not in DEPARTMENTS or name in team_ids:
            continue
        result = conn.execute("INSERT INTO company_teams(id,name,department_id,state) VALUES(%s,%s,%s,'ACTIVE') ON CONFLICT DO NOTHING RETURNING id", (ident, name, department)).fetchone()
        if result:
            team_ids[name] = ident
            active_teams += 1
            made_teams.append(ident)

    hire_count = 0
    existing_agents = conn.execute("SELECT count(*) FROM agents WHERE state<>'RETIRED'").fetchone()[0]
    for item in list_value(plan, "create_agents")[:min(max_hires, 3)]:
        if existing_agents >= max_agents or hire_count >= max_hires or not isinstance(item, dict):
            break
        name = str(item.get("name", "")).strip()[:80]
        role = str(item.get("role", "Pesquisador")).strip()[:120]
        room = str(item.get("room", "executive"))
        team_id = team_ids.get(str(item.get("team_name", "")))
        ident = "managed_" + slug(name)
        if not name or ident == "managed_" or room not in DEPARTMENTS:
            continue
        result = conn.execute("""INSERT INTO agents(id,name,role,room,state,detail,managed_by,team_id)
            VALUES(%s,%s,%s,%s,'IDLE',%s,'director',%s) ON CONFLICT(id) DO NOTHING RETURNING id""",
            (ident, name, role, room, Jsonb({"origin": "AI director", "independent_process": False}), team_id)).fetchone()
        if result:
            made_agents.append(ident)
            existing_agents += 1
            hire_count += 1

    for item in list_value(plan, "assignments")[:30]:
        if not isinstance(item, dict):
            continue
        agent_id = str(item.get("agent_id", ""))
        team_id = team_ids.get(str(item.get("team_name", "")))
        room = str(item.get("room", ""))
        if room and room not in DEPARTMENTS:
            continue
        row = conn.execute("SELECT managed_by FROM agents WHERE id=%s", (agent_id,)).fetchone()
        if not row or row[0] != "director":
            continue
        conn.execute("UPDATE agents SET team_id=COALESCE(%s,team_id),room=COALESCE(NULLIF(%s,''),room),updated_at=now() WHERE id=%s",
                     (team_id, room, agent_id))
        assigned.append(agent_id)
    for item in list_value(plan, "team_managers")[:8]:
        if not isinstance(item, dict):
            continue
        team_id, manager_id = str(item.get("team_id", "")), str(item.get("manager_agent_id", ""))
        manager = conn.execute("SELECT state FROM agents WHERE id=%s", (manager_id,)).fetchone()
        if team_id in team_ids.values() and manager and manager[0] != "RETIRED":
            conn.execute("UPDATE company_teams SET manager_agent_id=%s,updated_at=now() WHERE id=%s",
                         (manager_id, team_id))
            managers.append({"team_id": team_id, "manager_agent_id": manager_id})
    for item in list_value(plan, "strategy_weights"):
        if not isinstance(item, dict) or item.get("strategy") not in STRATEGIES:
            continue
        weight = bounded(item.get("weight"), 0, 0, 2)
        enabled = bool(item.get("enabled", True))
        rationale = str(item.get("rationale", "Ajuste da diretoria"))[:180]
        conn.execute("""INSERT INTO strategy_policies(strategy,enabled,weight,rationale)
            VALUES(%s,%s,%s,%s) ON CONFLICT(strategy) DO UPDATE SET enabled=EXCLUDED.enabled,
            weight=EXCLUDED.weight,rationale=EXCLUDED.rationale,updated_at=now()""",
            (item["strategy"], enabled, weight, rationale))
        strategies.append({"strategy": item["strategy"], "enabled": enabled, "weight": weight,
                           "rationale": rationale})
    return {"reserve_fraction": reserve, "order_budget_brl": budget,
            "created_teams": made_teams, "created_agents": made_agents, "assigned_agents": assigned,
            "retired_agents": fired, "team_managers": managers, "team_states": team_states,
            "strategy_weights": strategies}


def cycle(conn, client):
    score_new_fills(conn)
    bootstrap_teams(conn)
    data = context(conn)
    model = MODEL
    try:
        plan = call_model(client, data)
    except (httpx.HTTPError, KeyError, ValueError, TypeError) as exc:
        log("model_unavailable", error=type(exc).__name__)
        plan, model = fallback(data), "deterministic-fallback"
    if not isinstance(plan, dict):
        plan, model = fallback(data), "deterministic-fallback"
    applied = apply_plan(conn, plan, data)
    summary = str(plan.get("summary", "Revisão gerencial concluída."))[:1000]
    if summary.casefold() in {"simulation results", "avaliar a estratégia de trading simulado"}:
        summary = "Revisão concluída com preservação de caixa e limites globais de risco."
    plan = dict(plan)
    plan["reserve_fraction"] = applied["reserve_fraction"]
    plan["order_budget_brl"] = applied["order_budget_brl"]
    plan["create_teams"] = [item for item in list_value(plan, "create_teams")
                             if isinstance(item, dict) and slug(str(item.get("name", ""))) in applied["created_teams"]]
    plan["create_agents"] = [item for item in list_value(plan, "create_agents")
                             if isinstance(item, dict) and "managed_" + slug(str(item.get("name", ""))) in applied["created_agents"]]
    plan["assignments"] = [item for item in list_value(plan, "assignments")
                            if isinstance(item, dict) and str(item.get("agent_id", "")) in applied["assigned_agents"]]
    plan["fire_agents"] = applied["retired_agents"]
    plan["team_managers"] = applied["team_managers"]
    plan["team_states"] = applied["team_states"]
    plan["strategy_weights"] = applied["strategy_weights"]
    summary += f" Reserva de {applied['reserve_fraction']:.0%}; limite de R$ {applied['order_budget_brl']:.2f} por operação, sujeito ao motor de risco."
    review_id = uuid.uuid4()
    conn.execute("INSERT INTO management_reviews(id,model,summary,inputs,recommendations,applied) VALUES(%s,%s,%s,%s,%s,%s)",
                 (review_id, model, summary, Jsonb(data), Jsonb(plan), Jsonb(applied)))
    conn.execute("SELECT emit_activity('director','management_review',%s)",
                 (Jsonb({"review_id": str(review_id), "model": model, "summary": summary, "applied": applied}),))
    register_director(conn, "MONITORING", {"model": model, "last_review_id": str(review_id), "summary": summary, "applied": applied})
    log("review_completed", model=model, review_id=str(review_id), applied=applied)


def main():
    conn = connect()
    force_first_review = True
    with httpx.Client() as client:
        while True:
            try:
                register_director(conn, "ANALYZING", {"model": MODEL, "provider": "ollama", "mode": "company management; orders remain under risk engine"})
                last = conn.execute("SELECT created_at,model FROM management_reviews ORDER BY created_at DESC LIMIT 1").fetchone()
                runtime = conn.execute("SELECT review_minutes FROM ai_runtime_state WHERE id=1").fetchone()
                minutes = runtime[0] if runtime else 30
                if force_first_review or not last or last[1] == "deterministic-fallback" or time.time() - last[0].timestamp() >= minutes * 60:
                    cycle(conn, client)
                    force_first_review = False
                for _ in range(30):
                    time.sleep(10)
                    conn.execute("UPDATE agents SET heartbeat=now() WHERE id='director'")
            except Exception as exc:
                log("cycle_error", error=type(exc).__name__)
                try:
                    register_director(conn, "ERROR", {"error": type(exc).__name__})
                except Exception:
                    conn = connect()
                time.sleep(10)


if __name__ == "__main__":
    main()
