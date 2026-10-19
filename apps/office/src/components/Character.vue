<script setup lang="ts">
import { computed, onBeforeUnmount, shallowRef, watch } from 'vue'
import { useLoop } from '@tresjs/core'
import { useGLTF } from '@tresjs/cientos'
import { CanvasTexture, Group, Sprite, SpriteMaterial, SRGBColorSpace } from 'three'
import type { Agent } from '../types'
const props=defineProps<{agent:Agent;position:[number,number,number];selected:boolean;paused:boolean;pnl:string|null}>()
const emit=defineEmits<{select:[id:string]}>()
const {state:modelAsset}=useGLTF('/models/traders.glb')
const model=shallowRef<Group|null>(null), leftArm=shallowRef<any>(null),rightArm=shallowRef<any>(null),head=shallowRef<any>(null)
const index=[...props.agent.id].reduce((n,c)=>n+c.charCodeAt(0),0)%8
const statusColor=computed(()=>!props.agent.online?'#71808d':['ERROR','BLOCKED'].includes(props.agent.state)?'#fa716c':props.agent.state==='EXECUTING'?'#ffc05a':props.agent.state==='MEETING'?'#b6a1ef':props.agent.state==='ANALYZING'||props.agent.state==='APPROVING'?'#b6e66c':'#60d0c5')
function attach(asset:any){
  const source=asset?.scene?.getObjectByName(`trader-${index}`) as Group|undefined
  if(!source)return
  const clone=source.clone(true);clone.scale.setScalar(1.32);clone.traverse(object=>{if((object as any).isMesh){(object as any).castShadow=false;(object as any).receiveShadow=false}})
  model.value=clone;leftArm.value=clone.getObjectByName('Arm_L')??null;rightArm.value=clone.getObjectByName('Arm_R')??null;head.value=clone.getObjectByName('Body')??null
}
watch(modelAsset,attach,{immediate:true})
useLoop().onBeforeRender(({elapsed})=>{
  if(props.paused||!props.agent.online)return
  const phase=elapsed*5+(index*0.61),working=props.agent.state==='ANALYZING'||props.agent.state==='APPROVING'||props.agent.state==='EXECUTING'
  if(leftArm.value)leftArm.value.rotation.z=working?Math.sin(phase)*0.12+(props.agent.state==='EXECUTING'?0.1:0):props.agent.state==='MEETING'?Math.sin(phase*.45)*0.22:0
  if(rightArm.value)rightArm.value.rotation.z=working?-Math.sin(phase+0.8)*0.12-(props.agent.state==='EXECUTING'?0.1:0):props.agent.state==='MEETING'?-Math.sin(phase*.45)*0.22:0
  if(head.value)head.value.rotation.y=props.agent.state==='ANALYZING'?Math.sin(elapsed*1.3+index)*0.08:0
})
const canvas=document.createElement('canvas');canvas.width=384;canvas.height=128
const texture=new CanvasTexture(canvas);texture.colorSpace=SRGBColorSpace
const badge=new Sprite(new SpriteMaterial({map:texture,transparent:true,depthWrite:false}))
badge.scale.set(2.35,.78,1);badge.position.set(0,3.25,0)
watch(()=>[props.pnl,props.selected,props.agent.state],()=>{
  const context=canvas.getContext('2d')!;context.clearRect(0,0,384,128)
  context.fillStyle=props.selected?'#d6ff9d':props.pnl===null?'#16242deF':Number(props.pnl)>=0?'#0c3728ec':'#491f24ed';context.beginPath();context.roundRect(8,12,368,104,18);context.fill()
  context.fillStyle='#ffffff';context.textAlign='center';context.font='700 44px system-ui';
  context.fillText(props.pnl===null?props.agent.name:(Number(props.pnl)>=0?'+':'')+'$'+Number(props.pnl).toLocaleString('pt-BR',{minimumFractionDigits:2,maximumFractionDigits:2}),192,76)
  texture.needsUpdate=true
},{immediate:true})
onBeforeUnmount(()=>{texture.dispose();badge.material.dispose()})
</script>
<template>
  <TresGroup :position="position" :rotation="[0,Math.PI,0]" @click.stop="emit('select',agent.id)">
    <TresMesh :position="[0,.025,0]" :rotation="[-Math.PI/2,0,0]"><TresRingGeometry :args="[.4,selected ? .59 : .49,24]"/><TresMeshBasicMaterial :color="selected?'#d4fa92':statusColor"/></TresMesh>
    <primitive v-if="model" :object="model"/>
    <TresMesh :position="[0,2.7,0]"><TresSphereGeometry :args="[.08,8,6]"/><TresMeshBasicMaterial :color="statusColor"/></TresMesh>
    <primitive v-if="props.pnl!==null" :object="badge"/>
  </TresGroup>
</template>
