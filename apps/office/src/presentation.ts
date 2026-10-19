import type { Agent } from './types'
export const operationLabels: Record<string,string> = {BUY:'Compra',SELL:'Venda',HOLD:'Aguardar',PENDING:'Aguardando risco',APPROVED:'Aprovada',REJECTED:'Rejeitada',FILLED:'Executada',EXPIRED:'Expirada'}
export const eventLabels: Record<string,string> = {agent_state:'Atividade atualizada',risk_approved:'Operação aprovada pelo risco',risk_rejected:'Operação bloqueada pelo risco',paper_fill:'Operação fictícia executada',protective_exit:'Saída de proteção solicitada',committee_decision:'Decisão do comitê',news_analyzed:'Notícia analisada',execution_expired:'Autorização expirada',company_state:'Estado da empresa atualizado',entries_cancelled:'Entradas canceladas'}
export function agentSummary(agent: Agent): string {
  if (!agent.online) return 'Estou sem comunicação recente. Meu último estado pode estar desatualizado.'
  if (agent.state==='ERROR') return 'Encontrei uma falha operacional. Os detalhes estão disponíveis na área técnica.'
  const detail = agent.detail
  if (typeof detail.reason === 'string') return detail.reason
  if (agent.id==='execution') return agent.state==='EXECUTING'?'Estou registrando uma operação fictícia aprovada pelo risco.':'Estou aguardando o gerente de risco autorizar uma operação fictícia.'
  if (agent.id==='risk') return 'Estou acompanhando exposição, perdas, reservas e limites de risco da carteira.'
  if (agent.id==='audit') return 'Acompanho os registros de operações, reservas e patrimônio para auditoria.'
  if (agent.room==='news') {
    const sources = Array.isArray(detail.sources) ? detail.sources.map(String).map((url) => {
      try { return new URL(url).hostname } catch { return url }
    }) : []
    return sources.length
      ? `${agent.role}: pesquisando em ${sources.join(', ')}. A análise de notícias não autoriza operações.`
      : `${agent.role}: aguardando fontes de pesquisa. A análise de notícias não autoriza operações.`
  }
  if (agent.id==='scanner') return 'Estou classificando pares disponíveis por volume, liquidez, volatilidade e spread.'
  if (agent.id==='feed') return 'Estou recebendo cotações públicas para atualizar as análises e o acompanhamento de risco.'
  if (typeof detail.symbol === 'string') return `Estou analisando ${detail.symbol} com os critérios da minha especialidade.`
  if (typeof detail.candidate === 'string') return `Minha análise destacou ${detail.candidate}. A seleção ainda depende do comitê e do risco.`
  return 'Estou aguardando a próxima tarefa da minha área.'
}
