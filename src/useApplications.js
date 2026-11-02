import {useEffect,useRef,useState} from 'react'
import {invoke} from '@tauri-apps/api/core'
import {registeredLaunch} from './application-intent'
import {registeredApp} from './shortcut-ui'
export function useApplications({native,onResult}){
 const [discovered,setDiscovered]=useState([]),[discovering,setDiscovering]=useState(false),[discoveryLoaded,setDiscoveryLoaded]=useState(false)
 const discoveryLock=useRef(false)
 const [items,setItems]=useState([]),[pending,setPending]=useState(null),[busy,setBusy]=useState(false),[error,setError]=useState('')
 const itemsRef=useRef(items);itemsRef.current=items
 const lock=useRef(false),current=useRef(null),mounted=useRef(true),callback=useRef(onResult);callback.current=onResult
 const refresh=async()=>{if(!native)return;try{const entries=await invoke('apps_list');if(mounted.current){setItems(entries);setError('')}}catch(e){if(mounted.current)setError(String(e.message||e))}}
 useEffect(()=>{mounted.current=true;refresh();return()=>{mounted.current=false;if(current.current)invoke('apps_cancel',{id:current.current.id}).catch(()=>{})}},[native])
 const discover=async(force=false)=>{if(!native||discoveryLock.current||(!force&&discoveryLoaded))return;discoveryLock.current=true;setDiscovering(true);setError('');try{const list=await invoke('apps_discover');if(mounted.current){setDiscovered(list);setDiscoveryLoaded(true)}}catch(e){if(mounted.current)setError(String(e.message||e))}finally{discoveryLock.current=false;if(mounted.current)setDiscovering(false)}}
 const request=async (text,options={})=>{
  if(!registeredLaunch(items,text))return false
  if(!native){setError('Abrí la aplicación de escritorio para abrir otras aplicaciones.');return true}
  if(lock.current||current.current)return true
  lock.current=true;setBusy(true);setError('')
  try{const result=await invoke('apps_request',{text});if(!mounted.current){if(result?.status==='confirm')invoke('apps_cancel',{id:result.id}).catch(()=>{});return true}if(!result){options.onFeedback?.('error','No entendí qué aplicación querés abrir.');return false;}if(result.status==='confirm'){const proposal={...result,text,silent:!!options.silent};current.current=proposal;setPending(proposal);options.onFeedback?.('confirm','¿Abro '+result.name+'?')}else if(result.status==='opened'){if(!options.silent)callback.current(text,'Se solicitó la apertura de '+result.name+'.');options.onFeedback?.('success','Abriendo '+result.name+'…')}return true}
  catch(e){setError(String(e.message||e));options.onFeedback?.('error',String(e.message||e));return 'error'}finally{lock.current=false;if(mounted.current)setBusy(false)}
 }
 const confirm=async remember=>{
  const proposal=current.current;if(!proposal||lock.current)return
  lock.current=true;setBusy(true);setError('')
  try{const name=await invoke('apps_confirm',{id:proposal.id,remember});if(mounted.current){if(!proposal.silent)callback.current(proposal.text,'Se solicitó la apertura de '+name+'.');current.current=null;setPending(null);await refresh();return true}}
  catch(e){await invoke('apps_cancel',{id:proposal.id}).catch(()=>{});if(mounted.current){setError(String(e.message||e));current.current=null;setPending(null)}return false}finally{lock.current=false;if(mounted.current)setBusy(false)}
 }
 const cancel=async()=>{const proposal=current.current;if(!proposal||lock.current)return;try{await invoke('apps_cancel',{id:proposal.id});current.current=null;setPending(null);if(!proposal.silent)callback.current(proposal.text,'Apertura cancelada.')}catch(e){setError(String(e.message||e))}}
 const change=async(command,args,withEntries=false)=>{if(lock.current||current.current)return;lock.current=true;setBusy(true);setError('');try{const list=await invoke(command,args);if(list){itemsRef.current=list;if(mounted.current)setItems(list)}return withEntries?(list||null):true}catch(e){setError(String(e.message||e));return false}finally{lock.current=false;setBusy(false)}}
 return {items,pending,busy,error,refresh,request,confirm,cancel,ensureDiscovered:async item=>{const existing=registeredApp(itemsRef.current,item);if(existing)return existing;const list=await change('apps_add_discovered',{id:item.id},true);return list?registeredApp(list,item):null},pick:async()=>{const old=new Set(itemsRef.current.map(item=>item.id));const list=await change('apps_add',{},true);return list?.find(item=>!old.has(item.id))||null},discovered,discovering,discoveryLoaded,discover,addDiscovered:async id=>{const ok=await change('apps_add_discovered',{id});if(ok)setDiscovered(list=>list.map(item=>item.id===id?{...item,added:true}:item));return ok},add:()=>change('apps_add'),update:(id,name,aliases,remember)=>change('apps_update',{id,name,aliases,remember}),remove:id=>change('apps_remove',{id}),isCommand:text=>registeredLaunch(items,text)}
}
