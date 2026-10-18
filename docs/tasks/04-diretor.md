# 4. Diretor autônomo

Fase: 2. Estado: auditada; implementação parcial ou pendente conforme ../progress.md.

## Escopo e critérios

- [ ] Agente executivo com objetivos e memória
- [ ] create_agent e create_team idempotentes
- [ ] assign_task e transfer_agent
- [ ] pause_agent e retire_agent
- [ ] create_research_project e propose_strategy
- [ ] request_backtest e activate_validated_strategy
- [ ] request_market_analysis e request_news_investigation
- [ ] generate_management_report
- [ ] Limites de agentes, chamadas, CPU, memória, concorrência e orçamento
- [ ] ACL por ferramenta e auditoria de ações
- [ ] Proibir alteração de risco global, auditoria, histórico, fundos reais e código arbitrário

## Execução

Inspecionar a implementação existente antes de editar. Implementar os itens ausentes no repositório atual, mantendo trading real desabilitado. Registrar evidências em ../progress.md. Marcar apenas critérios efetivamente verificados.

## Validação

Validar código, persistência e integração aplicáveis. Interfaces sem backend e testes não executados não satisfazem aceitação.

## Evidências desta rodada

Ver a linha correspondente da tabela e as validações em [progresso](../progress.md). Itens não marcados permanecem sem aceitação integral, mesmo quando há código parcial anterior.
