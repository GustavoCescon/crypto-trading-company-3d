# Crypto Trading Company 3D

Empresa virtual de trading de criptomoedas, exclusivamente com dinheiro fictício.

**Entrega atual: 1 — fundação.** API Rust/Axum/Tokio, PostgreSQL/TimescaleDB,
Redis Streams, auditoria, outbox, health checks, métricas e testes. Ainda não há
coleta de preços, estratégias, execução, agentes de notícias ou interface 3D.
Nenhum retorno financeiro é calculado ou prometido nesta entrega.

## Executar

Pré-requisito: Docker Desktop com engine Linux e Docker Compose v2.

```powershell
Copy-Item .env.example .env
docker compose up --build -d --wait
Invoke-RestMethod http://localhost:8080/health/ready
Invoke-RestMethod http://localhost:8080/api/v1/system
Invoke-RestMethod http://localhost:8080/api/v1/events
```

No Linux/macOS, use `cp .env.example .env` e `curl` nos mesmos endpoints.
A senha do exemplo é apenas para desenvolvimento local. Ao escolher outra senha,
use caracteres seguros para URL (por exemplo, hex aleatório); a URL de conexão
é interpolada pelo Compose. Não versionar `.env`. Não há chaves de exchange.

A API é publicada somente em `127.0.0.1:8080`. Banco e Redis não publicam portas.
Migrations são aplicadas com lock e checksum pelo SQLx antes de a API escutar.
O Compose aguarda health checks das dependências. As tabelas e o stream começam
sem dados de mercado; a inicialização da API produz um evento operacional real.

```powershell
docker compose logs -f api
docker compose --profile test run --build --rm tests
docker compose --profile test run --rm smoke
docker compose down
```

`down` preserva volumes. `down -v` destrói dados e deve ser usado apenas quando
você decidir descartar o ambiente local. Reiniciar a API registra uma nova
inicialização, com identidade diferente; repetir a gravação do mesmo evento não
duplica o registro. Redis usa AOF; PostgreSQL continua sendo a fonte de auditoria.

## Desenvolvimento

Rust 1.90+ com rustfmt e Clippy, ou a imagem Rust do Dockerfile:

```text
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
```

Os testes de integração são explicitamente ignorados no comando unitário, pois
necessitam de dependências reais; o serviço `tests` os executa com
`--include-ignored`. O smoke Python testa HTTP contra a API em execução.

Endpoints:

| Caminho | Uso |
| --- | --- |
| `/health/live` | Processo respondendo, independente das dependências |
| `/health/ready` | PostgreSQL e Redis disponíveis; 503 em falha |
| `/api/v1/system` | Versão, estágio e execução desabilitada |
| `/api/v1/events` | Últimos 100 eventos, mais recentes primeiro |
| `/metrics` | Contadores Prometheus de publicação e falhas da outbox |

Não existe endpoint para enviar ordens. `TRADING_MODE` diferente de `simulated`
impede a inicialização. As rotas locais não têm autenticação nesta entrega;
autenticação e autorização serão necessárias antes de qualquer exposição remota.

## Organização

```text
packages/contracts/   contratos Rust versionados e estados dos agentes
services/api/         API, configuração, persistência e publicação da outbox
migrations/           esquema SQL auditável e hypertable de candles
tests/                smoke HTTP independente em Python
docs/                 arquitetura, decisões, roteiro e descrição de PR
.github/workflows/    validação em CI
```

O roadmap define quando criar os demais serviços e pacotes, evitando diretórios
vazios e implementações fictícias. Leia [arquitetura](docs/architecture.md),
[decisões](docs/decisions/0001-foundation.md),
[entregas](docs/roadmap.md) e [validação](docs/validation.md).
