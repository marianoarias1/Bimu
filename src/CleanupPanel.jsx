import React,{useRef,useState} from 'react'
import {invoke} from '@tauri-apps/api/core'
import {listen} from '@tauri-apps/api/event'
import {Search} from 'lucide-react'

const precise=new Intl.NumberFormat('es-AR',{maximumFractionDigits:2})
const format=new Intl.NumberFormat('es-AR',{maximumFractionDigits:1})
function size(bytes,live=false){
 if(bytes>=1024**3)return `${(live?precise:format).format(bytes/1024**3)} GB`
 if(bytes>=1024**2)return `${format.format(bytes/1024**2)} MB`
 if(bytes>=1024)return `${format.format(bytes/1024)} KB`
 return `${format.format(bytes)} B`
}

export function useCleanup(native){
 const [analysis,setAnalysis]=useState(null),[summary,setSummary]=useState(null)
 const [busy,setBusy]=useState(null),[error,setError]=useState('')
 const [progress,setProgress]=useState(null)
 const [planReady,setPlanReady]=useState(false)
 const [temporary,setTemporary]=useState(false),[recycle,setRecycle]=useState(false)
 const locked=useRef(false)
 const analyze=async()=>{
  if(!native||locked.current)return
  locked.current=true
  setBusy('analyze');setError('');setPlanReady(false)
  try{
   const result=await invoke('cleanup_analyze')
   setAnalysis(result);setSummary(null);setRecycle(false);setPlanReady(true)
   setTemporary((result.temporary?.files??0)>0)
  }catch(e){setError(typeof e==='string'?e:'No se pudo analizar el almacenamiento.')}
  finally{locked.current=false;setBusy(null)}
 }
 const clean=async()=>{
  if(!native||!analysis||!planReady||(!temporary&&!recycle)||locked.current)return
  locked.current=true
  setBusy('clean');setError('');setProgress(null);setPlanReady(false)
  const {planId}=analysis
  let off
  try{
   off=await listen('cleanup-progress',({payload})=>{
    if(payload.planId===planId)setProgress(payload)
   }).catch(()=>null)
   const result=await invoke('cleanup_run',{planId,temporary,recycle})
   setSummary({...result,estimatedBytes:selectionSize(analysis,temporary,recycle)})
  }catch(e){setError(typeof e==='string'?e:'No se pudo completar la limpieza.')}
  finally{
   locked.current=false;setBusy(null);setProgress(null)
   off?.()
  }
 }
 return {native,analysis,summary,busy,error,progress,planReady,temporary,recycle,setTemporary,setRecycle,analyze,clean}
}

const selectionSize=(analysis,temporary,recycle)=>
 (temporary?analysis?.temporary?.bytes??0:0)+(recycle?analysis?.recycle?.bytes??0:0)

function CleaningIcon(){
 return <svg width="28" height="19" viewBox="0 0 28 19" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
  <path d="M5 8V4.5a1.5 1.5 0 0 1 3 0V8M1.5 17v-4a5 5 0 0 1 5-5h.5a5 5 0 0 1 5 5v4ZM20 8V2.5a1.5 1.5 0 0 1 3 0V8m-6 4h9m-7.5 5 1.5-3m3 3-1-3M17 12a4.5 4.5 0 0 1 9 0l1 5H16Z"/>
 </svg>
}

function CleanupRing({amount,total,caption,live}){
 const percent=total>0?Math.max(0,Math.min(100,amount/total*100)):0
 const value=size(amount,live)
 return <div className="cleanup-ring" role="img" aria-label={`${value} ${caption} de ${size(total)}`}>
  <svg viewBox="0 0 100 100" aria-hidden="true">
   <circle className="cleanup-ring-track" cx="50" cy="50" r="46"/>
   <circle className="cleanup-ring-fill" cx="50" cy="50" r="46" pathLength="100" transform="rotate(-90 50 50)" strokeDasharray={`${percent} 100`} style={{opacity:percent>0?1:0}}/>
  </svg>
  <span aria-hidden="true"><strong>{value}</strong><small>{caption}</small></span>
 </div>
}

function cleanupMessage({analysis,summary,busy,error,progress,recycle,scanned}){
 if(error)return error
 if(busy==='analyze')return 'Analizando…'
 if(busy==='clean'){
  if(progress?.phase==='recycle')return 'Vaciando la papelera…'
  return progress?.phase==='temporary'?'Limpiando archivos temporales…':'Preparando la limpieza…'
 }
 if(summary){
  const binMessage={emptied:'Papelera vaciada.',changed:'La papelera cambió. Volvé a analizarla.',notEmptied:'No se pudo vaciar toda la papelera.'}[summary.recycleStatus]
  const skipped=summary.skipped?`${summary.skipped} ${summary.skipped===1?'archivo omitido':'archivos omitidos'}.`:null
  return [binMessage||'Limpieza finalizada.',skipped].filter(Boolean).join(' ')
 }
 if(!analysis?.limited&&!analysis?.temporary?.files&&!analysis?.recycle?.files){
  return !analysis?.temporary||!analysis?.recycle?'No hay archivos en las categorías disponibles.':'No hay archivos para limpiar.'
 }
 const available=`${size(scanned)} disponibles.`
 const notice=recycle?'La papelera se vaciará de forma permanente.':analysis?.limited?'Análisis parcial de archivos temporales.':'Se omiten archivos recientes o en uso.'
 return [available,notice,recycle&&analysis?.limited?'Análisis parcial.':null].filter(Boolean).join(' ')
}

export function CleanupPanel({state}){
 const {native,analysis,summary,busy,error,progress,planReady,temporary,recycle,setTemporary,setRecycle,analyze,clean}=state
 const selected=selectionSize(analysis,temporary,recycle)
 const categories=[
  ['Archivos temporales',analysis?.temporary,temporary,setTemporary],
  ['Papelera de reciclaje',analysis?.recycle,recycle,setRecycle],
 ]
 const removed=(summary?.removedBytes??0)+(summary?.recycleBytes??0)
 const scanned=selectionSize(analysis,true,true)
 const cleaning=busy==='clean'
 const intro=!analysis
 const canSelect=planReady&&!busy
 const ring=cleaning?{amount:progress?.removedBytes??0,total:selected,caption:'eliminados',live:true}
  :summary?{amount:removed,total:Math.max(removed,summary.estimatedBytes),caption:'liberados'}
  :{amount:selected,total:scanned,caption:'para limpiar'}
 return <section className={'cleanup-panel '+(intro?'cleanup-intro':'cleanup-detail')} aria-label="Limpieza de almacenamiento" aria-busy={intro&&!!busy}>
  <div className="clipboard-heading">
   <strong>Limpieza</strong>
   {intro&&<div><button type="button" disabled={!native||!!busy} onClick={analyze}><Search size={12} aria-hidden="true"/>{busy?'Analizando…':'Analizar'}</button></div>}
  </div>
  {intro?<div className="clipboard-empty">
   <CleaningIcon/>
   <strong>Liberá espacio en disco</strong>
   <small role={error?'status':undefined}>{error||(!native?'Disponible en Bimu para Windows.':'Revisá los archivos temporales y la papelera.')}</small>
  </div>:<div className="cleanup-detail-grid">
   <CleanupRing {...ring}/>
   <div className="cleanup-selection">
    <div className="cleanup-categories">
     {categories.map(([label,category,checked,change])=><label key={label} className="cleanup-category">
      <input type="checkbox" checked={checked} disabled={!canSelect||!category?.files} onChange={event=>change(event.target.checked)}/>
      <span>{label}</span><span>{category?size(category.bytes):'No disponible'}</span>
     </label>)}
    </div>
    <p className="cleanup-hint" role="status">{cleanupMessage({analysis,summary,busy,error,progress,recycle,scanned})}</p>
    <div className="cleanup-actions">
     <button type="button" disabled={!!busy} onClick={analyze}>Volver a analizar</button>
     <button type="button" className="utility-primary" disabled={!canSelect||!(temporary||recycle)} onClick={clean}>Limpiar selección</button>
    </div>
   </div>
  </div>}
 </section>
}
