<script setup lang="ts">
import { onBeforeUnmount, shallowRef, watch } from 'vue'
import { BoxGeometry, CanvasTexture, CylinderGeometry, Group, InstancedMesh, MeshStandardMaterial, Object3D, PlaneGeometry, SphereGeometry, SRGBColorSpace } from 'three'
import { RoundedBoxGeometry } from 'three/examples/jsm/geometries/RoundedBoxGeometry.js'
type Point={x:number;z:number;rotation?:number}
const props=defineProps<{desks:Point[];plants:Point[]}>()
const kit=shallowRef(new Group()),meshes:InstancedMesh[]=[]
const helper=new Object3D()
function instances(geometry:any,material:any,items:Array<{p:[number,number,number];s:[number,number,number];r?:number}>) {if(!items.length)return;const mesh=new InstancedMesh(geometry,material,items.length);mesh.frustumCulled=false;items.forEach((item,i)=>{helper.position.set(...item.p);helper.rotation.set(0,item.r??0,0);helper.scale.set(...item.s);helper.updateMatrix();mesh.setMatrixAt(i,helper.matrix)});mesh.instanceMatrix.needsUpdate=true;mesh.computeBoundingSphere();kit.value.add(mesh);meshes.push(mesh)}
const chartCanvas=document.createElement('canvas');chartCanvas.width=768;chartCanvas.height=384
const c=chartCanvas.getContext('2d')!;c.fillStyle='#111c20';c.fillRect(0,0,768,384);c.strokeStyle='#27383a';c.lineWidth=1
for(let x=0;x<768;x+=96){c.beginPath();c.moveTo(x,0);c.lineTo(x,384);c.stroke()}for(let y=0;y<384;y+=64){c.beginPath();c.moveTo(0,y);c.lineTo(768,y);c.stroke()}
c.strokeStyle='#64ad88';c.lineWidth=5;c.beginPath();[[0,280],[65,246],[130,258],[195,188],[260,224],[325,132],[390,157],[455,108],[520,145],[585,82],[650,118],[715,51],[768,68]].forEach(([x,y],i)=>i?c.lineTo(x,y):c.moveTo(x,y));c.stroke()
const chart=new CanvasTexture(chartCanvas);chart.colorSpace=SRGBColorSpace
const m={walnut:new MeshStandardMaterial({color:'#d3d0c6',roughness:.48}),edge:new MeshStandardMaterial({color:'#f0eee7',roughness:.42}),steel:new MeshStandardMaterial({color:'#3d4442',metalness:.62,roughness:.32}),bezel:new MeshStandardMaterial({color:'#202624',metalness:.2,roughness:.37}),display:new MeshStandardMaterial({map:chart,emissiveMap:chart,emissive:'#40654f',emissiveIntensity:.22,roughness:.36}),keyboard:new MeshStandardMaterial({color:'#d2d0c5',roughness:.48}),chair:new MeshStandardMaterial({color:'#39423f',roughness:.76}),pot:new MeshStandardMaterial({color:'#8f7860',roughness:.78}),leaf:new MeshStandardMaterial({color:'#536c50',roughness:.76})}
function rebuild(){for(const mesh of meshes){kit.value.remove(mesh);mesh.geometry.dispose();mesh.dispose()}meshes.length=0;kit.value.clear();const d=props.desks
 const top=new RoundedBoxGeometry(3.05,.14,1.32,4,.055),leg=new BoxGeometry(.075,.76,.075),apron=new BoxGeometry(2.45,.16,.1),monitor=new RoundedBoxGeometry(1.02,.9,.12,4,.035),panel=new PlaneGeometry(.9,.7),stem=new CylinderGeometry(.045,.065,.22,12),foot=new BoxGeometry(.38,.06,.18),keyboard=new RoundedBoxGeometry(.65,.035,.22,3,.012),mouse=new SphereGeometry(.09,12,8),seat=new RoundedBoxGeometry(.76,.16,.68,4,.07),back=new RoundedBoxGeometry(.72,.84,.15,4,.07),chairPost=new CylinderGeometry(.055,.075,.42,12),base=new CylinderGeometry(.13,.13,.08,16),spoke=new BoxGeometry(.72,.055,.07),wheel=new SphereGeometry(.085,10,8)
 instances(top,m.walnut,d.map(v=>({p:[v.x,.8,v.z],s:[1,1,1],r:v.rotation})))
 instances(new BoxGeometry(3.07,.035,1.34),m.edge,d.map(v=>({p:[v.x,.715,v.z],s:[1,1,1],r:v.rotation})))
 instances(leg,m.steel,d.flatMap(v=>[-1,1].flatMap(x=>[-1,1].map(z=>({p:[v.x+x*1.34,.38,v.z+z*.54] as [number,number,number],s:[1,1,1] as [number,number,number],r:v.rotation})))) )
 instances(apron,m.steel,d.map(v=>({p:[v.x,.59,v.z-.46],s:[1,1,1],r:v.rotation})))
 const screens=d.flatMap(v=>[-.52,.52].map(offset=>({p:[v.x+offset,1.42,v.z-.47] as [number,number,number],s:[1,1,1] as [number,number,number],r:v.rotation})))
 instances(monitor,m.bezel,screens)
 instances(panel,m.display,screens.map(v=>({p:[v.p[0],v.p[1],v.p[2]+.065] as [number,number,number],s:[1,1,1] as [number,number,number],r:v.r})))
 instances(stem,m.steel,screens.map(v=>({p:[v.p[0],1.02,v.p[2]+.01] as [number,number,number],s:[1,1,1] as [number,number,number],r:v.r})))
 instances(foot,m.steel,screens.map(v=>({p:[v.p[0],.92,v.p[2]+.01] as [number,number,number],s:[1,1,1] as [number,number,number],r:v.r})))
 instances(keyboard,m.keyboard,d.map(v=>({p:[v.x,.9,v.z+.24],s:[1,1,1],r:v.rotation})))
 instances(mouse,m.keyboard,d.map(v=>({p:[v.x+.52,.91,v.z+.23],s:[1,.35,1],r:v.rotation})))
 instances(seat,m.chair,d.map(v=>({p:[v.x,.49,v.z+1.42],s:[1,1,1],r:v.rotation})))
 instances(back,m.chair,d.map(v=>({p:[v.x,.95,v.z+1.76],s:[1,1,1],r:v.rotation})))
 instances(chairPost,m.steel,d.map(v=>({p:[v.x,.22,v.z+1.42],s:[1,1,1],r:v.rotation})))
 instances(base,m.steel,d.map(v=>({p:[v.x,.055,v.z+1.42],s:[1,1,1],r:v.rotation})))
 instances(spoke,m.steel,d.flatMap(v=>[0,Math.PI/3,-Math.PI/3,2*Math.PI/3,-2*Math.PI/3].map(r=>({p:[v.x,.07,v.z+1.42] as [number,number,number],s:[1,1,1] as [number,number,number],r}))))
 instances(wheel,m.steel,d.flatMap(v=>[0,Math.PI/3,-Math.PI/3,2*Math.PI/3,-2*Math.PI/3].map(a=>({p:[v.x+Math.cos(a)*.36,.07,v.z+1.42+Math.sin(a)*.36] as [number,number,number],s:[1,1,1] as [number,number,number]}))))
 const pots=props.plants
 instances(new CylinderGeometry(.18,.22,.3,16),m.pot,pots.map(v=>({p:[v.x,.19,v.z],s:[1,1,1],r:v.rotation})))
 instances(new CylinderGeometry(.025,.19,.74,8),m.leaf,pots.map(v=>({p:[v.x,.68,v.z],s:[1,1,1],r:v.rotation})))
 instances(new SphereGeometry(.22,12,8),m.leaf,pots.map(v=>({p:[v.x,.96,v.z],s:[1.05,1.35,1.05],r:v.rotation})))
}
watch(()=>[props.desks,props.plants],rebuild,{deep:true,immediate:true})
onBeforeUnmount(()=>{for(const mesh of meshes){kit.value.remove(mesh);mesh.geometry.dispose();mesh.dispose()}chart.dispose();Object.values(m).forEach(material=>material.dispose())})
</script>
<template><primitive :object="kit" :dispose="null"/></template>
