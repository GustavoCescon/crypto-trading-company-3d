<script setup lang="ts">
import { CanvasTexture, SRGBColorSpace } from 'three'
import { watch,onBeforeUnmount } from 'vue'
const props=withDefaults(defineProps<{title:string;lines?:string[];color?:string;width?:number;height?:number}>(),{lines:()=>[],color:'#b6e56d',width:3.4,height:1.8})
const canvas=document.createElement('canvas');canvas.width=1024;canvas.height=384
const texture=new CanvasTexture(canvas);texture.colorSpace=SRGBColorSpace
function draw(){const c=canvas.getContext('2d')!;c.fillStyle='#14201f';c.fillRect(0,0,1024,384);c.fillStyle=props.color;c.fillRect(28,26,12,64);c.font='bold 88px sans-serif';c.fillText(props.title,64,94,920);c.fillStyle='#e5ebe3';c.font='58px monospace';props.lines.slice(0,5).forEach((line,i)=>c.fillText(line,44,154+i*44,930));texture.needsUpdate=true}
watch(()=>[props.title,props.lines,props.color],draw,{immediate:true,deep:true});onBeforeUnmount(()=>texture.dispose())
</script>
<template><TresMesh><TresPlaneGeometry :args="[width,height]"/><TresMeshBasicMaterial :map="texture" :tone-mapped="false"/></TresMesh></template>
