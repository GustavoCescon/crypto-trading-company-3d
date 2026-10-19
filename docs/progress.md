# Progresso e retomada — master prompt

Data: 18/10/2026. **MVP completo: não.** Os 18 arquivos em `docs/tasks/` são especificações para trabalho de desenvolvimento; não são comandos de shell. A implementação atual cobre controle operacional, ledger BRL inicial, cotação cambial e reset solicitado. Não houve implementação integral de cada arquivo.

## Auditoria inicial

O repositório encontrado já tinha alterações locais, inclusive frontend e notícias ainda não versionados. Foram preservadas. Não foi criado outro projeto nem alterado o motor TresJS/Three.js.

- Rust/Axum/Tokio, TimescaleDB, Redis Streams, outbox, WebSocket e auditoria já funcionavam.
- O scanner consultava `exchangeInfo`, classificava pares USDT e recebia preços públicos por WebSocket. O universo não era uma lista fixa de moedas.
- Seis especialistas determinísticos geravam sinais; comitê, risco e execução fictícia funcionavam fora do navegador.
- Agentes eram registros atualizados por workers fixos. Isso **não** implementava diretor de IA ou contratação dinâmica. A interface chamava indevidamente cada personagem de processo independente.
- Notícias tinham coletor de múltiplos feeds configuráveis, mas somente CoinDesk no padrão. Não havia investigação cruzada nem diretor Ollama.
- Backtest já tinha separação cronológica e referências, mas não havia ciclo de promoção de estratégias declarativas.
- GLB de personagens era gerado pelo script local; não havia evidência de navegação, interações de lazer ou benchmark 20/50/100.
- README descrevia somente a fundação e não correspondia ao código encontrado.

## Investigação financeira: relato de aproximadamente R$ 17

Consulta somente de leitura no PostgreSQL existente encontrou:

- `initial_balance = 10000`, `order_budget = 500` e valores denominados em **USDT**.
- Cinco operações recentes com nocionais de aproximadamente 499,47; 501,87; 499,50; 504,47 e 499,66 USDT.
- O algoritmo calcula quantidade a partir de orçamento fixo, preço com spread/slippage, taxa e arredondamento ao lote; reserva adicional de 1% e limites de exposição podem vetar a operação.
- Não há conversão BRL/USDT implementada. Portanto, não há evidência suficiente para atribuir os R$ 17 ao dimensionamento dessas operações. Pode ser outra métrica ou outra versão, mas isso permanece hipótese.

O dado histórico foi preservado durante a investigação; após autorização explícita do proprietário, foi zerado pelo script transacional `scripts/reset-for-start.sql`. O saldo histórico não foi convertido: a carteira limpa foi criada com R$ 1.000 BRL e sem posições ou ordens abertas.

## Mudanças implementadas

1. Migration `0003_lifecycle.sql`: estado empresarial persistido, política explícita `CLOSE_SIMULATED`, auditoria e trigger que bloqueia compras fora de RUNNING. Instalação vazia começa desligada; carteira legada com posições mantém operação para não interromper proteção.
2. API `POST /api/v1/company/start` e `/stop`: comandos idempotentes, verificações de dependências no início e cancelamento das entradas pendentes/aprovadas no encerramento.
3. Worker de risco reconcilia STARTING/RUNNING/STOPPING/STOPPED/ERROR sem depender do navegador. Exige carteira, cotações recentes e heartbeats de risco, execução e scanner; início sem condições expira após três minutos.
4. Desligamento solicita saídas fictícias usando os mesmos filtros, preços e risco. Permanece pendente quando não houver cotação válida ou condições de negociação. Não declara STOPPED com posições ou ordens ativas.
5. Carteira, aprovação, execução e controle operacional usam o mesmo primeiro lock transacional. Análise verifica RUNNING novamente antes de persistir ordens.
6. Saldo disponível desconta reservas. Patrimônio não zera posições que perderam sua cotação: utiliza custo como fallback, com indicador de cotação desatualizada.
7. Orçamento não positivo e taxa negativa não produzem quantidade negociável. Execução também rejeita cotação excessivamente futura.
8. Cada nova aprovação inclui explicação de orçamento, arredondamento, taxa, reserva e câmbio. O orçamento contábil é BRL; os preços de negociação continuam em USDT.
9. Interface com botão ligar/desligar, progresso, descrições de atividade e área técnica separada. Removido JSON do perfil comum; rótulos principais de operações traduzidos. A tradução integral ainda exige revisão.
10. Modo 2D selecionável e desmontagem da cena quando o documento fica em segundo plano; processamento financeiro independente.
11. Corrigida leitura bloqueante do Redis no WebSocket: ela compartilhava conexão multiplexada com health check e outbox, causando 503 intermitente. Agora usa leitura não bloqueante com intervalo e captura cursor antes do snapshot para evitar perder eventos nessa janela.

## Situação dos 18 arquivos

| Tarefa | Estado real / próximo trabalho |
|---|---|
| 01 Princípios | Parcial: fictício, risco global e capital BRL preservados; faltam autonomia e memória. |
| 02 Stack | Stack preservada; Ollama e orquestrador multiagente ausentes. |
| 03 Organização | Departamentos ainda são agrupamentos visuais. Criar tabelas, relações e gestão. |
| 04 Diretor | Não implementado. Criar ferramentas tipadas, ACL, quotas, idempotência e executor. |
| 05 Multiagente | Workers fixos e registros existentes. Falta fila de tarefas lógicas, memória, retries e orçamento. |
| 06 Notícias | Coletor existente auditado. Falta multifonte padrão validada, proveniência de duplicatas, investigação e contradições. |
| 07 Universo | Descoberta/ranking existentes. Falta histórico auditável de descartes e política completa de elegibilidade. |
| 08 Financeiro | Parcial: wallet BRL 1.000, FX público USDT/BRL, custos/PnL e exposição em BRL; reset feito. Alocações por equipe e série histórica FX pendentes. |
| 09 Estratégias | Sinais e backtest existentes. Faltam promoção determinística, estratégias declarativas e prospectiva. |
| 10 Escritório | GLB e layout existentes preservados. Faltam ambientes e reformulação visual completa. |
| 11 Personagens | Animação de braços existente. Não há navegação nem comandos de lazer reais. |
| 12 Experiência | Perfil sem JSON e área técnica implementados. Falta chat ligado a ferramentas. |
| 13 Operação | Ciclo financeiro implementado/testado. Encerramento de futuras filas/equipes dependerá de 03–05. |
| 14 Performance | Qualidade existente, modo 2D/segundo plano adicionados. Benchmarks não executados. |
| 15 Gestão | Três páginas existentes. Faltam organograma, administração, diretor e custos computacionais. |
| 16 Qualidade | Testes financeiros/operacionais ampliados. Faltam testes das capacidades ainda não implementadas. |
| 17 Execução | Auditoria, tarefas e progresso documentados; execução integral permanece aberta. |
| 18 Aceitação | Não atendida integralmente; não declarar MVP entregue. |

## Validações

- Rust: 18 testes passaram, incluindo duas integrações executadas em bancos temporários; não foram ignoradas.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: passou.
- `cargo fmt --all --check`: passou após formatação.
- Frontend: TypeScript/Vite build passou; Vitest: 2 testes passaram.
- Deploy local aplicado em `http://localhost:5173`, preservando volumes e histórico.
- Teste operacional real da simulação: solicitou desligamento com seis posições; atingiu STOPPED com zero posições/ordens ativas e retornou a RUNNING. Trading real permaneceu desabilitado. O teste está em `tests/lifecycle_smoke.mjs`; executá-lo fecha posições fictícias intencionalmente.
- Notícias: quatro testes Python existentes passaram.
- A primeira execução de smoke HTTP detectou 503 intermitente no Redis; corrigido conforme item 11. Testes finais de navegador e HTTP registrados no fechamento abaixo.
- Fechamento: seis testes Playwright passaram, incluindo seis WebSockets simultâneos mais a conexão da interface; quatro testes HTTP passaram após a correção. Total: 34 testes entre Rust, Vitest, Python e Playwright, além do ciclo operacional de desligamento/reinício.
- Não foi feito benchmark de FPS/memória e não há garantia de desempenho com 100 personagens.

## Como retomar

1. Ler este arquivo e os checklists em `tasks/`; não executar novamente o gerador por cima dos checklists atualizados.
2. Continuar a tarefa 08: criar alocações virtuais auditáveis sem duplicar a carteira central e avaliar série histórica de FX para backtests. A carteira atual foi reiniciada em BRL, em STOPPED, sem posições/fills/decisões antigos.
3. Validar reconciliação de taxas, reserva, patrimônio, PnL e exposição em BRL, inclusive moeda sem cotação e reinício durante transação.
4. Implementar 03–05: organização persistida, tarefas com leases, ferramentas do diretor e políticas. A criação de um agente deve persistir entidade lógica, sem iniciar processos ilimitados.
5. Avançar nas fases 3–5, registrando testes e evidências por checklist.

## Reset e configuração de partida

Reset destrutivo solicitado pelo proprietário em 18/10/2026. O script limpa histórico operacional, preços, carteira, posições, ordens, fills, notícias, backtests e equipes; preserva schema, migrations, departamentos e `company_settings`. Redis Streams limpa a chave `ctc:events:v1` separadamente. A auditoria contém um novo evento `database_reset`.

- Patrimônio e saldo: R$ 1.000,00 fictícios; conta BRL.
- Máximo: 30 agentes, 8 equipes ativas, 3 contratações por ciclo de gestão.
- Revisão: 60 minutos; até 5 posições simultâneas.
- Exposição agregada: até 30% do patrimônio; ordens ainda sujeitas a limite por ativo e demais controles.
- Trading com dinheiro real: desabilitado por restrição de schema e aplicação.
- Estado após reset: empresa `STOPPED`, sem posição/fill/decisão/backtest; agentes de infraestrutura reaparecem quando os workers sobem.
- A taxa USDT/BRL é atualizada por fonte pública e uma cotação recente é obrigatória para ligar a empresa; indisponibilidade bloqueia novas operações.

Comandos de validação no Windows sem Rust local:

```powershell
docker compose --profile test build tests
docker compose --profile test run --rm --no-deps -v ./services:/app/services -v ./packages:/app/packages -v ./migrations:/app/migrations -v ./scripts:/app/scripts tests sh /app/scripts/check-rust.sh
docker compose build api frontend
docker compose up -d --no-build api market risk execution frontend
docker compose --profile test run --rm --no-deps smoke
```

No diretório `apps/office`: `npm test`, `npm run build`, `npm run test:e2e`.

Commits: `af63fec` (tarefas e auditoria) e `8ec91ce` (integração do trabalho local existente com controle operacional e correções). O arquivo de referência `escritorio.jpg` permanece preservado sem inclusão no commit.

Referência da integração existente com dados públicos: [documentação oficial Binance Spot](https://github.com/binance/binance-spot-api-docs/blob/master/rest-api.md). Nenhuma chave de exchange ou endpoint financeiro real foi adicionado.
