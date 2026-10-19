# Crypto Trading Company 3D

Empresa virtual de trading de criptomoedas, exclusivamente com dinheiro fictício.

**Estado atual: implementação parcial do master prompt.** Há coleta pública Binance, sinais determinísticos, risco, execução fictícia, notícias, backtests e escritório Vue/TresJS. O controle de ligar/desligar é persistido e independente do navegador.

**O MVP solicitado ainda não está completo.** Nesta configuração, o ledger contábil é BRL com R$ 1.000 de capital fictício; a cotação pública USDT/BRL é exigida para iniciar e dimensionar ordens. Trading real permanece bloqueado. Diretor autônomo, organização operacional dinâmica e interações de lazer seguem pendentes. Consulte [tarefas](docs/tasks/README.md) e [progresso, evidências e retomada](docs/progress.md).

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

A documentação de fundação abaixo registra decisões anteriores. Para o estado atual, use o relatório de progresso e os checklists do master prompt. Leia [arquitetura](docs/architecture.md),
[decisões](docs/decisions/0001-foundation.md),
[entregas](docs/roadmap.md) e [validação](docs/validation.md).
