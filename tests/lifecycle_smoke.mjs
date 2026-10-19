// Explicit operational test: closes paper positions, then restores RUNNING.
// Run only when this simulated shutdown is intended.
import assert from 'node:assert/strict'
const base = process.env.API_URL ?? 'http://localhost:8080'
async function get(path, method='GET') {
  const response=await fetch(base+path,{method,signal:AbortSignal.timeout(5000)})
  assert.equal(response.status,200,`${method} ${path}`)
  return response.json()
}
async function waitFor(target) {
  for(let i=0;i<20;i++) {
    const snapshot=await get('/api/v1/snapshot')
    if(snapshot.company.state===target)return snapshot
    await new Promise(resolve=>setTimeout(resolve,1500))
  }
  throw new Error(`Estado ${target} não atingido. Inspecione o estado persistido; não force o encerramento.`)
}
const before=await get('/api/v1/snapshot')
assert.equal(before.company.state,'RUNNING','Este teste exige empresa inicialmente ligada')
console.log(`Encerramento fictício iniciado com ${before.positions.length} posições.`)
await get('/api/v1/company/stop','POST')
const stopped=await waitFor('STOPPED')
assert.equal(stopped.positions.length,0)
assert.ok(stopped.orders.every(order=>!['APPROVED','PENDING'].includes(order.status)))
assert.equal((await get('/api/v1/system')).paper_execution_enabled,false)
console.log('STOPPED validado: nenhuma posição ou ordem ativa.')
await get('/api/v1/company/start','POST')
await waitFor('RUNNING')
assert.equal((await get('/api/v1/system')).execution_enabled,false)
console.log('RUNNING restaurado. Execução financeira real permanece desabilitada.')
