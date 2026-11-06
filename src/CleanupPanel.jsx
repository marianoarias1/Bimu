import React,{useRef,useState} from 'react'
import {invoke} from '@tauri-apps/api/core'

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
 const [temporary,setTemporary]=useState(false),[recycle,setRecycle]=useState(false)
 const locked=useRef(false)
 const analyze=async()=>{
  if(!native||locked.current)return
  locked.current=true
  setBusy('analyze');setError('');setSummary(null);setAnalysis(null);setRecycle(false)
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
  setBusy('clean');setError('')
  const planId=analysis.planId
  setAnalysis(null)
  try{setSummary(await invoke('cleanup_run',{planId,temporary,recycle}))}
  catch(e){setError(typeof e==='string'?e:'No se pudo completar la limpieza.')}
  finally{locked.current=false;setBusy(null);setTemporary(false);setRecycle(false)}
 }
 return {native,analysis,summary,busy,error,temporary,recycle,setTemporary,setRecycle,analyze,clean}
}

export function CleanupPanel({state}){
 const {native,analysis,summary,busy,error,temporary,recycle,setTemporary,setRecycle,analyze,clean}=state
 const selected=(temporary?analysis?.temporary?.bytes??0:0)+(recycle?analysis?.recycle?.bytes??0:0)
 const categories=[
  ['Archivos temporales',analysis?.temporary,temporary,setTemporary],
  ['Papelera de reciclaje',analysis?.recycle,recycle,setRecycle],
 ]
 return <section className="cleanup-panel" aria-label="Limpieza de almacenamiento" aria-busy={!!busy}>
  <div className="cleanup-heading"><svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="m18 3-5 8M10 10l5 3-1 3-7-4 3-2ZM7 12l-4 6 9 3 2-5M7 19l2-4"/></svg><div><strong>Limpieza</strong><small>{analysis?`Espacio seleccionado: ${size(selected)}`:'Liberá espacio en disco'}</small></div></div>
  {analysis&&<>
   <div className="cleanup-categories">
    {categories.map(([label,category,checked,change])=><label key={label} className="cleanup-category">
     <input type="checkbox" checked={checked} disabled={!category?.files||!!busy} onChange={event=>change(event.target.checked)}/>
     <span>{label}</span><span>{category?size(category.bytes):'No disponible'}</span>
    </label>)}
   </div>
   <p className="cleanup-hint">{recycle?'Windows pedirá confirmar el vaciado de la papelera.':analysis.limited?'Análisis parcial. Solo se limpiarán los archivos encontrados.':'Se omiten archivos recientes, en uso o sin permiso.'}</p>
  </>}
  {summary&&<div className="cleanup-summary" role="status">
   <p>{summary.removedFiles?`${size(summary.removedBytes)} en archivos eliminados`:'No se eliminaron archivos temporales.'}{summary.skipped?` · ${summary.skipped} omitidos`:''}</p>
   {summary.recycleStatus&&<p>{{emptied:'Papelera vaciada.',changed:'La papelera cambió. Volvé a analizarla.',notEmptied:'No se vació toda la papelera.'}[summary.recycleStatus]}</p>}
  </div>}
  {busy&&<p className="cleanup-hint" role="status">{busy==='analyze'?'Analizando…':'Limpiando selección…'}</p>}
  {!native&&<p className="cleanup-hint">Disponible en Bimu para Windows.</p>}
  {error&&<p className="cleanup-hint" role="status">{error}</p>}
  <div className="cleanup-actions">
   <button type="button" className={analysis?'':'utility-primary'} disabled={!native||!!busy} onClick={analyze}>{analysis||summary?'Volver a analizar':'Analizar'}</button>
   {analysis&&<button type="button" className="utility-primary" disabled={!!busy||!(temporary||recycle)} onClick={clean}>Limpiar selección</button>}
  </div>
 </section>
}
