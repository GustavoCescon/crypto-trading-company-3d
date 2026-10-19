import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { isSnapshot, type Snapshot } from './types'
export const useCompany=defineStore('company',()=>{
  const data=ref<Snapshot|null>(null), connected=ref(false), error=ref(''), selected=ref<string|null>(null), selectedTeam=ref<string|null>(null), lastReceived=ref(0)
  let socket:WebSocket|undefined, retry:ReturnType<typeof setTimeout>|undefined, poll:ReturnType<typeof setInterval>|undefined, refreshTimer:ReturnType<typeof setTimeout>|undefined, stopped=false
  function accept(value:unknown){if(isSnapshot(value)&&(!data.value||value.cursor>=data.value.cursor)){data.value=value;lastReceived.value=Date.now();error.value=''}}
  async function refresh(){try{const response=await fetch('/api/v1/snapshot');if(!response.ok)throw new Error('Backend indisponível');accept(await response.json())}catch(e){error.value=String(e)}}
  function connect(){
    if(stopped)return
    socket=new WebSocket(`${location.protocol==='https:'?'wss':'ws'}://${location.host}/api/v1/ws`)
    socket.onopen=()=>{connected.value=true;error.value=''}
    socket.onmessage=e=>{try{const message=JSON.parse(e.data);if(message?.type==='events'){if(message.events?.length){clearTimeout(refreshTimer);refreshTimer=setTimeout(refresh,250)}}else accept(message)}catch{error.value='Evento inválido'}}
    socket.onclose=()=>{connected.value=false;if(!stopped)retry=setTimeout(connect,3000)}
    socket.onerror=()=>{error.value='Reconectando ao backend';socket?.close()}
  }
  function start(){stopped=false;refresh();connect();poll=setInterval(refresh,10000)}
  function stop(){stopped=true;socket?.close();clearTimeout(retry);clearTimeout(refreshTimer);clearInterval(poll)}
  const agent=computed(()=>data.value?.agents.find(a=>a.id===selected.value))
  const online=computed(()=>data.value?.agents.filter(a=>a.online).length??0)
  return{data,connected,error,selected,selectedTeam,agent,online,lastReceived,start,stop,refresh}
})
