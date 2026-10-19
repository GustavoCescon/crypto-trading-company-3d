<script setup lang="ts">
import { computed, ref } from 'vue'
import { useCompany } from '../store'
const company = useCompany(), busy = ref(false), error = ref('')
const state = computed(() => company.data?.company?.state)
const labels: Record<string,string> = {STOPPED:'Desligada',STARTING:'Iniciando',RUNNING:'Em operação',STOPPING:'Encerrando expediente',ERROR:'Atenção necessária'}
async function command(action: 'start'|'stop') {
  busy.value = true; error.value = ''
  try {
    const response = await fetch(`/api/v1/company/${action}`, {method:'POST'})
    if (!response.ok) throw new Error('Não foi possível alterar o estado da empresa. Verifique a conexão e tente novamente.')
    await company.refresh()
  } catch (e) { error.value = e instanceof Error ? e.message : 'Falha de comunicação' }
  finally { busy.value = false }
}
</script>
<template>
  <section class="company-control" aria-label="Controle operacional">
    <div><strong>{{state ? labels[state] : 'Consultando empresa…'}}</strong><p role="status">{{company.data?.company?.message}}</p><small>Desligar bloqueia compras e solicita o fechamento fictício das posições. O risco permanece ativo até concluir.</small></div>
    <button v-if="state==='RUNNING'||state==='STARTING'" class="primary" :disabled="busy" @click="command('stop')">Desligar empresa</button>
    <button v-else class="primary" :disabled="busy||!state||state==='STOPPING'" @click="command('start')">{{state==='STOPPING'?'Finalizando operações…':'Ligar empresa'}}</button>
    <p v-if="error" role="alert">{{error}}</p>
  </section>
</template>
<style scoped>
.company-control{display:flex;align-items:center;justify-content:space-between;gap:20px;padding:18px 22px;margin-bottom:20px;border:1px solid #cbd5cb;border-radius:12px;background:#eef3eb}.company-control p{margin:6px 0;font-size:13px}.company-control small{color:#526252}.company-control button{white-space:nowrap}@media(max-width:780px){.company-control{align-items:flex-start;flex-direction:column}}
</style>
