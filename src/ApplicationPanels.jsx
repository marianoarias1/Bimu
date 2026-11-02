import React,{useEffect,useMemo,useState} from 'react'
import {Search,Plus,Check,RefreshCw,ChevronLeft,Pencil,MoreHorizontal,LoaderCircle} from 'lucide-react'
import {AppIcon} from './AppIcon'
import {PairInfo} from './DevicePairingSettings'
import {appNameKey,registeredApp,selectableApps} from './shortcut-ui'

function AppRow({item,apps,used}){
 const [editing,setEditing]=useState(false),[name,setName]=useState(item.name),[remove,setRemove]=useState(false),[menu,setMenu]=useState(false)
 const disabled=apps.busy||!!apps.pending
 return <div className="sc-app-entry"><div className="sc-app-row"><AppIcon item={item}/><span className="sc-app-name"><strong>{item.name}</strong><small>{item.available?'Disponible':'No encontrada'}</small></span>
  <button type="button" className="sc-icon-button" disabled={disabled} aria-label={'Editar nombre de '+item.name} title="Editar nombre" onClick={()=>{setName(item.name);setEditing(v=>!v)}}><Pencil size={15}/></button>
  <button type="button" className="sc-icon-button" disabled={disabled} aria-label={'Más opciones de '+item.name} aria-expanded={menu} onClick={()=>setMenu(v=>!v)}><MoreHorizontal size={17}/></button></div>
  {menu&&<div className="sc-row-menu-options"><details className="sc-options"><summary>Ver ubicación</summary><code className="sc-path">{item.target}</code></details><button type="button" disabled={disabled||used>0} onClick={()=>{setRemove(true);setMenu(false)}}>Quitar de Bimu</button>{used>0&&<small>Se usa en {used} {used===1?'atajo':'atajos'}. Editalos antes de quitarla.</small>}</div>}
  {editing&&<form className="sc-inline-editor" onSubmit={async e=>{e.preventDefault();if(await apps.update(item.id,name.trim(),item.aliases,item.remember))setEditing(false)}}><label>Nombre de la aplicación<input value={name} maxLength={80} onChange={e=>setName(e.target.value)} required/></label><div className="sc-form-footer"><button type="button" disabled={disabled} onClick={()=>setEditing(false)}>Cancelar</button><button className="sc-primary" disabled={disabled||!name.trim()}>Guardar nombre</button></div></form>}
  {remove&&<div className="sc-inline-confirm"><p>¿Quitar {item.name} de Bimu? No se desinstala la aplicación.</p><button type="button" disabled={disabled} onClick={()=>setRemove(false)}>Cancelar</button><button type="button" className="sc-danger" disabled={disabled} onClick={async()=>{if(await apps.remove(item.id))setRemove(false)}}>Quitar</button></div>}
 </div>
}
export function ApplicationSettings({apps,onBack,shortcuts=[]}){
 const [query,setQuery]=useState(''),[tab,setTab]=useState('available')
 useEffect(()=>{apps.discover()},[])
 const filtered=useMemo(()=>{const key=appNameKey(query);return(tab==='added'?apps.items:apps.discovered).filter(item=>appNameKey(item.name).includes(key))},[tab,apps.items,apps.discovered,query])
 return <section className="shortcuts-flow glass-card sc-library" aria-label="Aplicaciones para atajos">
  {onBack&&<button className="sc-back" type="button" onClick={onBack}><ChevronLeft size={14}/> Tus atajos</button>}
  <div className="sc-heading"><h2>Aplicaciones para atajos</h2><PairInfo title="Aplicaciones para atajos"><p>Podés elegir una aplicación directamente al crear un atajo. Esta lista permite administrar las que agregaste a Bimu.</p><p>Quitar una app de esta lista no la desinstala.</p></PairInfo></div>
  <div className="sc-type-tabs" aria-label="Listado de aplicaciones"><button type="button" aria-pressed={tab==='available'} onClick={()=>setTab('available')}>Disponibles en tu PC</button><button type="button" aria-pressed={tab==='added'} onClick={()=>setTab('added')}>Agregadas a Bimu</button></div>
  <label className="sc-search"><Search size={15}/><input value={query} onChange={e=>setQuery(e.target.value)} placeholder="Buscar una aplicación" aria-label="Buscar una aplicación"/></label>
  {apps.error&&<p role="alert" className="sc-error">{apps.error}</p>}
  <div className="sc-app-list" aria-label={tab==='added'?'Aplicaciones agregadas':'Aplicaciones disponibles'} aria-busy={apps.discovering}>
   {tab==='added'?filtered.map(item=><AppRow key={item.id} item={item} apps={apps} used={shortcuts.filter(s=>s.actions.some(a=>a.appId===item.id||a.browserId===item.id)).length}/>):filtered.map(item=>{const added=registeredApp(apps.items,item);return <div className="sc-app-row" key={item.id}><AppIcon item={added||item}/><span className="sc-app-name"><strong>{item.name}</strong></span>{added?<span className="sc-added"><Check size={13}/> Agregada</span>:<button type="button" disabled={apps.busy||!!apps.pending} onClick={()=>apps.addDiscovered(item.id)}><Plus size={14}/> Agregar</button>}</div>})}
   {!filtered.length&&<div className="sc-empty" role="status">{apps.discovering&&tab==='available'?<><LoaderCircle size={18} className="pair-spinner"/><strong>Buscando aplicaciones…</strong></>:<><strong>{query?'No encontramos esa aplicación':tab==='added'?'Todavía no agregaste aplicaciones':'No encontramos aplicaciones'}</strong><small>{query?'Probá otro nombre o elegí su archivo .exe.':'Podés elegir una app directamente al crear un atajo.'}</small></>}</div>}
  </div>
  <div className="sc-toolbar"><button type="button" disabled={apps.busy||!!apps.pending} onClick={apps.add}>Registrar archivo .exe</button><button type="button" disabled={apps.discovering||apps.busy} onClick={()=>{apps.discover(true);apps.refresh()}}><RefreshCw size={14} className={apps.discovering?'pair-spinner':''}/>{apps.discovering?'Actualizando…':'Actualizar'}</button></div>
 </section>
}
export function ApplicationPicker({apps,onChoose,onBack,disabled}){
 const [query,setQuery]=useState('')
 useEffect(()=>{apps.discover()},[])
 const items=useMemo(()=>selectableApps(apps.items,apps.discovered,query),[apps.items,apps.discovered,query])
 const choose=async item=>{const app=item.registered?item:await apps.ensureDiscovered(item);if(app)onChoose(app)}
 const pick=async()=>{const app=await apps.pick();if(app)onChoose(app)}
 return <div className="sc-picker">
  <label className="sc-search"><Search size={15}/><input value={query} onChange={e=>setQuery(e.target.value)} placeholder="Buscar una aplicación" aria-label="Buscar una aplicación"/></label>
  <div className="sc-subheading"><strong>Aplicaciones de esta PC</strong><button type="button" className="sc-icon-button" disabled={apps.discovering||disabled} aria-label="Actualizar aplicaciones" title="Actualizar aplicaciones" onClick={()=>apps.discover(true)}><RefreshCw size={14} className={apps.discovering?'pair-spinner':''}/></button></div>
  {apps.error&&<p className="sc-error" role="alert">{apps.error}</p>}
  <div className="sc-app-list" aria-label="Elegir aplicación" aria-busy={apps.discovering}>
   {items.map(item=><div key={item.id} className="sc-app-row"><AppIcon item={item}/><span className="sc-app-name"><strong>{item.name}</strong></span><button type="button" disabled={disabled||apps.busy||!!apps.pending} onClick={()=>choose(item)}><Plus size={14}/> Elegir</button></div>)}
   {!items.length&&<div className="sc-empty" role="status"><strong>{apps.discovering?'Buscando aplicaciones…':'No encontramos esa aplicación'}</strong><small>Podés buscar su ejecutable con el botón de abajo.</small></div>}
  </div>
  <div className="sc-toolbar"><button type="button" disabled={disabled||apps.busy||!!apps.pending} onClick={pick}>Elegir archivo .exe</button><small>Si no encontrás una app, podés buscar su ejecutable.</small></div>
  <div className="sc-form-footer"><button type="button" disabled={disabled||apps.busy} onClick={onBack}>Cancelar</button></div>
 </div>
}
