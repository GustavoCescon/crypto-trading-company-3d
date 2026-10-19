<script setup lang="ts">
import { CanvasTexture, SRGBColorSpace } from 'three'
import { watch,onBeforeUnmount } from 'vue'
const props=withDefaults(defineProps<{title:string;lines?:string[];color?:string;width?:number;height?:number}>(),{lines:()=>[],color:'#b6e56d',width:3.4,height:1.8})
const canvas=document.createElement('canvas');canvas.width=768;canvas.height=384
const texture=new CanvasTexture(canvas);texture.colorSpace=SRGBColorSpace
function draw(){const c=canvas.getContext('2d')!;c.fillStyle='#142532';c.fillRect(0,0,768,384);c.fillStyle=props.color;c.fillRect(26,28,7,44);c.font='bold 34px sans-serif';c.fillText(props.title,50,61);c.fillStyle='#dce7eb';c.font='28px monospace';props.lines.slice(0,5).forEach((line,i)=>c.fillText(line,32,124+i*49));texture.needsUpdate=true}
watch(()=>[props.title,props.lines,props.color],draw,{immediate:true,deep:true});onBeforeUnmount(()=>texture.dispose())
</script>
<template><TresMesh><TresPlaneGeometry :args="[width,height]"/><TresMeshBasicMaterial :map="texture" :tone-mapped="false"/></TresMesh></template>
