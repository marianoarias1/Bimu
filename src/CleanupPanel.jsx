import React,{useRef,useState} from 'react'
import {invoke} from '@tauri-apps/api/core'
import {listen} from '@tauri-apps/api/event'

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

function Broom(){
 return <span className="cleanup-icon" aria-hidden="true"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round"><path d="m13 10 7-7M4 16c2 0 2-6 6-6a4 4 0 0 1 4 4v2c-4 2-7 2-10 0ZM6 17l1 3m2-2 1 3m2-4 1 3"/></svg></span>
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
 if(!analysis&&!summary&&!cleaning)return <section className="cleanup-panel cleanup-intro" aria-label="Limpieza de almacenamiento" aria-busy={!!busy}>
  <div className="cleanup-intro-row">
   <Broom/>
   <div className="cleanup-intro-copy"><span className="cleanup-kicker">Mantenimiento</span><h2>Limpieza</h2><p>Liberá espacio en disco</p><div className="cleanup-chips"><span>Temporales</span><span>Papelera</span></div></div>
   <div className="cleanup-actions"><button type="button" className="utility-primary" disabled={!native||!!busy} onClick={analyze}>{busy?'Analizando…':'Analizar'}</button></div>
  </div>
  {(error||!native)&&<p className="cleanup-hint" role="status">{error||'Disponible en Bimu para Windows.'}</p>}
 </section>
 return <section className="cleanup-panel cleanup-detail" aria-label="Limpieza de almacenamiento" aria-busy={cleaning}>
  <div className="cleanup-heading">
   <Broom/>
   <div><h2>{cleaning?'Limpiando…':summary?'Limpieza finalizada':'Limpieza'}</h2><small>{cleaning?'Archivos eliminados':summary?'Tamaño eliminado':'Espacio seleccionado'}</small></div>
   <strong className="cleanup-amount">{size(cleaning?progress?.removedBytes??0:summary?removed:selected)}</strong>
  </div>
  {cleaning?<div className="cleanup-progress">
   <progress aria-label={processingFiles?'Archivos temporales revisados':'Limpieza en curso'} max={processingFiles?progress.total:1} value={processingFiles?progress.processed:undefined}/>
   <p className="cleanup-hint" role="status">{progress?.phase==='recycle'?'Vaciando la papelera…':processingFiles?`${progress.processed} de ${progress.total} archivos temporales revisados`:'Preparando la limpieza…'}</p>
   <p className="cleanup-hint">Se omiten archivos recientes, en uso o sin permiso.</p>
  </div>:analysis?<>
   <div className="cleanup-chart" role="img" aria-label={`${size(selected)} seleccionados de ${size(scanned)} encontrados`}>
    {categories.map(([label,category,checked],i)=>category?.bytes>0&&<span key={label} className={`${i===1?'is-recycle':''} ${checked?'is-selected':''}`} style={{flexGrow:category.bytes/scanned}}/>)}
   </div>
   <div className="cleanup-categories">
    {categories.map(([label,category,checked,change])=><label key={label} className="cleanup-category">
     <input type="checkbox" checked={checked} disabled={!category?.files} onChange={event=>change(event.target.checked)}/>
     <span>{label}</span><span>{category?size(category.bytes):'No disponible'}</span>
    </label>)}
   </div>
   <p className="cleanup-hint">{recycle?'La papelera se vaciará de forma permanente al limpiar.':analysis.limited?'Análisis parcial. Solo se limpiarán los archivos encontrados.':'Se omiten archivos recientes, en uso o sin permiso.'}</p>
   {recycle&&analysis.limited&&<p className="cleanup-hint">Análisis parcial de archivos temporales.</p>}
  </>:summary&&<div className="cleanup-summary" role="status">
   <div className="cleanup-chart" role="img" aria-label={`${size(removed)} eliminados de ${size(summary.estimatedBytes)} estimados`}>
    {summary.removedBytes>0&&<span className="is-selected" style={{flexGrow:summary.removedBytes/Math.max(removed,summary.estimatedBytes)}}/>}
    {summary.recycleBytes>0&&<span className="is-selected is-recycle" style={{flexGrow:summary.recycleBytes/Math.max(removed,summary.estimatedBytes)}}/>}
    {removed<summary.estimatedBytes&&<span style={{flexGrow:(summary.estimatedBytes-removed)/summary.estimatedBytes}}/>}
   </div>
   {(summary.removedFiles||summary.skipped||!summary.recycleStatus)?<p>{summary.removedFiles?`Temporales: ${size(summary.removedBytes)} eliminados`:'No se eliminaron archivos temporales.'}{summary.skipped?` · ${summary.skipped} omitidos`:''}</p>:null}
   {summary.recycleStatus&&<p>{{emptied:`Papelera vaciada · ${size(summary.recycleBytes)} eliminados`,changed:'La papelera cambió. Volvé a analizarla.',notEmptied:'No se vació toda la papelera.'}[summary.recycleStatus]}</p>}
  </div>}
  {!cleaning&&<div className="cleanup-actions">
   <button type="button" className={analysis?'':'utility-primary'} onClick={analyze}>Volver a analizar</button>
   {analysis&&<button type="button" className="utility-primary" disabled={!(temporary||recycle)} onClick={clean}>Limpiar selección</button>}
  </div>}
 </section>
}
