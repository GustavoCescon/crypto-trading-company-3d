<script setup lang="ts">
import { computed, onBeforeUnmount, shallowRef, watch } from 'vue'
import { useLoop } from '@tresjs/core'
import { useGLTF } from '@tresjs/cientos'
import { Box3, CanvasTexture, Group, Sprite, SpriteMaterial, SRGBColorSpace, Vector3 } from 'three'
import type { Agent } from '../types'
const props=defineProps<{agent:Agent;position:[number,number,number];route:[number,number,number][];bubble:string;selected:boolean;paused:boolean;pnl:string|null;activity:string|null}>()
const emit=defineEmits<{select:[id:string]}>()
const {state:modelAsset}=useGLTF('/models/traders.glb')
const model=shallowRef<Group|null>(null),actor=shallowRef<any>(null), leftArm=shallowRef<any>(null),rightArm=shallowRef<any>(null),leftLeg=shallowRef<any>(null),rightLeg=shallowRef<any>(null),head=shallowRef<any>(null)
const index=[...props.agent.id].reduce((n,c)=>n+c.charCodeAt(0),0)%8
let routeSignature='',routePoints:Vector3[]=[],routeLength=0,routeStarted=0,initialized=false,lastFrame=0
const statusColor=computed(()=>!props.agent.online?'#71808d':['ERROR','BLOCKED'].includes(props.agent.state)?'#fa716c':props.agent.state==='EXECUTING'?'#ffc05a':props.agent.state==='MEETING'?'#b6a1ef':props.agent.state==='ANALYZING'||props.agent.state==='APPROVING'?'#b6e66c':'#60d0c5')
function attach(asset:any){
  const source=asset?.scene?.getObjectByName(`trader-${index}`) as Group|undefined
  if(!source)return
  const clone=source.clone(true);clone.scale.setScalar(1.32);clone.traverse(object=>{if((object as any).isMesh){(object as any).castShadow=true;(object as any).receiveShadow=true}});clone.updateMatrixWorld(true);const bounds=new Box3().setFromObject(clone);clone.position.y-=bounds.min.y
  model.value=clone;leftArm.value=clone.getObjectByName('Arm_L')??null;rightArm.value=clone.getObjectByName('Arm_R')??null;leftLeg.value=clone.getObjectByName('Leg_L')??null;rightLeg.value=clone.getObjectByName('Leg_R')??null;head.value=clone.getObjectByName('Body')??null
}
watch(modelAsset,attach,{immediate:true})
useLoop().onBeforeRender(({elapsed})=>{
  const frameDelta=Math.max(0,elapsed-lastFrame);lastFrame=elapsed
  if((props.paused||!props.agent.online)&&routePoints.length>1)routeStarted+=frameDelta
  const phase=elapsed*5+(index*0.61),signature=props.route.map(point=>point.join(',')).join('|')
  if(actor.value&&signature&&signature!==routeSignature){routeSignature=signature;const points=props.route.map(point=>new Vector3(...point));if(!initialized){actor.value.position.copy(points[0]);initialized=true;routePoints=points.slice(1)}else routePoints=points;routeLength=0;for(let i=1;i<routePoints.length;i++)routeLength+=routePoints[i-1].distanceTo(routePoints[i]);if(initialized&&routePoints.length){routePoints.unshift(actor.value.position.clone());routeLength=0;for(let i=1;i<routePoints.length;i++)routeLength+=routePoints[i-1].distanceTo(routePoints[i])}routeStarted=elapsed}
  const walking=routePoints.length>1&&routeLength>.01
  if(walking&&!props.paused&&props.agent.online){const progress=Math.min(1,(elapsed-routeStarted)/Math.max(.9,routeLength/5.5)),distance=progress*routeLength;let covered=0;for(let i=1;i<routePoints.length;i++){const length=routePoints[i-1].distanceTo(routePoints[i]);if(distance<=covered+length||i===routePoints.length-1){const t=length?Math.min(1,(distance-covered)/length):1;actor.value.position.lerpVectors(routePoints[i-1],routePoints[i],t);actor.value.position.y=.045;const delta=routePoints[i].clone().sub(routePoints[i-1]);actor.value.rotation.y=Math.atan2(delta.x,delta.z)+Math.PI;break}covered+=length}if(progress>=1)routePoints=[]}
  else if(actor.value&&!walking&&!props.paused&&props.agent.online){const roam=.1;actor.value.position.set(props.position[0]+Math.sin(elapsed*.55+index)*roam,.045,props.position[2]+Math.sin(elapsed*.37+index*1.7)*roam);actor.value.rotation.y=0}
  if(props.paused||!props.agent.online)return
  const working=props.activity?false:props.agent.state==='ANALYZING'||props.agent.state==='APPROVING'||props.agent.state==='EXECUTING',swing=walking?Math.sin(phase)*.34:0
  if(leftArm.value)leftArm.value.rotation.z=walking?swing:working?Math.sin(phase)*0.12+(props.agent.state==='EXECUTING'?0.1:0):props.agent.state==='MEETING'?Math.sin(phase*.45)*0.22:0
  if(rightArm.value)rightArm.value.rotation.z=walking?-swing:working?-Math.sin(phase+0.8)*0.12-(props.agent.state==='EXECUTING'?0.1:0):props.agent.state==='MEETING'?-Math.sin(phase*.45)*0.22:0
  if((props.activity||walking)&&leftLeg.value)leftLeg.value.rotation.x=Math.sin(phase)*.22
  if((props.activity||walking)&&rightLeg.value)rightLeg.value.rotation.x=-Math.sin(phase)*.22
  if(!props.activity){if(leftArm.value)leftArm.value.rotation.x=0;if(rightArm.value)rightArm.value.rotation.x=0;if(leftLeg.value)leftLeg.value.rotation.x=0;if(rightLeg.value)rightLeg.value.rotation.x=0}
  if(head.value)head.value.rotation.y=props.agent.state==='ANALYZING'?Math.sin(elapsed*1.3+index)*0.08:0
})
const canvas=document.createElement('canvas');canvas.width=1280;canvas.height=320
const texture=new CanvasTexture(canvas);texture.colorSpace=SRGBColorSpace
const badge=new Sprite(new SpriteMaterial({map:texture,transparent:true,depthWrite:false}))
badge.scale.set(5.8,1.55,1);badge.position.set(0,3.4,0)
const nameCanvas=document.createElement('canvas');nameCanvas.width=768;nameCanvas.height=192
const nameTexture=new CanvasTexture(nameCanvas);nameTexture.colorSpace=SRGBColorSpace
const nameplate=new Sprite(new SpriteMaterial({map:nameTexture,transparent:true,depthWrite:false}))
nameplate.scale.set(3.5,.88,1);nameplate.position.set(0,2.92,0)
const showBubble=computed(()=>props.selected||props.activity!==null||['APPROVING','EXECUTING','MEETING','BLOCKED','ERROR'].includes(props.agent.state)||props.pnl!==null)
watch(()=>[props.agent.name,props.agent.state,props.agent.online],()=>{
  const context=nameCanvas.getContext('2d')!;context.clearRect(0,0,768,192)
  context.shadowColor='#071116';context.shadowBlur=16;context.fillStyle='rgba(13,25,28,.92)';context.beginPath();context.roundRect(20,25,728,142,28);context.fill();context.shadowBlur=0
  context.fillStyle=statusColor.value;context.beginPath();context.arc(72,96,15,0,Math.PI*2);context.fill()
  context.fillStyle='#f5f7ee';context.textAlign='left';context.textBaseline='middle';context.font='700 46px system-ui';context.fillText(props.agent.name,110,96,590)
  nameTexture.needsUpdate=true
},{immediate:true})
watch(()=>[props.pnl,props.selected,props.agent.state,props.activity,props.bubble],()=>{
  const context=canvas.getContext('2d')!;context.setTransform(1280/384,0,0,2.5,0,0);context.clearRect(0,0,384,128)
  context.shadowColor='#071116';context.shadowBlur=12;context.fillStyle='#f5f7ee';context.beginPath();context.roundRect(7,8,370,112,18);context.fill();context.shadowBlur=0
  context.fillStyle=props.selected?'#32733f':props.activity?'#24654c':['BLOCKED','ERROR'].includes(props.agent.state)?'#aa463f':'#157a73';context.beginPath();context.roundRect(7,8,12,112,6);context.fill()
  context.fillStyle='#172b2a';context.textAlign='center';context.font='700 48px system-ui';context.fillText(props.bubble,198,58,344)
  if(props.pnl!==null&&!props.activity){context.fillStyle=Number(props.pnl)>=0?'#26713e':'#b93c35';context.font='700 42px system-ui';context.fillText(`P&L ${Number(props.pnl)>=0?'+':''}R$ ${Number(props.pnl).toLocaleString('pt-BR',{minimumFractionDigits:2,maximumFractionDigits:2})}`,198,108,344)}
  texture.needsUpdate=true
},{immediate:true})
onBeforeUnmount(()=>{texture.dispose();badge.material.dispose();nameTexture.dispose();nameplate.material.dispose()})
</script>
<template>
  <TresGroup ref="actor" :position="position" :rotation="[0,0,0]" @click.stop="emit('select',agent.id)">
    <TresMesh :position="[0,.025,0]" :rotation="[-Math.PI/2,0,0]"><TresRingGeometry :args="[.4,selected ? .59 : .49,24]"/><TresMeshBasicMaterial :color="selected?'#d4fa92':statusColor"/></TresMesh>
    <primitive v-if="model" :object="model"/>
    <TresMesh :position="[0,2.7,0]"><TresSphereGeometry :args="[.08,8,6]"/><TresMeshBasicMaterial :color="statusColor"/></TresMesh>
    <primitive :object="nameplate"/>
    <primitive v-if="showBubble" :object="badge"/>
  </TresGroup>
</template>
