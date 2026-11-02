import React,{useEffect,useRef,useState} from 'react'
import {invoke,isTauri} from '@tauri-apps/api/core'
import {AppWindow} from 'lucide-react'

// Bounded shared cache, in-flight deduplication and two workers. Offscreen rows
// never ask native Windows for an icon; abandoned queued jobs are discarded.
const cache=new Map(),pending=new Map(),queue=[]
let running=0
function pump(){
 while(running<2&&queue.length){
  const job=queue.shift()
  if(!job.listeners.size){pending.delete(job.key);continue}
  running++
  Promise.resolve().then(()=>invoke('apps_icon',{id:job.id})).then(value=>{
   const image=typeof value==='string'&&value.startsWith('data:image/png;base64,')&&value.length<=24000?value:null
   cache.delete(job.key);cache.set(job.key,image)
   while(cache.size>128)cache.delete(cache.keys().next().value)
   job.listeners.forEach(fn=>fn(image))
  }).catch(()=>{cache.set(job.key,null);while(cache.size>128)cache.delete(cache.keys().next().value)}).finally(()=>{pending.delete(job.key);running--;pump()})
 }
}
function subscribe(key,id,listener){
 if(cache.has(key)){listener(cache.get(key));return()=>{}}
 let job=pending.get(key)
 if(!job){job={key,id,listeners:new Set()};pending.set(key,job);queue.push(job)}
 job.listeners.add(listener);pump()
 return()=>job.listeners.delete(listener)
}
export function AppIcon({item,size=28}){
 const element=useRef(null),[image,setImage]=useState(null),key=(item?.id||'')+'|'+(item?.target||'')
 useEffect(()=>{
  setImage(null)
  if(!item?.id||!isTauri())return
  let active=true,off
  const start=()=>{if(active&&!off)off=subscribe(key,item.id,value=>{if(active)setImage(value)})}
  if(typeof IntersectionObserver==='undefined')return
  const observer=new IntersectionObserver(entries=>{if(entries.some(entry=>entry.isIntersecting)){start();observer.disconnect()}},{rootMargin:'0px'})
  observer.observe(element.current)
  return()=>{active=false;observer.disconnect();off?.()}
 },[key,item?.id])
 return <span ref={element} className="app-real-icon" style={{width:size,height:size}} aria-hidden="true">{image?<img src={image} alt="" width={size} height={size} draggable={false} onError={()=>setImage(null)}/>:<AppWindow size={Math.min(19,size-6)}/>}</span>
}
