# Entregas incrementais

Cada entrega exige build, lint, testes aplicáveis, documentação e commit pequeno.
O gate bloqueia avanço quando a entrega anterior não pode ser validada. Descrições
de PR ficam locais até haver um remote definido; uma PR não é considerada aberta
sem URL da plataforma.

| # | Entrega / pacote ou serviço | Critério de aceite |
|---|---|---|
| 1 | Fundação: `contracts`, `api`, migrations, Compose | Startup, health, audit/outbox, teste real PostgreSQL/Redis, CI |
| 2 | `services/market-data` Rust | REST + WS Binance, pares spot USDT elegíveis por exchangeInfo, rate limits, reconexão, candles fechados, gaps e fixtures sem rede |
| 3 | `packages/strategy-engine` | MA, RSI, MACD, Bollinger, breakout/volume e momentum; warmup explícito, séries conhecidas e decisões reproduzíveis |
| 4 | Ranking em `services/market-intelligence` | Volume, liquidez, variação, volatilidade, spread, momentum e timeframes; score normalizado versionado, elegibilidade e empate determinístico |
| 5 | `committee` e `risk` separados | BUY/SELL/HOLD auditável, todos os limites solicitados, veto obrigatório, reserva atômica, testes concorrentes e filtros atualizados |
| 6 | `execution` e `exchange-adapters` | Simulador de preço real com taxas/spread/slippage; demo/testnet condicionais, idempotência, reconciliação e testes de falha sem fundos reais |
| 7 | `backtest` | Sem look-ahead, splits temporais, buy-and-hold/referências; PnL realizado/não realizado, taxas, slippage, acerto, profit factor, expectativa, drawdown, exposição e retorno ajustado ao risco |
| 8 | Projeções e WebSocket API | Snapshot+cursor, retomada e dedup, heartbeat, consumer groups, recuperação de pendentes, dead letter, histórico por agente |
| 9 | `apps/office` Vue 3/TS/Vite/Pinia | Dashboard de ranking, posições, PnL e equipes a partir do backend; estados vazio/offline, testes de componentes e E2E |
| 10 | Three.js/TresJS no office | Sete salas, órbita/zoom/seleção, oito estados de personagens, painel de agente real, telões, desempenho e alternativa acessível 2D |
| 11 | `services/news-intelligence` Python | Fontes allowlisted, proveniência, dedup, entidades/sentimento/impacto estruturados, modelo/versão/confiança, defesa contra prompt injection, sem capacidade de ordens |
| 12 | Operação e verificação completa | E2E do pipeline, caos/recovery, limites de carga, observabilidade, backups/replay, permissões, documentação e runbooks |

As sete salas são Market Intelligence, Notícias, Análise Técnica, Comitê,
Gestão de Risco, Execução e Auditoria/Performance. Estados de contrato já definidos:
IDLE, ANALYZING, MEETING, APPROVING, EXECUTING, MONITORING, BLOCKED e ERROR.

## Próximo corte: entrega 2

Consultar documentação oficial atual da Binance antes de fixar endpoints e
filtros. Implementar cliente público sem API keys, universo elegível baseado em
status/permissions/quoteAsset, normalização decimal e persistência idempotente.
Validar desconexão, 429/backoff, eventos duplicados, candle parcial e recuperação
de gaps com fixtures determinísticas; depois realizar smoke público autorizado.
