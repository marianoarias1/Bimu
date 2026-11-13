import React,{useEffect,useRef,useState} from 'react'
import {invoke} from '@tauri-apps/api/core'
import {listen} from '@tauri-apps/api/event'
export function FilePreview({scope,id,preview,version,native,fallback,className=''}){
 const [image,setImage]=useState(preview||null),element=useRef(null)
 useEffect(()=>{let alive=true,off,visible=false,attempts=0,complete=false;setImage(preview||null);if(preview||!native||!scope)return()=>{alive=false}
  const load=async()=>{attempts++;try{const value=await invoke(scope==='shelf'?'shelf_thumbnail':'clipboard_history_thumbnail',{id});complete=typeof value==='string';if(alive)setImage(typeof value==='string'&&value.startsWith('data:image/png;base64,')?value:null)}catch{if(alive)setImage(null)}}
  const observer=new IntersectionObserver(entries=>{visible=entries.some(entry=>entry.isIntersecting);if(visible)load()},{rootMargin:'40px'});if(element.current)observer.observe(element.current);if(scope==='shelf')listen('file-preview-ready',()=>{if(alive&&visible)load()}).then(fn=>{if(alive)off=fn;else fn()}).catch(()=>{})
  const retry=scope==='shelf'?setInterval(()=>{if(visible&&!complete&&attempts<4)load()},5000):null;return()=>{alive=false;off?.();observer.disconnect();clearInterval(retry)}
 },[scope,id,preview,version,native])
 return <span ref={element} className={'file-preview '+className}>{image?<img src={image} alt="" loading="lazy" decoding="async" onError={()=>setImage(null)}/>:fallback}</span>
}
