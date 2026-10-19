import { test, expect } from '@playwright/test'

test('control shows authoritative lifecycle and 2D mode preserves management', async ({ page, request }) => {
  const snapshot = await (await request.get('/api/v1/snapshot')).json()
  expect(['STOPPED','STARTING','RUNNING','STOPPING','ERROR']).toContain(snapshot.company.state)
  const status = await (await request.get('/api/v1/system')).json()
  expect(status.execution_enabled).toBe(false)
  expect(status.mvp_complete).toBe(false)
  await page.goto('/', {waitUntil:'domcontentloaded'})
  await expect(page.getByRole('region', {name:'Controle operacional'})).toBeVisible()
  await page.getByRole('button', {name:'Usar modo 2D'}).click()
  await expect(page.getByRole('heading', {name:'Visão 2D da empresa'})).toBeVisible()
  await expect(page.getByTestId('office').locator('canvas')).toHaveCount(0)
  if (snapshot.agents.length) {
    await page.locator('.agent-list button').first().click()
    await expect(page.locator('.agent-description')).toBeVisible()
    await expect(page.locator('.agent-detail pre')).toHaveCount(0)
  }
  await page.getByRole('button', {name:'Exibir escritório 3D'}).click()
  await expect(page.getByTestId('office').locator('canvas')).toBeVisible()
})

test('multiple live sockets do not block readiness', async ({ page, request }) => {
  await page.goto('/', {waitUntil:'domcontentloaded'})
  await page.evaluate(async () => {
    const sockets = Array.from({length:6}, () => new WebSocket(`ws://${location.host}/api/v1/ws`))
    ;(window as unknown as {testSockets:WebSocket[]}).testSockets = sockets
    await Promise.all(sockets.map(socket => new Promise<void>((resolve,reject) => {
      const timer=setTimeout(()=>reject(new Error('WebSocket snapshot timeout')),10000)
      socket.onmessage=()=>{clearTimeout(timer);resolve()}
      socket.onerror=()=>{clearTimeout(timer);reject(new Error('WebSocket failed'))}
    })))
  })
  try {
    for (let i=0;i<8;i++) {
      const response=await request.get('/health/ready', {timeout:3000})
      expect(response.status()).toBe(200)
      expect((await response.json()).ready).toBe(true)
    }
  } finally {
    await page.evaluate(()=>(window as unknown as {testSockets:WebSocket[]}).testSockets.forEach(socket=>socket.close()))
  }
})
