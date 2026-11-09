import React,{useEffect,useRef,useState} from 'react'
import {invoke} from '@tauri-apps/api/core'
import {Volume2,VolumeX} from 'lucide-react'

export function VolumeControl({native,open,setOpen}){
 const [volume,setVolume]=useState(native?null:{level:62,muted:false})
 const [error,setError]=useState('')
 const root=useRef(null),button=useRef(null)
 const pending=useRef(null),writing=useRef(false),revision=useRef(0)

 useEffect(()=>{
  if(!open)return
  const outside=event=>{if(!root.current?.contains(event.target))setOpen(false)}
  const escape=event=>{
   if(event.key!=='Escape')return
   event.preventDefault()
   setOpen(false)
   button.current?.focus()
  }
  document.addEventListener('pointerdown',outside)
  root.current?.addEventListener('keydown',escape)
  const container=root.current
  return()=>{
   document.removeEventListener('pointerdown',outside)
   container?.removeEventListener('keydown',escape)
  }
 },[open,setOpen])

 useEffect(()=>{
  if(!open||!native)return
  let active=true,reading=false
  const refresh=async()=>{
   if(reading||writing.current)return
   reading=true
   const ticket=revision.current
   try{
    const next=await invoke('system_volume')
    if(active&&ticket===revision.current){setVolume(next);setError('')}
   }catch{
    if(active&&ticket===revision.current){setVolume(null);setError('Salida de audio no disponible.')}
   }finally{reading=false}
  }
  refresh()
  const timer=setInterval(refresh,850)
  return()=>{active=false;clearInterval(timer)}
 },[native,open])

 const change=async update=>{
  revision.current++
  setVolume(value=>({...value,...update}))
  if(!native)return
  // Keep the last requested value while Windows applies the current one.
  // Merge mute and level independently so a quick mute cannot drop a drag.
  pending.current={...pending.current,...update}
  if(writing.current)return
  writing.current=true
  try{
   while(pending.current){
    const next=pending.current
    pending.current=null
    let actual
    if(next.level!=null)actual=await invoke('system_volume_set',{level:next.level,muted:null})
    if(next.muted!=null)actual=await invoke('system_volume_set',{level:null,muted:next.muted})
    if(!pending.current)setVolume(actual)
   }
   setError('')
  }catch{
   pending.current=null
   setVolume(null)
   setError('No se pudo ajustar el volumen.')
  }finally{writing.current=false}
 }

 const Speaker=volume?.muted||volume?.level===0?VolumeX:Volume2
 return <div className="volume-control" ref={root}>
  <button type="button" ref={button} className="volume-toggle" aria-label="Volumen de Windows" title="Volumen de Windows" aria-expanded={open} aria-controls={open?'bimu-volume-panel':undefined} onClick={()=>setOpen(!open)}><Speaker size={16}/></button>
  {open&&<section className="volume-popover" id="bimu-volume-panel" aria-label="Volumen de Windows">
   <strong>Volumen de Windows</strong>
   <div className="volume-row">
    <button type="button" disabled={!volume} aria-label={volume?.muted?'Quitar silencio':'Silenciar'} title={volume?.muted?'Quitar silencio':'Silenciar'} onClick={()=>change({muted:!volume.muted})}>{volume?.muted?<VolumeX size={14}/>:<Volume2 size={14}/>}</button>
    <input type="range" min="0" max="100" style={{'--volume-fill':`${volume?.level??0}%`}} value={volume?.level??0} disabled={!volume} aria-label="Nivel de volumen de Windows" aria-valuetext={volume?`${volume.level} por ciento`:undefined} onChange={event=>change({level:Number(event.target.value)})}/>
    <span>{volume?`${volume.level} %`:'—'}</span>
   </div>
   {!volume&&<small role="status">{error||'Consultando…'}</small>}
  </section>}
 </div>
}
