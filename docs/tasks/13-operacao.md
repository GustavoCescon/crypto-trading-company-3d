# 13. Ligar e desligar empresa

Fase: 1. Estado: auditada; implementação parcial ou pendente conforme ../progress.md.

## Escopo e critérios

- [x] Persistir STOPPED, STARTING, RUNNING, STOPPING e ERROR
- [x] Botão principal e progresso
- [ ] Verificar dependências, restaurar agentes e conectar mercado
- [x] Validar risco antes de entradas
- [x] Bloquear e cancelar entradas ao desligar
- [ ] Finalizar tarefas críticas
- [ ] Fechamento simulado explícito ou aguardar posições
- [x] Persistir resultados
- [x] Monitoramento ativo enquanto houver posições pendentes
- [ ] Recuperação após falha sem declarar desligamento prematuro

## Execução

Inspecionar a implementação existente antes de editar. Implementar os itens ausentes no repositório atual, mantendo trading real desabilitado. Registrar evidências em ../progress.md. Marcar apenas critérios efetivamente verificados.

## Validação

Validar código, persistência e integração aplicáveis. Interfaces sem backend e testes não executados não satisfazem aceitação.

## Evidências desta rodada

Ver a linha correspondente da tabela e as validações em [progresso](../progress.md). Itens não marcados permanecem sem aceitação integral, mesmo quando há código parcial anterior.
