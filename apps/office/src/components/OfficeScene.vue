<script setup lang="ts">
import { computed, shallowRef } from 'vue'
import { OrbitControls } from '@tresjs/cientos'
import { useLoop } from '@tresjs/core'
import { Vector3, type Camera } from 'three'
import { useCompany } from '../store'
import Character from './Character.vue'
import FloorKit from './FloorKit.vue'
import Screen from './Screen.vue'
const props=defineProps<{paused:boolean;showPnl:boolean}>()
const company=useCompany(),camera=shallowRef<any>(),controls=shallowRef<any>()
type Zone={id:string;name:string;x:number;z:number;color:string}
const palette:Record<string,string>={market:'#49ad91',technical:'#80a9dc',news:'#e3bd75',committee:'#b5a0d3',risk:'#ee887c',board:'#71b9d0',cafe:'#a3c98a',floor:'#c6ec8c'}
const defs=[['market','INTELIGENCIA DE MERCADO',-17,-13],['technical','PESQUISA QUANT',0,-13],['news','NOTICIAS E ANALISE',17,-13],['committee','COMITE DE INVESTIMENTOS',-18,1],['floor','MESA DE OPERACOES',0,1],['risk','CONTROLE DE RISCO',18,1],['board','DIRETORIA',-9,15],['cafe','LOUNGE',9,15]] as const
function home(a:any){if(a.id.startsWith('analyst_')||a.room==='execution')return'floor';if(a.room==='audit'||a.room==='executive')return'board';if(a.room==='quant')return'technical';return a.room}
function zoneFor(a:any){if(['BLOCKED','ERROR'].includes(a.state))return'cafe';if(a.state==='APPROVING')return'committee';if(a.state==='MEETING')return'board';if(a.state==='EXECUTING')return'floor';return home(a)}
function activity(a:any){if(a.state==='EXECUTING'&&a.detail?.symbol)return`${a.detail.side==='SELL'?'VENDA':'COMPRA'} ${a.detail.symbol}`;if(a.state==='APPROVING')return'ORDEM EM ANALISE';if(a.state==='MEETING')return'REUNIAO DE EQUIPE';if(['BLOCKED','ERROR'].includes(a.state))return'EM REVISAO';if(a.state==='ANALYZING')return'ANALISANDO';return'MONITORANDO MERCADO'}
const zones=computed<Zone[]>(()=>defs.map(([id,name,x,z])=>({id,name,x,z,color:palette[id]})))
const layout=computed(()=>{const out:Array<{agent:any;position:[number,number,number];desk:{x:number;z:number};route:[number,number,number][];bubble:string;pnl:string|null}>=[];for(const zone of zones.value){const crew=(company.data?.agents??[]).filter(a=>zoneFor(a)===zone.id),cols=Math.min(zone.id==='floor'?6:4,Math.max(1,Math.ceil(Math.sqrt(crew.length*1.5)))),rows=Math.max(1,Math.ceil(crew.length/cols));crew.forEach((agent,i)=>{const col=i%cols,row=Math.floor(i/cols),x=zone.x+(col-(Math.min(cols,crew.length)-1)/2)*3.05,z=zone.z+(row-(rows-1)/2)*3.65,homeZone=zones.value.find(v=>v.id===home(agent))??zone,entry=(p:Zone):[number,number,number]=>[p.x,.04,p.z+6.2],route:[number,number,number][]=homeZone.id===zone.id?[entry(zone),[x,.04,z+1.15]]:[entry(homeZone),[homeZone.x,.04,0],[zone.x,.04,0],entry(zone),[x,.04,z+1.15]],team=company.data?.teams.find(t=>t.strategy.toLowerCase()===agent.id.toLowerCase());out.push({agent,position:[x,.045,z+1.15],desk:{x,z},route,bubble:activity(agent),pnl:props.showPnl&&team?team.realized_pnl:null})})}return out})
const desks=computed(()=>layout.value.map(v=>v.desk)),plants=computed(()=>[{x:-27,z:-20},{x:27,z:-20},{x:-27,z:20},{x:27,z:20},{x:-29,z:0},{x:29,z:0}])
const ranking=computed(()=>company.data?.ranking.slice(0,5).map(r=>`${r.symbol.padEnd(10)} ${Number(r.change)>=0?'+':''}${Number(r.change).toFixed(2)}%`)),account=computed(()=>{const a=company.data?.account;return a?[`CAIXA  R$ ${Number(a.cash).toLocaleString('pt-BR',{maximumFractionDigits:0})}`,`P&L  R$ ${(Number(a.realized_pnl)+Number(a.unrealized_pnl)).toLocaleString('pt-BR',{minimumFractionDigits:2,maximumFractionDigits:2})}`,`${company.data?.positions.length??0} POSICOES ABERTAS`]:['CONECTANDO A PLATAFORMA']})
const cameraTarget=new Vector3(),cameraPosition=new Vector3();let focusing=false
function focus(zone:Zone){company.selected=null;company.selectedTeam=zone.id;const orbit=controls.value?.instance??controls.value,cam=(camera.value?.instance??camera.value) as Camera|undefined;if(!orbit||!cam)return;const to=new Vector3(zone.x,0,zone.z),offset=cam.position.clone().sub(orbit.target).multiplyScalar(.55);cameraTarget.copy(to);cameraPosition.copy(to).add(offset);cameraPosition.y=Math.max(23,cam.position.y*.58);focusing=true}
useLoop().onBeforeRender(()=>{const orbit=controls.value?.instance??controls.value,cam=(camera.value?.instance??camera.value) as any;if(!orbit||!cam||!focusing)return;orbit.target.lerp(cameraTarget,.055);cam.position.lerp(cameraPosition,.055);orbit.update();if(cam.position.distanceTo(cameraPosition)<.08)focusing=false})
</script>
<template>
  <TresOrthographicCamera ref="camera" :position="[58,76,64]" :left="-34" :right="34" :top="22" :bottom="-22" :near=".1" :far="240"/>
  <OrbitControls ref="controls" :target="[0,0,0]" :min-distance="22" :max-distance="100" :max-polar-angle="1.18" :min-polar-angle=".58" enable-damping :damping-factor=".08"/>
  <TresAmbientLight :intensity=".9"/><TresHemisphereLight color="#f9f1e2" ground-color="#14191c" :intensity=".72"/><TresDirectionalLight :position="[-30,52,24]" :intensity="2.2" cast-shadow/>
  <!-- one continuous operations hall -->
  <TresMesh :position="[0,-.53,0]" receive-shadow><TresBoxGeometry :args="[62,1.05,48]"/><TresMeshStandardMaterial color="#171c1e" :roughness=".86"/></TresMesh>
  <TresMesh :position="[0,-1.08,0]"><TresBoxGeometry :args="[63,.12,49]"/><TresMeshStandardMaterial color="#333a39" :roughness=".58"/></TresMesh>
  <TresMesh :position="[0,.02,0]"><TresBoxGeometry :args="[61,.08,47]"/><TresMeshStandardMaterial color="#292e2f" :roughness=".76"/></TresMesh>
  <!-- timber feature wall with architectural slats and market display -->
  <TresMesh :position="[0,4.8,-23.4]" cast-shadow><TresBoxGeometry :args="[60,9.8,.45]"/><TresMeshStandardMaterial color="#755b42" :roughness=".73"/></TresMesh>
  <TresMesh v-for="n in 74" :key="`slat-${n}`" :position="[(n-37)*.8,4.8,-23.12]" cast-shadow><TresBoxGeometry :args="[.075,9.5,.12]"/><TresMeshStandardMaterial color="#a17e56" :roughness=".68"/></TresMesh>
  <TresMesh :position="[0,9.72,-23]" cast-shadow><TresBoxGeometry :args="[60,.22,.7]"/><TresMeshStandardMaterial color="#343b39" :metalness=".3"/></TresMesh>
  <TresMesh :position="[0,.3,-23]" cast-shadow><TresBoxGeometry :args="[60,.5,.65]"/><TresMeshStandardMaterial color="#333a38"/></TresMesh>
  <Screen :position="[0,8,-22.88]" title="MERCADOS AO VIVO  /  OPERACOES" :lines="[...(ranking?.slice(0,2)??[]),...(account??[])]" :width="25" :height="5.2" color="#9edb9e"/>
  <!-- Brazilian flag and company mark -->
  <TresMesh :position="[-23,8.7,-22.82]"><TresBoxGeometry :args="[4.3,2.7,.05]"/><TresMeshBasicMaterial color="#178344"/></TresMesh>
  <TresMesh :position="[-23,8.7,-22.76]" :rotation="[0,0,.785]"><TresBoxGeometry :args="[2.25,2.25,.06]"/><TresMeshBasicMaterial color="#f3cc37"/></TresMesh>
  <TresMesh :position="[-23,8.7,-22.69]"><TresSphereGeometry :args="[.72,24,16]"/><TresMeshBasicMaterial color="#2465a6"/></TresMesh>
  <!-- side glazing, window frames, and structural columns -->
  <TresMesh v-for="side in [-1,1]" :key="`side-wall-${side}`" :position="[side*30.5,1.1,0]" cast-shadow><TresBoxGeometry :args="[.45,2.2,46]"/><TresMeshStandardMaterial color="#d3d0c5" :roughness=".74"/></TresMesh>
  <TresMesh v-for="side in [-1,1]" :key="`window-${side}`" :position="[side*30.23,4.6,0]"><TresBoxGeometry :args="[.05,4.4,38]"/><TresMeshPhysicalMaterial color="#9db4b6" :transparent="true" :opacity=".28" :metalness=".08" :roughness=".18"/></TresMesh>
  <!-- glazed main entrance and double glass doors -->
  <TresMesh v-for="side in [-1,1]" :key="`entry-wing-${side}`" :position="[side*8,3.1,23.7]"><TresBoxGeometry :args="[16,6.2,.16]"/><TresMeshPhysicalMaterial color="#a9c4c2" :transparent="true" :opacity=".23" :metalness=".12" :roughness=".14"/></TresMesh>
  <TresMesh v-for="side in [-1,1]" :key="`entry-door-${side}`" :position="[side*1.05,2.1,23.68]" :rotation="[0,side*.045,0]" cast-shadow><TresBoxGeometry :args="[2.05,4.15,.12]"/><TresMeshPhysicalMaterial color="#b6d0cd" :transparent="true" :opacity=".34" :metalness=".2" :roughness=".12"/></TresMesh>
  <TresMesh v-for="side in [-1,1]" :key="`entry-handle-${side}`" :position="[side*.38,2,23.53]"><TresCylinderGeometry :args="[.035,.035,.72,12]"/><TresMeshStandardMaterial color="#c6aa73" :metalness=".86" :roughness=".24"/></TresMesh>
  <TresMesh :position="[0,4.35,23.62]"><TresBoxGeometry :args="[4.4,.12,.18]"/><TresMeshStandardMaterial color="#48534f" :metalness=".55"/></TresMesh>
  <TresMesh v-for="side in [-1,1]" :key="`entry-pier-${side}`" :position="[side*10,3,23.65]"><TresBoxGeometry :args="[.28,6,.28]"/><TresMeshStandardMaterial color="#48534f" :metalness=".52"/></TresMesh>
  <TresGroup v-for="side in [-1,1]" :key="`frames-${side}`"><TresMesh v-for="z in [-17,-9,-1,7,15]" :key="z" :position="[side*30.15,4.5,z]"><TresBoxGeometry :args="[.12,5, .12]"/><TresMeshStandardMaterial color="#59615e" :metalness=".55"/></TresMesh></TresGroup>
  <!-- canopy beams and warm linear luminaires -->
  <TresMesh v-for="z in [-18,-7,5,17]" :key="`canopy-${z}`" :position="[0,10,z]"><TresBoxGeometry :args="[58,.22,.28]"/><TresMeshStandardMaterial color="#535955" :metalness=".38"/></TresMesh>
  <TresMesh v-for="z in [-15,-3,9]" :key="`light-${z}`" :position="[0,9.2,z]"><TresBoxGeometry :args="[34,.06,.2]"/><TresMeshStandardMaterial color="#fff0d3" emissive="#ffe5ad" :emissive-intensity=".62"/></TresMesh>
  <!-- dedicated work islands keep each department legible without boxing in the floor -->
  <TresGroup v-for="zone in zones" :key="zone.id">
    <TresMesh :position="[zone.x,.075,zone.z]" @click.stop="focus(zone)"><TresBoxGeometry :args="[zone.id==='floor'?21:16,.045,zone.id==='floor'?15:11]"/><TresMeshStandardMaterial :color="zone.id==='floor'?'#202b28':'#252a29'" :roughness=".82"/></TresMesh>
    <TresMesh :position="[zone.x,.12,zone.z+(zone.id==='floor'?7.1:5.1)]"><TresBoxGeometry :args="[zone.id==='floor'?20:15,.035,.08]"/><TresMeshStandardMaterial :color="zone.color" :emissive="zone.color" :emissive-intensity=".22"/></TresMesh>
    <template v-if="!['floor','cafe'].includes(zone.id)">
      <TresMesh :position="[zone.x-8,1.65,zone.z]" cast-shadow><TresBoxGeometry :args="[.12,3.25,10.8]"/><TresMeshPhysicalMaterial color="#aac0be" :transparent="true" :opacity=".17" :metalness=".12" :roughness=".12"/></TresMesh>
      <TresMesh :position="[zone.x+8,1.65,zone.z]" cast-shadow><TresBoxGeometry :args="[.12,3.25,10.8]"/><TresMeshPhysicalMaterial color="#aac0be" :transparent="true" :opacity=".17" :metalness=".12" :roughness=".12"/></TresMesh>
      <TresMesh :position="[zone.x,1.65,zone.z-5.5]" cast-shadow><TresBoxGeometry :args="[15.8,3.25,.1]"/><TresMeshPhysicalMaterial color="#aac0be" :transparent="true" :opacity=".15" :metalness=".12" :roughness=".12"/></TresMesh>
      <TresMesh v-for="side in [-1,1]" :key="`glass-rail-${side}`" :position="[zone.x+side*8, .12, zone.z]"><TresBoxGeometry :args="[.17,.16,11]"/><TresMeshStandardMaterial color="#414946" :metalness=".58"/></TresMesh>
      <TresMesh v-for="side in [-1,1]" :key="`glass-cap-${side}`" :position="[zone.x+side*8,3.32,zone.z]"><TresBoxGeometry :args="[.17,.13,11]"/><TresMeshStandardMaterial color="#414946" :metalness=".58"/></TresMesh>
      <TresMesh v-for="end in [-1,1]" :key="`glass-post-${end}`" :position="[zone.x-8,1.7,zone.z+end*5.45]"><TresBoxGeometry :args="[.17,3.4,.17]"/><TresMeshStandardMaterial color="#414946" :metalness=".58"/></TresMesh>
      <TresMesh v-for="end in [-1,1]" :key="`glass-top-post-${end}`" :position="[zone.x+8,1.7,zone.z+end*5.45]"><TresBoxGeometry :args="[.17,3.4,.17]"/><TresMeshStandardMaterial color="#414946" :metalness=".58"/></TresMesh>
      <TresMesh v-for="side in [-1,1]" :key="`front-glass-${side}`" :position="[zone.x+side*5.25,1.65,zone.z+5.5]" cast-shadow><TresBoxGeometry :args="[4.5,3.25,.1]"/><TresMeshPhysicalMaterial color="#aac0be" :transparent="true" :opacity=".17" :metalness=".12" :roughness=".12"/></TresMesh>
      <TresMesh v-for="side in [-1,1]" :key="`door-side-glass-${side}`" :position="[zone.x+side*2.45,1.65,zone.z+5.5]"><TresBoxGeometry :args="[1.05,3.25,.08]"/><TresMeshPhysicalMaterial color="#aac0be" :transparent="true" :opacity=".17" :metalness=".12" :roughness=".12"/></TresMesh>
      <TresMesh v-for="side in [-1,1]" :key="`jamb-${side}`" :position="[zone.x+side*8,1.7,zone.z+5.5]"><TresBoxGeometry :args="[.16,3.4,.2]"/><TresMeshStandardMaterial color="#3f4946" :metalness=".6"/></TresMesh>
      <TresMesh v-for="side in [-1,1]" :key="`door-frame-${side}`" :position="[zone.x+side*1.9,1.7,zone.z+5.5]"><TresBoxGeometry :args="[.14,3.4,.2]"/><TresMeshStandardMaterial color="#3f4946" :metalness=".6"/></TresMesh>
      <TresMesh :position="[zone.x,3.35,zone.z+5.5]"><TresBoxGeometry :args="[16,.16,.2]"/><TresMeshStandardMaterial color="#3f4946" :metalness=".6"/></TresMesh>
      <TresGroup :position="[zone.x-1.82,.06,zone.z+5.42]" :rotation="[0,.38,0]">
        <TresMesh :position="[1.82,1.55,0]" cast-shadow><TresBoxGeometry :args="[3.62,3.04,.08]"/><TresMeshPhysicalMaterial color="#b8d0cd" :transparent="true" :opacity=".28" :metalness=".18" :roughness=".12"/></TresMesh>
        <TresMesh :position="[3.35,1.5,.13]"><TresBoxGeometry :args="[.06,.72,.12]"/><TresMeshStandardMaterial color="#b99b68" :metalness=".8" :roughness=".28"/></TresMesh>
      </TresGroup>
      <TresMesh :position="[zone.x, .12, zone.z-5.5]"><TresBoxGeometry :args="[16,.16,.16]"/><TresMeshStandardMaterial color="#414946" :metalness=".48"/></TresMesh>
      <TresMesh :position="[zone.x,3.3,zone.z-5.5]"><TresBoxGeometry :args="[16,.14,.16]"/><TresMeshStandardMaterial color="#414946" :metalness=".48"/></TresMesh>
      <TresMesh v-for="side in [-1,1]" :key="`glass-mullion-${side}`" :position="[zone.x+side*4,1.65,zone.z-5.58]"><TresBoxGeometry :args="[.07,3.2,.08]"/><TresMeshStandardMaterial color="#606b67" :metalness=".54"/></TresMesh>
    </template>
    <Screen :position="[zone.x,3.8,zone.z-(zone.id==='floor'?6.7:4.7)]" :title="zone.name" :width="zone.id==='floor'?11:9" :height="1.7" :color="zone.color"/>
    <TresMesh :position="[zone.x,.001,zone.z]" @click.stop="focus(zone)"><TresBoxGeometry :args="[zone.id==='floor'?21:16,.015,zone.id==='floor'?15:11]"/><TresMeshBasicMaterial :transparent="true" :opacity="0" :depth-write="false"/></TresMesh>
  </TresGroup>
  <!-- quiet lounge seating in the front right corner -->
  <TresGroup :position="[9,0,15]">
    <TresMesh :position="[0,.45,0]" cast-shadow><TresBoxGeometry :args="[4,.72,.9]"/><TresMeshStandardMaterial color="#626a60" :roughness=".86"/></TresMesh>
    <TresMesh :position="[0,.84,.02]"><TresBoxGeometry :args="[3.9,.1,.86]"/><TresMeshStandardMaterial color="#c8b899" :roughness=".52"/></TresMesh>
    <TresMesh :position="[-3,.45,-2.4]" cast-shadow><TresBoxGeometry :args="[2.8,.72,.9]"/><TresMeshStandardMaterial color="#626a60"/></TresMesh>
    <TresMesh :position="[-3,.84,-2.4]"><TresBoxGeometry :args="[2.7,.1,.86]"/><TresMeshStandardMaterial color="#c8b899"/></TresMesh>
    <TresMesh :position="[0,.4,2.1]" cast-shadow><TresCylinderGeometry :args="[1.05,1.05,.12,32]"/><TresMeshStandardMaterial color="#876d4c"/></TresMesh>
    <TresMesh :position="[0,.75,2.1]"><TresCylinderGeometry :args="[1.1,1.1,.1,32]"/><TresMeshStandardMaterial color="#c4ae81"/></TresMesh>
  </TresGroup>
  <!-- reception desk beside the entrance -->
  <TresGroup :position="[-23,0,16.3]">
    <TresMesh :position="[0,.72,0]" cast-shadow receive-shadow><TresBoxGeometry :args="[5.8,1.38,1.55]"/><TresMeshStandardMaterial color="#66523e" :roughness=".55"/></TresMesh>
    <TresMesh :position="[0,1.46,-.05]" cast-shadow><TresBoxGeometry :args="[5.92,.12,1.62]"/><TresMeshStandardMaterial color="#d7c7a8" :roughness=".32"/></TresMesh>
    <TresMesh :position="[0,.22,.15]"><TresBoxGeometry :args="[5.55,.12,1.3]"/><TresMeshStandardMaterial color="#313a38" :metalness=".32"/></TresMesh>
    <Screen :position="[0,2.8,-.72]" title="RECEPCAO" :lines="['ESCRITORIO CENTRAL','BEM-VINDO']" :width="4.7" :height="1.6" color="#c6ec8c"/>
  </TresGroup>
  <FloorKit :desks="desks" :plants="plants"/>
  <TresGroup v-for="m in layout" :key="m.agent.id"><Character :agent="m.agent" :position="m.position" :route="m.route" :bubble="m.bubble" :selected="company.selected===m.agent.id" :paused="paused" :pnl="m.pnl" :activity="null" @select="company.selected=$event;company.selectedTeam=null"/></TresGroup>
</template>
