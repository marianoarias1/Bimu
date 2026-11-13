import React,{useEffect,useRef,useState} from 'react'
import {invoke} from '@tauri-apps/api/core'
import {listen} from '@tauri-apps/api/event'
export function FilePreview({scope,id,preview,version,native,fallback,className=''}){
 const [image,setImage]=useState(preview||null),element=useRef(null)
 useEffect(()=>{
  let alive=true,off,visible=false,pending=false,queued=false,complete=false,attempts=0
  setImage(preview||null)
  if(preview||!native||!scope)return()=>{alive=false}
  const load=async()=>{
   if(!alive||!visible||complete)return
   if(pending){queued=true;return}
   pending=true;attempts++
   try{
    const value=await invoke(scope==='shelf'?'shelf_thumbnail':'clipboard_history_thumbnail',{id})
    if(alive&&typeof value==='string'&&value.startsWith('data:image/png;base64,')){complete=true;setImage(value)}
   }catch{/* A temporary provider failure can recover on the next visible retry. */}
   finally{pending=false;if(queued&&alive){queued=false;load()}}
  }
  const observer=new IntersectionObserver(entries=>{visible=entries.some(entry=>entry.isIntersecting);if(visible){attempts=0;load()}},{rootMargin:'40px'})
  if(element.current)observer.observe(element.current)
  const reveal=()=>{attempts=0;load()}
  document.addEventListener('bimu-panel-visible',reveal)
  if(scope==='shelf')listen('file-preview-ready',reveal).then(fn=>{if(alive)off=fn;else fn()}).catch(()=>{})
  const retry=setInterval(()=>{if(visible&&!complete&&attempts<6)load()},2000)
  return()=>{alive=false;off?.();observer.disconnect();clearInterval(retry);document.removeEventListener('bimu-panel-visible',reveal)}
 },[scope,id,preview,version,native])
 // Visibility is already controlled above; a second browser lazy-load gate can
 // otherwise keep images blank when a transparent native window moves.
 return <span ref={element} className={'file-preview '+className}>{image?<img src={image} alt="" decoding="async" onError={()=>setImage(null)}/>:fallback}</span>
}
