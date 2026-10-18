# ADR 0001 — Fundação local, simulação e entrega durável

Status: aceito para entrega 1.

Começar com API e infraestrutura verificáveis. Não criar agentes, rankings ou
resultados financeiros artificiais para preencher a interface.

1. Rust/Axum/Tokio para aplicação concorrente; workspace Cargo para contratos.
2. PostgreSQL/TimescaleDB como estado autoritativo. NUMERIC(38,18) para candles;
   extensão e índices versionados por SQLx, sem dependência de banco na compilação.
3. Outbox transacional para evitar perder eventos entre commit SQL e envio Redis.
   Publicação pode duplicar após falha; consumidores precisam ser idempotentes.
4. Redis AOF e volumes nomeados para desenvolvimento. Não substituem backup.
5. Recusar qualquer modo diferente de `simulated`. Não incluir credenciais,
   adaptadores ou endpoints de ordens nesta fundação.
6. Contratos `/api/v1` e envelope v1. Frontend, Python e domínio entram quando
   houver funcionalidade testável para cada um, conforme roteiro.
7. Health/readiness separados, logs JSON e métricas de publicação. As respostas
   públicas de falha não expõem URLs de banco ou detalhes de credenciais.

Consequências: stack local pequena; o publisher compartilha o processo da API
nesta fase. Antes de suportar ordens, risco e execução terão processos e
credenciais separados. A primeira entrega não oferece segurança multiusuário,
alta disponibilidade, coleta de mercado nem qualquer retorno financeiro.
