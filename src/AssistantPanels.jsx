import React,{useState,useEffect,useRef} from 'react'
import {GripVertical,ChevronLeft,ChevronRight,Plus,Play,Pencil,MoreHorizontal,Trash2,AppWindow,Globe,FolderOpen,LoaderCircle} from 'lucide-react'
import {invoke} from '@tauri-apps/api/core'
import {ApplicationSettings,ApplicationPicker} from './ApplicationPanels'
import {AppIcon} from './AppIcon'
import {PairInfo} from './DevicePairingSettings'
import {moveShortcut,actionTitle,shortcutDescription,validShortcutDraft} from './shortcut-ui'

function ActionIcon({step,apps,size=28}){
 if(step.kind==='app')return <AppIcon item={apps.find(app=>app.id===step.appId)||{id:step.appId}} size={size}/>
 const Icon=step.kind==='url'?Globe:FolderOpen
 return <span className="app-real-icon" style={{width:size,height:size}} aria-hidden="true"><Icon size={Math.min(20,size-5)}/></span>
}
function ShortcutRows({items,apps,disabled,onReorder,onRun,onEdit,onRemove,result,running}){
 const drag=useRef(null),list=useRef(null),[motion,setMotion]=useState(null),[menu,setMenu]=useState(null),[removing,setRemoving]=useState(null)
 const move=(id,target)=>{if(disabled||id===target)return;onReorder(moveShortcut(items,id,target))}
 const end=(event,cancel=false)=>{const active=drag.current;if(!active||active.pointerId!==event.pointerId)return;if(!cancel&&active.started)move(active.id,active.target);drag.current=null;setMotion(null);setMenu(null)}
 const update=event=>{const active=drag.current;if(!active||active.pointerId!==event.pointerId)return;const delta=event.clientY-active.startY;if(!active.started&&Math.abs(delta)<4)return;active.started=true;const center=active.rows[active.origin].mid+delta;const index=active.rows.findIndex((row,i)=>i===active.rows.length-1||center<(row.mid+active.rows[i+1].mid)/2);active.target=active.rows[index]?.id||active.id;setMotion({id:active.id,delta,target:active.target,origin:active.origin,step:active.step})}
 return <ol className="sc-shortcut-list" ref={list} aria-label="Tus atajos, en el orden del notch">{items.map((item,index)=>{
  const targetIndex=motion?items.findIndex(entry=>entry.id===motion.target):-1
  const offset=!motion?0:item.id===motion.id?motion.delta:motion.origin<targetIndex&&index>motion.origin&&index<=targetIndex?-motion.step:motion.origin>targetIndex&&index>=targetIndex&&index<motion.origin?motion.step:0
  return <li key={item.id} data-shortcut-id={item.id} className={motion?.id===item.id?'is-dragging':''} style={{transform:`translateY(${offset}px)`}}>
   <div className="sc-shortcut-row">
    <button type="button" className="sc-drag sc-icon-button" disabled={disabled||items.length<2} aria-label={'Ordenar '+item.name} title="Arrastrar para ordenar; también podés usar Más opciones"
     onPointerDown={event=>{if(disabled||event.button!==0)return;event.preventDefault();event.currentTarget.setPointerCapture(event.pointerId);const rows=[...(list.current?.querySelectorAll('[data-shortcut-id]')||[])].map(element=>{const rect=element.getBoundingClientRect();return{id:element.dataset.shortcutId,mid:rect.top+rect.height/2,top:rect.top,height:rect.height}});const origin=rows.findIndex(row=>row.id===item.id);if(origin<0)return;drag.current={id:item.id,pointerId:event.pointerId,startY:event.clientY,target:item.id,started:false,rows,origin,step:rows[origin].height+8}}}
     onPointerMove={update} onPointerUp={end} onPointerCancel={event=>end(event,true)} onLostPointerCapture={event=>end(event,true)}><GripVertical size={15}/></button>
    <span className="sc-icon-stack">{item.actions.slice(0,3).map((step,i)=><ActionIcon key={i} step={step} apps={apps} size={item.actions.length===1?28:22}/>)}</span>
    <span className="sc-shortcut-name"><strong title={item.name}>{item.name}</strong><small title={shortcutDescription(item,apps)}>{shortcutDescription(item,apps)}</small></span>
    <button type="button" className="sc-open" disabled={disabled} onClick={()=>onRun(item.id)}>{running===item.id?<LoaderCircle className="pair-spinner" size={14}/>:<Play size={13} fill="currentColor"/>} Abrir</button>
    <button type="button" className="sc-icon-button" disabled={disabled} title="Editar atajo" aria-label={'Editar '+item.name} onClick={()=>onEdit(item)}><Pencil size={15}/></button>
    <button type="button" className="sc-icon-button" disabled={disabled} aria-label={'Más opciones de '+item.name} aria-expanded={menu===item.id} onClick={()=>setMenu(menu===item.id?null:item.id)}><MoreHorizontal size={17}/></button>
   </div>
   {menu===item.id&&<div className="sc-row-menu-options"><button type="button" disabled={disabled||index===0} onClick={()=>move(item.id,items[index-1]?.id)}>Mover arriba</button><button type="button" disabled={disabled||index===items.length-1} onClick={()=>move(item.id,items[index+1]?.id)}>Mover abajo</button><button type="button" className="sc-danger" disabled={disabled} onClick={()=>{setRemoving(item.id);setMenu(null)}}>Eliminar atajo</button></div>}
   {removing===item.id&&<div className="sc-inline-confirm"><p>¿Eliminar «{item.name}»? Sus aplicaciones no se desinstalan.</p><button type="button" disabled={disabled} onClick={()=>setRemoving(null)}>Cancelar</button><button type="button" className="sc-danger" disabled={disabled} onClick={async()=>{if(await onRemove(item.id))setRemoving(null)}}>Eliminar</button></div>}
   {result?.id===item.id&&<p className="sc-run-result" role="status">Apertura solicitada: {result.results.filter(step=>step.ok).length}/{result.results.length}.{result.results.filter(step=>!step.ok).map((step,i)=><span key={i}>{step.name}: {step.detail}</span>)}</p>}
  </li>
 })}</ol>
}
function ActionPicker({apps,actions,initial,onChoose,onBack,disabled}){
 const [kind,setKind]=useState(initial?.kind||'app'),[url,setUrl]=useState(initial?.kind==='url'?initial.url:''),[browserId,setBrowserId]=useState(initial?.browserId||''),[profile,setProfile]=useState(initial?.profileDirectory||''),[path,setPath]=useState(initial?.kind==='folder'?initial.path:''),[picking,setPicking]=useState(false)
 const browsers=apps.items.filter(a=>/\b(chrome|msedge|firefox|brave|opera|vivaldi)\.exe$/i.test(a.target))
 const chrome=/\bchrome\.exe$/i.test(apps.items.find(a=>a.id===browserId)?.target||'')
 const step=kind==='url'?{kind:'url',url:url.trim(),browserId:browserId||null,profileDirectory:profile||null}:{kind:'folder',path}
 const pickFolder=async()=>{setPicking(true);try{const selected=await actions.pickFolder();if(selected)setPath(selected)}finally{setPicking(false)}}
 return <section className="shortcuts-flow glass-card"><button type="button" className="sc-back" disabled={disabled||picking} onClick={onBack}><ChevronLeft size={14}/> Volver al atajo</button><h2>{initial?'Editar acción':'Agregar una acción'}</h2>
  <div className="sc-type-tabs" aria-label="Tipo de acción">{[['app','Aplicación',AppWindow],['url','Página web',Globe],['folder','Carpeta',FolderOpen]].map(([value,label,Icon])=><button key={value} type="button" aria-pressed={kind===value} disabled={disabled||picking} onClick={()=>setKind(value)}><Icon size={15}/>{label}</button>)}</div>
  {kind==='app'?<ApplicationPicker apps={apps} disabled={disabled} onBack={onBack} onChoose={app=>onChoose({kind:'app',appId:app.id,profileDirectory:initial?.appId===app.id?initial.profileDirectory||null:null},app.name)}/>:<form className="sc-action-form" onSubmit={e=>{e.preventDefault();if(validShortcutDraft({name:'Acción',actions:[step]}))onChoose(step)}}>
   {kind==='url'?<><label>Dirección de la página<input type="url" required maxLength={2048} value={url} onChange={e=>setUrl(e.target.value)} placeholder="https://mail.google.com"/></label><small>Se abre en el navegador que elijas, sin cerrar tus sesiones.</small><details className="sc-options"><summary>Navegador y perfil</summary><label>Abrir con<select value={browserId} onChange={e=>{setBrowserId(e.target.value);setProfile('')}}><option value="">Navegador predeterminado</option>{browserId&&!browsers.some(b=>b.id===browserId)&&<option value={browserId}>Navegador no disponible</option>}{browsers.map(browser=><option key={browser.id} value={browser.id}>{browser.name}{browser.available?'':' · no disponible'}</option>)}</select></label>{chrome&&<ChromeProfilePicker appId={browserId} value={profile} disabled={disabled} onChange={setProfile}/>}</details></>:<><div className="sc-folder-choice"><FolderOpen size={30}/><strong>{path?actionTitle(step,apps.items):'Elegí una carpeta de esta PC'}</strong>{path&&<small className="sc-path">{path}</small>}<button type="button" disabled={disabled||picking} onClick={pickFolder}>{picking?'Abriendo selector…':path?'Cambiar carpeta':'Elegir carpeta'}</button></div></>}
   <div className="sc-form-footer"><button type="button" disabled={disabled||picking} onClick={onBack}>Cancelar</button><button className="sc-primary" disabled={disabled||picking||!validShortcutDraft({name:'Acción',actions:[step]})}>{initial?'Guardar acción':'Agregar acción'}</button></div>
  </form>}
  {actions.error&&<p className="sc-error" role="alert">{actions.error}</p>}
 </section>
}
export function ShortcutSettings({actions,apps}){
 const [draft,setDraft]=useState(null),[picker,setPicker]=useState(null),[library,setLibrary]=useState(false),[result,setResult]=useState(null),[running,setRunning]=useState(null),[menu,setMenu]=useState(null)
 const manualName=useRef(false),nameInput=useRef(null)
 const disabled=actions.busy||apps.busy||!!apps.pending
 const edit=item=>{manualName.current=!!item;setDraft(item?JSON.parse(JSON.stringify(item)):{id:'',name:'',aliases:[],remember:false,actions:[]});setLibrary(false);setPicker(null);setMenu(null);actions.clear()}
 useEffect(()=>{if(draft&&picker===null)nameInput.current?.focus()},[!!draft,picker===null])
 const choose=(step,label)=>{setDraft(d=>{const list=[...d.actions];const index=picker.index;if(index==null)list.push(step);else list[index]=step;return {...d,name:!manualName.current&&list.length===1?(label||actionTitle(step,apps.items)).slice(0,80):d.name,actions:list}});setPicker(null)}
 const changeProfile=(index,value)=>setDraft(d=>({...d,actions:d.actions.map((step,i)=>i===index?{...step,profileDirectory:value||null}:step)}))
 const moveStep=(i,delta)=>setDraft(d=>{const list=[...d.actions];[list[i],list[i+delta]]=[list[i+delta],list[i]];return {...d,actions:list}})
 const run=async id=>{setResult(null);setRunning(id);try{setResult(await actions.runButton(id))}finally{setRunning(null)}}
 if(library)return <ApplicationSettings apps={apps} shortcuts={actions.items} onBack={()=>setLibrary(false)}/>
 if(picker)return <ActionPicker key={picker.index==null?'new':picker.index} apps={apps} actions={actions} disabled={disabled} initial={picker.index==null?null:draft.actions[picker.index]} onChoose={choose} onBack={()=>setPicker(null)}/>
 if(draft)return <section className="shortcuts-flow glass-card"><button type="button" className="sc-back" disabled={disabled} onClick={()=>setDraft(null)}><ChevronLeft size={14}/> Tus atajos</button>
  <div className="sc-heading"><h2>{draft.id?'Editar atajo':'Crear atajo'}</h2><PairInfo title="Crear un atajo"><p>Puede abrir una sola aplicación o combinar hasta 12 acciones. Las aperturas se solicitan en el orden de esta lista.</p><p>Podés llamarlo igual que una app, por ejemplo Spotify. El nombre no puede repetirse entre tus atajos.</p></PairInfo></div>
  {actions.error&&<p className="sc-error" role="alert">{actions.error}</p>}
  <form className="sc-editor" onSubmit={async e=>{e.preventDefault();if(validShortcutDraft(draft)&&await actions.save({...draft,name:draft.name.trim()}))setDraft(null)}}>
   <label>Nombre del atajo<input ref={nameInput} required maxLength={80} value={draft.name} placeholder="Spotify o Modo trabajo" disabled={disabled} onChange={e=>{manualName.current=true;setDraft(d=>({...d,name:e.target.value}))}}/></label><small>Así aparecerá el botón en Bimu.</small>
   <h3>Qué querés abrir</h3>
   <ol className="sc-action-list">{draft.actions.map((step,i)=>{const chrome=step.kind==='app'&&/\bchrome\.exe$/i.test(apps.items.find(a=>a.id===step.appId)?.target||'');return <li key={i}><div className="sc-action-row"><ActionIcon step={step} apps={apps.items}/><span className="sc-app-name"><strong>{actionTitle(step,apps.items)}</strong><small>{step.kind==='app'?'Aplicación':step.kind==='url'?'Página web':'Carpeta'}</small></span><button type="button" className="sc-icon-button" disabled={disabled} aria-label={'Editar acción '+(i+1)} onClick={()=>setPicker({index:i})}><Pencil size={15}/></button><button type="button" className="sc-icon-button" disabled={disabled} aria-label={'Más opciones de la acción '+(i+1)} aria-expanded={menu===i} onClick={()=>setMenu(menu===i?null:i)}><MoreHorizontal size={16}/></button><button type="button" className="sc-icon-button" disabled={disabled} aria-label={'Quitar acción '+(i+1)} onClick={()=>setDraft(d=>({...d,actions:d.actions.filter((_,index)=>index!==i)}))}><Trash2 size={16}/></button></div>
    {menu===i&&<div className="sc-row-menu-options"><button type="button" disabled={disabled||i===0} onClick={()=>moveStep(i,-1)}>Mover arriba</button><button type="button" disabled={disabled||i===draft.actions.length-1} onClick={()=>moveStep(i,1)}>Mover abajo</button>{chrome&&<details className="sc-options"><summary>Perfil de Chrome</summary><ChromeProfilePicker appId={step.appId} value={step.profileDirectory||''} disabled={disabled} onChange={value=>changeProfile(i,value)}/></details>}</div>}
   </li>})}</ol>
   <button type="button" className="sc-add-action" disabled={disabled||draft.actions.length>=12} onClick={()=>setPicker({index:null})}><Plus size={16}/>{draft.actions.length?'Agregar otra acción':'Elegir qué abrir'}</button>
   <small>{draft.actions.length>=12?'Llegaste al máximo de 12 acciones.':'Podés guardar una sola app o combinar varias acciones.'}</small>
   <div className="sc-form-footer"><button type="button" disabled={disabled} onClick={()=>setDraft(null)}>Cancelar</button><button className="sc-primary" disabled={disabled||!validShortcutDraft(draft)}>Guardar atajo</button></div>
  </form>
 </section>
 return <section className="shortcuts-flow glass-card">
  <div className="sc-heading"><h2>Tus atajos</h2><span className="sc-heading-actions"><PairInfo title="Tus atajos"><p>Abrí una app, páginas o carpetas con un clic. Arrastrá el control de puntos para cambiar el orden, o usá Mover arriba/abajo en Más opciones.</p></PairInfo><button className="sc-primary" type="button" disabled={disabled||actions.items.length>=30} onClick={()=>edit(null)}><Plus size={15}/> Crear atajo</button></span></div><p className="sc-helper">Abrí una app o varias cosas con un clic.</p>
  {actions.error&&<p className="sc-error" role="alert">{actions.error}</p>}
  {actions.items.length?<><ShortcutRows items={actions.items} apps={apps.items} disabled={disabled} onReorder={actions.reorder} onRun={run} onEdit={edit} onRemove={actions.remove} result={result} running={running}/><small>Arrastrá los atajos para cambiar su orden.</small></>:<div className="sc-empty"><AppWindow size={25}/><strong>Tu primer atajo, a un clic</strong><small>Elegí una app, por ejemplo Spotify, o armá un modo con varias acciones.</small><button className="sc-primary" type="button" disabled={disabled} onClick={()=>edit(null)}>Crear mi primer atajo</button></div>}
  <button type="button" className="sc-library-link" disabled={disabled} onClick={()=>setLibrary(true)}><AppWindow size={20}/><strong>Aplicaciones para atajos</strong><small>{apps.items.length} agregadas</small><ChevronRight size={16}/></button>
 </section>
}

export function AssistantResults({actions}){
 const [query,setQuery]=useState('')
 if(!actions.searching&&!actions.search&&!actions.execution&&!actions.error)return null
 return <section className="assistant-results glass-card" aria-label="Resultados del asistente"><div className="section-line"><h2>{actions.searching?'Buscando carpetas…':actions.search?'Carpetas encontradas':'Resultado del atajo'}</h2><button disabled={actions.busy&&!actions.searching} onClick={actions.searching?actions.cancelSearch:actions.clear}>{actions.searching?'Cancelar búsqueda':'Cerrar'}</button></div>
 {actions.searching&&<p role="status">Buscando por nombre en tu perfil y discos locales. Puede tardar unos segundos.</p>}
 {actions.error&&<p role="alert" className="ollama-error">{actions.error}</p>}
 {actions.search&&<><p>{actions.search.matches.length?`${actions.search.matches.length} coincidencias para «${actions.search.query}».`:'No encontré carpetas con ese nombre en las ubicaciones recorridas.'}</p>{actions.search.opened&&<p role="status">Apertura solicitada: {actions.search.opened}</p>}{actions.search.incomplete&&<p className="search-limit">La búsqueda llegó a su límite de tiempo o resultados. Puede haber más coincidencias.</p>}<small>Se omiten carpetas del sistema, dependencias de desarrollo, enlaces y ubicaciones inaccesibles. No se leen archivos.{actions.search.skipped?` ${actions.search.skipped} ubicaciones no pudieron recorrerse.`:''}</small><div className="folder-matches">{actions.search.matches.map(m=><div key={m.id}><span><strong>{m.name}</strong><small>{m.path}</small></span><button disabled={actions.busy} onClick={()=>actions.openFolder(m.id)}>Abrir</button></div>)}</div><button disabled={actions.busy} onClick={async()=>{const within=await actions.pickFolder();if(within)actions.request(actions.search.text,{within})}}>Buscar dentro de una ubicación…</button></>}
 {actions.execution&&<ul className="execution-results">{actions.execution.results.map((r,i)=><li key={i}><strong>{r.ok?'✓':'!'} {r.name}</strong><small>{r.detail}</small></li>)}</ul>}
 {!actions.searching&&<form className="folder-search-form" onSubmit={e=>{e.preventDefault();actions.request('Buscá una carpeta llamada '+query)}}><label htmlFor="folder-query">Buscar otra carpeta por nombre</label><div><input id="folder-query" value={query} maxLength={120} onChange={e=>setQuery(e.target.value)} placeholder="Fotos de viajes"/><button disabled={actions.busy||!query.trim()}>Buscar</button></div></form>}
 </section>
}

function ChromeProfilePicker({appId,value,disabled,onChange}){
 const [profiles,setProfiles]=useState([]),[loading,setLoading]=useState(false),[error,setError]=useState('')
 const detect=async valid=>{setLoading(true);setError('');try{const result=await invoke('chrome_profiles',{appId});if(valid())setProfiles(result)}catch(e){if(valid())setError(String(e.message||e))}finally{if(valid())setLoading(false)}}
 useEffect(()=>{let active=true;setProfiles([]);detect(()=>active);return()=>{active=false}},[appId])
 return <div className="chrome-profile-picker"><label>Perfil de Chrome<select value={value} disabled={disabled||loading} onChange={e=>onChange(e.target.value)}><option value="">Sin especificar perfil</option>{value&&!profiles.some(p=>p.id===value)&&<option value={value}>{value} · sin detectar</option>}{profiles.map(p=><option key={p.id} value={p.id}>{p.name} ({p.id})</option>)}</select></label><button type="button" disabled={disabled||loading} onClick={()=>detect(()=>true)}>{loading?'Detectando perfiles…':'Volver a detectar perfiles'}</button>{error&&<p role="alert" className="ollama-error">{error}</p>}<small>Elegí el mismo perfil para Gmail y Jira. Si falta, abrilo una vez en Chrome y volvé a detectar.</small></div>
}
