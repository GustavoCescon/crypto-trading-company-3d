<script setup lang="ts">
import { onBeforeUnmount, shallowRef, watch } from 'vue'
import { BoxGeometry, CanvasTexture, Color, CylinderGeometry, Group, InstancedMesh, Matrix4, MeshStandardMaterial, Object3D, PlaneGeometry, SRGBColorSpace, Vector3 } from 'three'
type Desk={x:number;z:number;rotation?:number}
const props=defineProps<{desks:Desk[];plants:Desk[]}>()
const kit=shallowRef(new Group())
const reusable:InstancedMesh[]=[]
const helper=new Object3D()
function addInstances(geometry:any,material:any,transforms:Array<{p:[number,number,number];s:[number,number,number];r?:number}>,group:Group){
  if(!transforms.length)return
  const mesh=new InstancedMesh(geometry,material,transforms.length);mesh.frustumCulled=false
  transforms.forEach((t,index)=>{helper.position.set(...t.p);helper.rotation.set(0,t.r??0,0);helper.scale.set(...t.s);helper.updateMatrix();mesh.setMatrixAt(index,helper.matrix)})
  mesh.instanceMatrix.needsUpdate=true;mesh.computeBoundingSphere();group.add(mesh);reusable.push(mesh)
}
const graphCanvas=document.createElement('canvas');graphCanvas.width=256;graphCanvas.height=128
const g=graphCanvas.getContext('2d')!;g.fillStyle='#071d24';g.fillRect(0,0,256,128);g.strokeStyle='#173b3c';g.lineWidth=1
for(let x=32;x<256;x+=32){g.beginPath();g.moveTo(x,0);g.lineTo(x,128);g.stroke()}for(let y=24;y<128;y+=24){g.beginPath();g.moveTo(0,y);g.lineTo(256,y);g.stroke()}
g.strokeStyle='#66d39b';g.lineWidth=3;g.beginPath();[[0,94],[25,81],[48,90],[72,57],[96,70],[118,39],[143,59],[166,43],[189,48],[211,21],[234,33],[256,14]].forEach(([x,y],i)=>i?g.lineTo(x,y):g.moveTo(x,y));g.stroke()
const graph=new CanvasTexture(graphCanvas);graph.colorSpace=SRGBColorSpace
const materials={wood:new MeshStandardMaterial({color:'#e4e0d2',roughness:.83}),edge:new MeshStandardMaterial({color:'#f5f2e8',roughness:.7}),metal:new MeshStandardMaterial({color:'#596873',metalness:.55,roughness:.42}),screen:new MeshStandardMaterial({color:'#b9d4cf',roughness:.55,metalness:.2}),pixels:new MeshStandardMaterial({map:graph,emissive:'#315a4d',emissiveMap:graph,emissiveIntensity:.22}),chair:new MeshStandardMaterial({color:'#263743',roughness:.7}),pot:new MeshStandardMaterial({color:'#b58d6d',roughness:.9}),leaf:new MeshStandardMaterial({color:'#658b65',roughness:.8})}
function rebuild(){
  for(const mesh of reusable){mesh.geometry.dispose();mesh.material instanceof Array?mesh.material.forEach(m=>m.dispose()):mesh.material.dispose()}reusable.length=0;kit.value.clear()
  const d=props.desks,deskTop=new BoxGeometry(3.05,.16,1.32),leg=new BoxGeometry(.1,.78,.1),frame=new BoxGeometry(1.34,.88,.11),plane=new PlaneGeometry(1.17,.67),keyboard=new BoxGeometry(.62,.035,.22),seat=new BoxGeometry(.72,.14,.66),back=new BoxGeometry(.68,.92,.13),post=new CylinderGeometry(.055,.08,.4,8),base=new BoxGeometry(.78,.08,.78)
  addInstances(deskTop,materials.wood,d.map(v=>({p:[v.x,.82,v.z],s:[1,1,1],r:v.rotation})),kit.value)
  addInstances(leg,materials.metal,d.flatMap(v=>[-1,1].flatMap(x=>[-1,1].map(z=>({p:[v.x+x*1.36,.4,v.z+z*.53] as [number,number,number],s:[1,1,1] as [number,number,number],r:v.rotation})))),kit.value)
  addInstances(frame,materials.metal,d.map(v=>({p:[v.x,1.38,v.z-.43],s:[1,1,1],r:v.rotation})),kit.value)
  addInstances(plane,materials.pixels,d.map(v=>({p:[v.x,1.39,v.z-.495],s:[1,1,1],r:v.rotation})),kit.value)
  addInstances(keyboard,materials.screen,d.map(v=>({p:[v.x,.93,v.z+.29],s:[1,1,1],r:v.rotation})),kit.value)
  addInstances(seat,materials.chair,d.map(v=>({p:[v.x,.47,v.z+1.15],s:[1,1,1],r:v.rotation})),kit.value)
  addInstances(back,materials.chair,d.map(v=>({p:[v.x,.91,v.z+1.44],s:[1,1,1],r:v.rotation})),kit.value)
  addInstances(post,materials.metal,d.map(v=>({p:[v.x,.22,v.z+1.15],s:[1,1,1],r:v.rotation})),kit.value)
  addInstances(base,materials.metal,d.map(v=>({p:[v.x,.055,v.z+1.15],s:[1,1,1],r:v.rotation})),kit.value)
  const p=props.plants
  addInstances(new CylinderGeometry(.2,.25,.34,8),materials.pot,p.map(v=>({p:[v.x,.22,v.z],s:[1,1,1]})),kit.value)
  addInstances(new CylinderGeometry(.025,.23,.78,5),materials.leaf,p.map(v=>({p:[v.x,.73,v.z],s:[1,1,1],r:v.rotation})),kit.value)
}
watch(()=>[props.desks,props.plants],rebuild,{deep:true,immediate:true})
onBeforeUnmount(()=>{rebuild();graph.dispose();Object.values(materials).forEach(m=>m.dispose())})
</script>
<template><primitive :object="kit" :dispose="null"/></template>
