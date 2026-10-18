# Fundação Rust com auditoria durável e infraestrutura local

Cria a primeira entrega do Crypto Trading Company 3D: API Axum/Tokio com
PostgreSQL/TimescaleDB e Redis Streams em Docker Compose. A inicialização registra
um evento transacional e o publica pela outbox, preservando auditoria após retry.

O modo permitido é exclusivamente `simulated` e não existem rotas de ordens.
Health checks distinguem processo ativo de dependências disponíveis; métricas
expõem publicação e falhas. Contratos v1 e roteiro definem os próximos serviços.

Validação e limitações: consultar `docs/validation.md`. CI executa formato, Clippy,
testes unitários, integração com dependências reais e smoke HTTP em containers.

Esta é uma descrição de PR pronta para revisão local, não uma PR publicada.
