# Validação da entrega 1

Executada em 18/10/2026, Windows com Docker Desktop/engine Linux.
Compilador Rust 1.90.0 em container; não foi necessário instalar Rust no host.

| Verificação | Resultado |
| --- | --- |
| `cargo fmt --all` | Formatação aplicada |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Passou sem warnings |
| `cargo test --locked --workspace` | 4 unitários passaram; integração ignorada conforme configuração |
| `docker compose up --build -d --wait --wait-timeout 180` | Build release passou; 3 serviços saudáveis |
| `docker compose --profile test run --build --rm tests` | 4 unitários + 1 integração passaram, nenhum ignorado |
| `docker compose --profile test run --rm smoke` | 4 testes HTTP passaram |
| HTTP do host `/api/v1/system` e `/health/ready` | Modo simulated, execução false, banco e Redis true |

O teste de integração verifica migration reaplicável, gravação idempotente de
evento, bloqueio de DELETE na auditoria, publicação real em Redis Streams,
rotas HTTP, ausência de endpoint de ordens e readiness 503 após fechamento do
pool de banco, mantendo liveness 200. Não equivale a um teste completo de caos
de rede: perda do Redis entre XADD e commit e replay operacional ficam pendentes.

Total: 9 testes distintos aprovados. A configuração GitHub Actions foi criada,
mas não executada remotamente: este repositório não tem remote configurado.
Os comandos correspondentes foram executados localmente nos containers.

## Limites desta validação

- Não houve conexão à Binance nem teste de ordens, preços ou rentabilidade.
- Não há frontend, escritório 3D, estratégias ou serviço Python de notícias
  implementados; Python nesta entrega executa somente os testes HTTP.
- Ainda não foram testados carga, alta disponibilidade, backup/restauração,
  autorização multiusuário ou integração demo/testnet.
- PostgreSQL e Redis usam volumes locais; a stack foi deixada em execução.
- PR preparada em `docs/pr-001-foundation.md`, sem publicação remota.

Gate da entrega 1: aprovado para iniciar coleta pública de mercado na entrega 2.
