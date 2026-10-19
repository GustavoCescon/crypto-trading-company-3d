<script setup lang="ts">
import { computed,shallowRef } from 'vue'
import { OrbitControls } from '@tresjs/cientos'
import { useLoop } from '@tresjs/core'
import { Vector3, type Camera } from 'three'
import { useCompany } from '../store'
import Character from './Character.vue'
import FloorKit from './FloorKit.vue'
import Screen from './Screen.vue'
const props=defineProps<{paused:boolean;showPnl:boolean}>()
const company=useCompany(), camera=shallowRef<any>(), controls=shallowRef<any>()
type Department={id:string;name:string;x:number;z:number;w:number;d:number;color:string;floor:string;room:string}
const palette={market:['#54b5a6','#182c31'],technical:['#7296bd','#1c2a37'],news:['#cfaa64','#332c29'],committee:['#a394cc','#282638'],risk:['#d37468','#34272d'],board:['#70a3bc','#1c303b'],cafe:['#7caa73','#26322c'],floor:['#a2c964','#142329']}
const labels:Record<string,string>={market:'02 · INTELIGÊNCIA DE MERCADO',technical:'03 · ANÁLISE TÉCNICA',news:'04 · NOTÍCIAS & IA',committee:'05 · COMITÊ DE TRADING',risk:'06 · GESTÃO DE RISCO',board:'07 · DIRETORIA & AUDITORIA',cafe:'08 · CAFÉ / DESCANSO',floor:'01 · TRADING FLOOR'}
function departmentForAgent(agent:{id:string;room:string}){return agent.id.startsWith('analyst_')?'floor':agent.room==='audit'?'board':agent.room==='execution'?'floor':agent.room}
const departments=computed<Department[]>(()=>{
  const agents=company.data?.agents??[],count=(id:string)=>Math.max(id==='cafe'?0:1,agents.filter(a=>departmentForAgent(a)===id).length)
  const sizing=(id:string,w:number)=>{const n=count(id),cols=Math.min(4,Math.max(1,Math.ceil(Math.sqrt(n*1.5)))),rows=Math.ceil(n/cols);return{w:Math.max(w,cols*3.8+2),d:Math.max(13,rows*3.2+3),cols,rows}}
  const top=['market','technical','news','committee'],bottom=['risk','board','cafe'],topSize=top.map(id=>sizing(id,18)),bottomSize=bottom.map(id=>sizing(id,20)),topDepth=Math.max(...topSize.map(x=>x.d)),bottomDepth=Math.max(...bottomSize.map(x=>x.d)),cols=Math.min(6,Math.max(1,Math.ceil(Math.sqrt(count('floor')*1.5)))),floorDepth=Math.max(19,Math.ceil(count('floor')/cols)*6+4)
  const defs:Department[]=[];top.forEach((id,i)=>{const size=topSize[i],color=palette[id as keyof typeof palette];defs.push({id,name:labels[id],x:(i-1.5)*20,z:-floorDepth/2-topDepth/2-1,w:size.w,d:size.d,color:color[0],floor:color[1],room:id})})
  const floor=palette.floor;defs.push({id:'floor',name:labels.floor,x:0,z:0,w:Math.max(58,topSize.reduce((n,s)=>n+s.w,0)-4),d:floorDepth,color:floor[0],floor:floor[1],room:'floor'})
  bottom.forEach((id,i)=>{const size=bottomSize[i],color=palette[id as keyof typeof palette];defs.push({id,name:labels[id],x:(i-1)*25,z:floorDepth/2+bottomDepth/2+1,w:size.w,d:size.d,color:color[0],floor:color[1],room:id})})
  return defs
})
const worldSize=computed(()=>({width:Math.max(...departments.value.map(r=>r.x+r.w/2))-Math.min(...departments.value.map(r=>r.x-r.w/2))+6,depth:Math.max(...departments.value.map(r=>r.z+r.d/2))-Math.min(...departments.value.map(r=>r.z-r.d/2))+6}))
const viewportAspect=computed(()=>window.innerWidth<850 ? .95 : 1.46)
const viewSpan=computed(()=>Math.max(90,(worldSize.value.width+worldSize.value.depth)*.67))
const layout=computed(()=>{
  const result:Array<{agent:any;department:Department;position:[number,number,number];desk:{x:number;z:number;rotation?:number};pnl:string|null}>=[]
  for(const room of departments.value){const staff=(company.data?.agents??[]).filter(a=>departmentForAgent(a)===room.id),cols=Math.min(room.id==='floor'?6:4,Math.max(1,Math.ceil(Math.sqrt(staff.length*1.5)))),rows=Math.ceil(staff.length/cols)
    staff.forEach((agent,index)=>{const col=index%cols,row=Math.floor(index/cols),x=room.x+(col-(Math.min(cols,staff.length)-1)/2)*(room.id==='floor'?Math.min(9,(room.w-7)/Math.max(1,cols-1)):Math.min(4.2,(room.w-3)/Math.max(1,cols-1))),z=room.z+(row-(rows-1)/2)*(room.id==='floor'?Math.min(6,(room.d-3)/Math.max(1,rows-1)):Math.min(3.35,(room.d-3)/Math.max(1,rows-1)));const team=company.data?.teams.find(t=>t.strategy.toLowerCase()===agent.id.toLowerCase());result.push({agent,department:room,desk:{x,z},position:[x,.045,z+1.22],pnl:props.showPnl&&team?team.realized_pnl:null})})
  }
  return result
})
const desks=computed(()=>layout.value.map(m=>m.desk))
const plants=computed(()=>{const r: {x:number;z:number}[]=[];for(const room of departments.value){r.push({x:room.x-room.w/2+1,z:room.z-room.d/2+1},{x:room.x+room.w/2-1,z:room.z-room.d/2+1},{x:room.x-room.w/2+1,z:room.z+room.d/2-1},{x:room.x+room.w/2-1,z:room.z+room.d/2-1})}return r})
const rankingLines=computed(()=>company.data?.ranking.slice(0,5).map(r=>`${r.symbol.padEnd(11)} ${Number(r.change)>=0?'+':''}${Number(r.change).toFixed(2)}%`))
const accountLines=computed(()=>{const a=company.data?.account;return a?[`SALDO  $${Number(a.cash).toLocaleString('pt-BR',{maximumFractionDigits:0})}`,`P&L SIM  $${(Number(a.realized_pnl)+Number(a.unrealized_pnl)).toLocaleString('pt-BR',{minimumFractionDigits:2,maximumFractionDigits:2})}`,`${company.data?.positions.length??0} POSIÇÕES ABERTAS`]:['AGUARDANDO API']})
const roomPnl=computed(()=>Object.fromEntries((company.data?.teams??[]).map(t=>[t.strategy,t.realized_pnl])))
const desiredTarget=new Vector3(),desiredPosition=new Vector3();let focusPending=false
function focus(room:Department){company.selected=null;company.selectedTeam=room.id;const orbit=controls.value?.instance??controls.value,cam=(camera.value?.instance??camera.value) as Camera|undefined;if(!orbit||!cam)return;const from=orbit.target.clone(),to=new Vector3(room.x,0,room.z),offset=cam.position.clone().sub(from);desiredTarget.copy(to);desiredPosition.copy(to).add(offset.multiplyScalar(.62));desiredPosition.y=Math.max(28,cam.position.y*.62);focusPending=true}
useLoop().onBeforeRender(()=>{const orbit=controls.value?.instance??controls.value,cam=(camera.value?.instance??camera.value) as any;if(!orbit||!cam||!focusPending)return;orbit.target.lerp(desiredTarget,.045);cam.position.lerp(desiredPosition,.045);orbit.update();if(cam.position.distanceTo(desiredPosition)<.08)focusPending=false})
</script>
<template>
  <TresOrthographicCamera ref="camera" :position="[55,62,58]" :left="-viewSpan/2" :right="viewSpan/2" :top="viewSpan/(2*viewportAspect)" :bottom="-viewSpan/(2*viewportAspect)" :near=".1" :far="320" :zoom="1"/>
  <OrbitControls ref="controls" :target="[0,0,0]" :min-distance="24" :max-distance="120" :max-polar-angle="Math.PI/2.15" :min-polar-angle=".35" enable-damping :damping-factor=".09"/>
  <TresAmbientLight :intensity=".62"/><TresHemisphereLight color="#d9edff" ground-color="#172328" :intensity=".48"/>
  <TresDirectionalLight :position="[-30,45,22]" :intensity="1.15"/>
  <TresMesh :position="[0,-.48,0]" receive-shadow><TresBoxGeometry :args="[worldSize.width,.95,worldSize.depth]"/><TresMeshStandardMaterial color="#111b21" :roughness=".94"/></TresMesh>
  <TresMesh :position="[0,-.99,0]"><TresBoxGeometry :args="[worldSize.width+2,.12,worldSize.depth+2]"/><TresMeshStandardMaterial color="#0c1419" :roughness=".9"/></TresMesh>
  <TresGroup v-for="room in departments" :key="room.id">
    <TresMesh :position="[room.x,.015,room.z]" @click.stop="focus(room)"><TresBoxGeometry :args="[room.w-.25,.06,room.d-.25]"/><TresMeshStandardMaterial :color="room.floor" :roughness=".82"/></TresMesh>
    <TresMesh :position="[room.x,.08,room.z-room.d/2+.14]"><TresBoxGeometry :args="[room.w-.7,.1,.25]"/><TresMeshStandardMaterial :color="room.color" :emissive="room.color" :emissive-intensity=".16"/></TresMesh>
    <TresMesh :position="[room.x,.92,room.z-room.d/2]" @click.stop="focus(room)"><TresBoxGeometry :args="[room.w-.3,1.65,.12]"/><TresMeshStandardMaterial color="#aec0bf" :transparent="true" :opacity=".22" :metalness=".45" :roughness=".28"/></TresMesh>
    <Screen :position="[room.x,1.9,room.z-room.d/2+.25]" :title="room.name" :lines="room.id==='market'?rankingLines:room.id==='board'?accountLines:[]" :width="Math.min(room.w-2,15)" :height="1.8" :color="room.color"/>
    <TresMesh :position="[room.x,2.84,room.z-room.d/2+.2]"><TresBoxGeometry :args="[Math.min(room.w-1,16),.06,.13]"/><TresMeshStandardMaterial :color="room.color" :emissive="room.color" :emissive-intensity=".7"/></TresMesh>
    <TresMesh :position="[room.x,.001,room.z]" @click.stop="focus(room)"><TresBoxGeometry :args="[room.w,.02,room.d]"/><TresMeshBasicMaterial :transparent="true" :opacity="0" :depth-write="false"/></TresMesh>
  </TresGroup>
  <FloorKit :desks="desks" :plants="plants"/>
  <TresGroup v-for="m in layout" :key="m.agent.id">
    <Character :agent="m.agent" :position="m.position" :selected="company.selected===m.agent.id" :paused="paused" :pnl="m.pnl" @select="company.selected=$event;company.selectedTeam=null"/>
  </TresGroup>
  <TresGroup :position="[0,.2,0]" @click.stop="focus(departments.find(d=>d.id==='floor')!)">
    <Screen :position="[0,4,-8.8]" title="TRADING FLOOR  /  SIMULATION" :lines="accountLines" :width="16" :height="4.2" color="#b6e66c"/>
  </TresGroup>
  <Screen :position="[-20,2.1,0]" title="LIVE MARKET / BINANCE" :lines="rankingLines" :width="8.4" :height="3.4" color="#58d1ae"/>
</template>
