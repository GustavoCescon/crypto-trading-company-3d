import { readFileSync, writeFileSync, readdirSync } from 'node:fs'
const verified = new Set([
 'Rust, Tokio e Axum','PostgreSQL e TimescaleDB','Redis Streams, WebSocket, migrations e auditoria','Vue 3, TypeScript, Pinia, TresJS e Three.js',
 'Execução exclusivamente fictícia','Custos, spread, slippage, arredondamento e mínimos',
 'Persistir STOPPED, STARTING, RUNNING, STOPPING e ERROR','Botão principal e progresso','Validar risco antes de entradas','Bloquear e cancelar entradas ao desligar','Persistir resultados','Monitoramento ativo enquanto houver posições pendentes',
 'Remover payloads dos painéis comuns','Diagnóstico técnico separado','Modo 2D','Não prejudicar backend financeiro',
 'Auditar repositório existente','Preservar trabalho local','Criar tarefas markdown e prioridades','Não chamar mock ou estrutura de funcionalidade concluída'
])
for (const filename of readdirSync('docs/tasks').filter(n=>/^\d/.test(n))) {
 const path=`docs/tasks/${filename}`
 let text=readFileSync(path,'utf8').replace('Estado: pendente de execução e validação integral.','Estado: auditada; implementação parcial ou pendente conforme ../progress.md.')
 text=text.split('\n').map(line=>line.startsWith('- [ ] ')&&verified.has(line.slice(6))?line.replace('- [ ]','- [x]'):line).join('\n')
 text+='\n## Evidências desta rodada\n\nVer a linha correspondente da tabela e as validações em [progresso](../progress.md). Itens não marcados permanecem sem aceitação integral, mesmo quando há código parcial anterior.\n'
 writeFileSync(path,text)
}
const path='README.md'
let text=readFileSync(path,'utf8')
const start=text.indexOf('**Entrega atual:')
const end=text.indexOf('## Executar',start)
text=text.slice(0,start)+'**Estado atual: implementação parcial do master prompt.** Há coleta pública Binance, sinais determinísticos, risco, execução fictícia, notícias, backtests e escritório Vue/TresJS. O controle de ligar/desligar é persistido e independente do navegador.\n\n**O MVP solicitado ainda não está completo.** A carteira legada continua em USDT; o capital de R$ 1.000, o diretor autônomo, a organização dinâmica e as interações de lazer ainda estão pendentes. Consulte [tarefas](docs/tasks/README.md) e [progresso, evidências e retomada](docs/progress.md).\n\n'+text.slice(end)
text=text.replace('O roadmap define quando criar os demais serviços e pacotes, evitando diretórios\nvazios e implementações fictícias.', 'A documentação de fundação abaixo registra decisões anteriores. Para o estado atual, use o relatório de progresso e os checklists do master prompt.')
writeFileSync(path,text)
