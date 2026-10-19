<script setup lang="ts">
import { computed,ref,onErrorCaptured,onMounted,onUnmounted } from 'vue'
import { TresCanvas } from '@tresjs/core'
import { Eye,EyeOff,Pause,Play,Settings2 } from 'lucide-vue-next'
import OfficeScene from './OfficeScene.vue'
import { useCompany } from '../store'
import { stateLabels } from '../types'
const company=useCompany(), mode2d=ref(false), hidden=ref(document.hidden)
function visibility(){hidden.value=document.hidden}
onMounted(()=>document.addEventListener('visibilitychange',visibility))
onUnmounted(()=>document.removeEventListener('visibilitychange',visibility))
const failed=ref(false),paused=ref(false),showPnl=ref(true),quality=ref<'economy'|'balanced'|'detail'>('balanced')
const dpr=computed(()=>quality.value==='economy'?0.8:quality.value==='detail'?2:1.5)
const qualityLabel=computed(()=>quality.value==='economy'?'Leve':quality.value==='detail'?'Detalhe':'Equilibrado')
const mobile=window.matchMedia('(max-width: 780px)')
function adapt(){if(mobile.matches&&quality.value==='balanced')quality.value='economy'}
onMounted(()=>{adapt();mobile.addEventListener('change',adapt)})
onUnmounted(()=>mobile.removeEventListener('change',adapt))
onErrorCaptured(error=>{console.error('Office 3D failed',error);failed.value=true;return false})
</script>
<template>
  <div class="office-canvas" data-testid="office" aria-label="Trading floor 3D com oito departamentos e equipes do backend">
    <TresCanvas v-if="!failed&&!mode2d&&!hidden" class="office-webgl" clear-color="#b9b5aa" :dpr="dpr" :alpha="false" :antialias="quality!=='economy'" :shadows="quality!=='economy'" :power-preference="'high-performance'">
      <OfficeScene :paused="paused" :show-pnl="showPnl"/>
    </TresCanvas>
    <div v-else class="office-fallback" role="status"><div><h2>{{mode2d?'Visão 2D da empresa':hidden?'Escritório em segundo plano':'Visualização 3D indisponível'}}</h2><p>Os serviços e o monitoramento de risco continuam no backend.</p><button v-for="agent in company.data?.agents" :key="agent.id" @click="company.selected=agent.id">{{agent.name}} · {{agent.online?stateLabels[agent.state]:'Sem comunicação'}}</button></div></div>
    <div class="scene-tools" aria-label="Controles do escritÃ³rio 3D">
      <button :aria-pressed="mode2d" @click="mode2d=!mode2d">{{mode2d?'Exibir escritório 3D':'Usar modo 2D'}}</button>
      <button :aria-pressed="paused" :title="paused?'Retomar animaÃ§Ãµes':'Pausar animaÃ§Ãµes'" @click="paused=!paused"><Play v-if="paused" :size="14"/><Pause v-else :size="14"/>{{paused?'Retomar':'Pausar'}}</button>
      <button :aria-pressed="!showPnl" :title="showPnl?'Ocultar P&L':'Mostrar P&L'" @click="showPnl=!showPnl"><EyeOff v-if="showPnl" :size="14"/><Eye v-else :size="14"/>{{showPnl?'Ocultar P&L':'Exibir P&L'}}</button>
      <label><Settings2 :size="14"/><select v-model="quality" aria-label="Qualidade grÃ¡fica"><option value="economy">Leve</option><option value="balanced">Equilibrado</option><option value="detail">Detalhe</option></select></label>
      <span class="quality-state">{{qualityLabel}}</span>
    </div>
  </div>
</template>
