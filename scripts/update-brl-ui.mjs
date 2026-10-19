import {readFileSync,writeFileSync} from 'node:fs'
function replace(path, pairs) {
 let source=readFileSync(path,'utf8')
 for (const [before,after] of pairs) {
  if (!source.includes(before)) throw new Error(`UI marker absent in ${path}: ${before}`)
  source=source.replaceAll(before,after)
 }
 writeFileSync(path,source)
}
replace('apps/office/src/App.vue',[
 ['const stale=computed(()=>company.data?.positions.some(p=>p.stale))','const stale=computed(()=>company.data?.positions.some(p=>p.stale))\nconst fxStale=computed(()=>company.data?.account?.fx_stale===true)'],
 ['        <CompanyControl/><section class="metrics">','        <div v-if="fxStale" class="notice">A cotaÃ§Ã£o USDT/BRL estÃ¡ desatualizada. O motor de risco bloqueia ordens atÃ© receber uma cotaÃ§Ã£o nova.</div>\n        <CompanyControl/><section class="metrics">'],
 ['Patrimônio simulado <span>USDT</span>','Patrimônio simulado <span>BRL</span>'],
 ['max_positions??6','max_positions??5'],
 ["{{money(company.data?.account?.exposure)}} USDT","{{money(company.data?.account?.exposure)}} BRL"],
 ["{{money(company.data?.account?.fees)}} USDT","{{money(company.data?.account?.fees)}} BRL"],
 [' USDT <small>P&amp;L realizado atribuído',' BRL <small>P&amp;L realizado atribuído'],
 [' USDT <small>P&amp;L realizado desta estratégia',' BRL <small>P&amp;L realizado desta estratégia'],
 ['{{money(team.realized_pnl)}} USDT','{{money(team.realized_pnl)}} BRL'],
 ['<span>USDT · sem fundos reais</span>','<span>Carteira em BRL · cotações e preços em USDT</span>'],
 ['<th>TAXA</th><th>PNL REALIZADO','<th>TAXA (BRL)</th><th>PNL REALIZADO (BRL)'],
 ['<p>Histórico real de 1 minuto.','<p>Valores do backtest em USDT; a equivalência de R$ 1.000 usa o câmbio atual, sem série histórica BRL. Histórico real de 1 minuto.']
])
replace('apps/office/src/components/OfficeScene.vue',[
 ['SALDO  $${Number(a.cash)','SALDO  R$ ${Number(a.cash)'],
 ['P&L SIM  $${(Number(a.realized_pnl)','P&L SIM  R$ ${(Number(a.realized_pnl)']
])
