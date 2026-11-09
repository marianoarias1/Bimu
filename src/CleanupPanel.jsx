import React,{useRef,useState} from 'react'
import {invoke} from '@tauri-apps/api/core'
import {listen} from '@tauri-apps/api/event'
import {Search} from 'lucide-react'

const format=new Intl.NumberFormat('es-AR',{maximumFractionDigits:1})
function size(bytes){
 if(bytes>=1024**3)return `${format.format(bytes/1024**3)} GB`
 if(bytes>=1024**2)return `${format.format(bytes/1024**2)} MB`
 if(bytes>=1024)return `${format.format(bytes/1024)} KB`
 return `${format.format(bytes)} B`
}

export function useCleanup(native){
 const [analysis,setAnalysis]=useState(null),[summary,setSummary]=useState(null)
 const [busy,setBusy]=useState(null),[error,setError]=useState('')
 const [progress,setProgress]=useState(null)
 const [temporary,setTemporary]=useState(false),[recycle,setRecycle]=useState(false)
 const locked=useRef(false)
 const analyze=async()=>{
  if(!native||locked.current)return
  locked.current=true
  setBusy('analyze');setError('');setSummary(null);setAnalysis(null);setTemporary(false);setRecycle(false)
  try{
   const result=await invoke('cleanup_analyze')
   setAnalysis(result)
   setTemporary((result.temporary?.files??0)>0)
  }catch(e){setError(typeof e==='string'?e:'No se pudo analizar el almacenamiento.')}
  finally{locked.current=false;setBusy(null)}
 }
 const clean=async()=>{
  if(!native||!analysis||(!temporary&&!recycle)||locked.current)return
  locked.current=true
  setBusy('clean');setError('');setProgress(null)
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
   off?.()
   locked.current=false;setBusy(null);setAnalysis(null);setProgress(null);setTemporary(false);setRecycle(false)
  }
 }
 return {native,analysis,summary,busy,error,progress,temporary,recycle,setTemporary,setRecycle,analyze,clean}
}

const selectionSize=(analysis,temporary,recycle)=>
 (temporary?analysis?.temporary?.bytes??0:0)+(recycle?analysis?.recycle?.bytes??0:0)

function CleaningIcon(){
 return <svg width="28" height="19" viewBox="0 0 28 19" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
  <path d="M5 8V4.5a1.5 1.5 0 0 1 3 0V8M1.5 17v-4a5 5 0 0 1 5-5h.5a5 5 0 0 1 5 5v4ZM20 8V2.5a1.5 1.5 0 0 1 3 0V8m-6 4h9m-7.5 5 1.5-3m3 3-1-3M17 12a4.5 4.5 0 0 1 9 0l1 5H16Z"/>
 </svg>
}

function CleanupRing({amount,total,caption}){
 const percent=total>0?Math.max(0,Math.min(100,amount/total*100)):0
 return <div className="cleanup-ring" role="img" aria-label={`${size(amount)} ${caption} de ${size(total)}`}>
  <svg viewBox="0 0 100 100" aria-hidden="true">
   <circle className="cleanup-ring-track" cx="50" cy="50" r="46"/>
   <circle className="cleanup-ring-fill" cx="50" cy="50" r="46" pathLength="100" transform="rotate(-90 50 50)" strokeDasharray={`${percent} 100`} style={{opacity:percent>0?1:0}}/>
  </svg>
  <span aria-hidden="true"><strong>{size(amount)}</strong><small>{caption}</small></span>
 </div>
}

export function CleanupPanel({state}){
 const {native,analysis,summary,busy,error,progress,temporary,recycle,setTemporary,setRecycle,analyze,clean}=state
 const selected=selectionSize(analysis,temporary,recycle)
 const categories=[
  ['Archivos temporales',analysis?.temporary,temporary,setTemporary],
  ['Papelera de reciclaje',analysis?.recycle,recycle,setRecycle],
 ]
 const removed=(summary?.removedBytes??0)+(summary?.recycleBytes??0)
 const scanned=selectionSize(analysis,true,true)
 const cleaning=busy==='clean'
 const processingFiles=progress?.phase==='temporary'&&progress.total>0
 const intro=!analysis&&!summary&&!cleaning
 return <section className={'cleanup-panel '+(intro?'cleanup-intro':'cleanup-detail')} aria-label="Limpieza de almacenamiento" aria-busy={!!busy}>
  <div className="clipboard-heading">
   <strong>Limpieza</strong>
   {intro&&<div><button type="button" disabled={!native||!!busy} onClick={analyze}><Search size={12} aria-hidden="true"/>{busy?'Analizando…':'Analizar'}</button></div>}
  </div>
  {intro?<div className="clipboard-empty">
   <CleaningIcon/>
   <strong>Liberá espacio en disco</strong>
   <small role={error?'status':undefined}>{error||(!native?'Disponible en Bimu para Windows.':'Revisá los archivos temporales y la papelera.')}</small>
  </div>:cleaning?<div className="cleanup-progress">
   <strong>Limpiando…</strong>
   <p className="cleanup-hint">{size(progress?.removedBytes??0)} en archivos eliminados</p>
   <progress aria-label={processingFiles?'Archivos temporales revisados':'Limpieza en curso'} max={processingFiles?progress.total:1} value={processingFiles?progress.processed:undefined}/>
   <p className="cleanup-hint" role="status">{progress?.phase==='recycle'?'Vaciando la papelera…':processingFiles?`${progress.processed} de ${progress.total} archivos temporales revisados`:'Preparando la limpieza…'}</p>
   <p className="cleanup-hint">Se omiten archivos recientes, en uso o sin permiso.</p>
  </div>:analysis?<div className="cleanup-detail-grid">
   <CleanupRing amount={selected} total={scanned} caption="seleccionados"/>
   <div className="cleanup-selection">
    <div className="cleanup-categories">
     {categories.map(([label,category,checked,change])=><label key={label} className="cleanup-category">
      <input type="checkbox" checked={checked} disabled={!category?.files} onChange={event=>change(event.target.checked)}/>
      <span>{label}</span><span>{category?size(category.bytes):'No disponible'}</span>
     </label>)}
    </div>
    <p className="cleanup-hint">{recycle?'La papelera se vaciará de forma permanente al limpiar.':analysis.limited?'Análisis parcial. Solo se limpiarán los archivos encontrados.':'Se omiten archivos recientes, en uso o sin permiso.'}</p>
    {recycle&&analysis.limited&&<p className="cleanup-hint">Análisis parcial de archivos temporales.</p>}
    <div className="cleanup-actions">
     <button type="button" onClick={analyze}>Volver a analizar</button>
     <button type="button" className="utility-primary" disabled={!(temporary||recycle)} onClick={clean}>Limpiar selección</button>
    </div>
   </div>
  </div>:<div className="cleanup-detail-grid">
   <CleanupRing amount={removed} total={Math.max(removed,summary.estimatedBytes)} caption="eliminados"/>
   <div className="cleanup-summary">
    <div role="status">
     <strong>Limpieza finalizada</strong>
     {(summary.removedFiles||summary.skipped||!summary.recycleStatus)?<p>{summary.removedFiles?`Temporales: ${size(summary.removedBytes)} eliminados`:'No se eliminaron archivos temporales.'}{summary.skipped?` · ${summary.skipped} omitidos`:''}</p>:null}
     {summary.recycleStatus&&<p>{{emptied:`Papelera vaciada · ${size(summary.recycleBytes)} eliminados`,changed:'La papelera cambió. Volvé a analizarla.',notEmptied:'No se vació toda la papelera.'}[summary.recycleStatus]}</p>}
    </div>
    <div className="cleanup-actions"><button type="button" onClick={analyze}>Volver a analizar</button></div>
   </div>
  </div>}
 </section>
}
