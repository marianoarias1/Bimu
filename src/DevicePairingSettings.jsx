import React,{useCallback,useEffect,useId,useRef,useState} from 'react'
import {invoke,isTauri} from '@tauri-apps/api/core'
import {Link2,Monitor,Laptop,Unlink,Info,X,ChevronLeft,Copy,Check,RefreshCw,LoaderCircle,Clock3,Clipboard,Files,Star,MoreHorizontal} from 'lucide-react'
import {pairingView,canPair,remainingCodeTime,codeTimeLabel,limitSummary,validTransferLimit} from './device-pairing-ui'

export function PairInfo({title,children,above=false}){
 const [open,setOpen]=useState(false),root=useRef(null),trigger=useRef(null),panel=useRef(null),id=useId()
 const close=()=>{setOpen(false);trigger.current?.focus()}
 useEffect(()=>{
  if(!open)return
  panel.current?.focus()
  const outside=e=>{if(!root.current?.contains(e.target))setOpen(false)}
  const key=e=>{if(e.key==='Escape'){e.preventDefault();e.stopPropagation();setOpen(false);trigger.current?.focus()}}
  document.addEventListener('pointerdown',outside,true);document.addEventListener('focusin',outside);document.addEventListener('keydown',key,true)
  return()=>{document.removeEventListener('pointerdown',outside,true);document.removeEventListener('focusin',outside);document.removeEventListener('keydown',key,true)}
 },[open])
 return <div ref={root} className={'pair-info'+(above?' pair-info-above':'')}>
  <button ref={trigger} type="button" className="pair-icon-button" aria-label={title+' · más información'} aria-expanded={open} aria-controls={open?id:undefined} onClick={()=>setOpen(v=>!v)}><Info size={16}/></button>
  {open&&<div ref={panel} id={id} className="pair-info-popover" role="dialog" aria-label={title} tabIndex={-1}><span className="pair-popover-heading"><strong>{title}</strong><button type="button" className="pair-icon-button" aria-label="Cerrar información" onClick={close}><X size={14}/></button></span><div className="pair-popover-content">{children}</div></div>}
 </div>
}

export function DevicePairingSettings(){
 const native=isTauri()
 const [status,setStatus]=useState({phase:native?'loading':'idle'}),[route,setRoute]=useState('start'),[adding,setAdding]=useState(false)
 const [address,setAddress]=useState(''),[code,setCode]=useState(''),[error,setError]=useState(''),[notice,setNotice]=useState('')
 const [busy,setBusy]=useState(false),[peers,setPeers]=useState([]),[searching,setSearching]=useState(false),[searched,setSearched]=useState(false),[manual,setManual]=useState(false)
 const [editing,setEditing]=useState(false),[limit,setLimit]=useState('30'),[copied,setCopied]=useState(false),[now,setNow]=useState(Date.now()),[confirmForget,setConfirmForget]=useState(false)
 const alive=useRef(true),refreshing=useRef(false),scanning=useRef(false),operation=useRef(false),epoch=useRef(0),titleRef=useRef(null)
 useEffect(()=>{alive.current=true;return()=>{alive.current=false;epoch.current++}},[])
 const refresh=useCallback(async()=>{
  if(!native||refreshing.current||operation.current)return
  refreshing.current=true;const token=epoch.current
  try{const value=await invoke('device_pairing_status');if(alive.current&&token===epoch.current)setStatus(value);return value}
  catch(e){if(alive.current&&token===epoch.current){setError(String(e));setStatus(s=>s.phase==='loading'?{...s,phase:'idle'}:s)}}
  finally{refreshing.current=false}
 },[native])
 useEffect(()=>{refresh();const timer=setInterval(refresh,1000);return()=>clearInterval(timer)},[refresh])
 const discover=useCallback(async()=>{
  if(!native||scanning.current)return
  scanning.current=true;setSearching(true);const token=epoch.current
  try{const found=await invoke('device_pairing_discover');if(alive.current&&token===epoch.current){setPeers(found);setSearched(true)}}
  catch(e){if(alive.current&&token===epoch.current){setError(String(e));setSearched(true)}}
  finally{scanning.current=false;if(alive.current)setSearching(false)}
 },[native])
 const view=pairingView(status,route,adding),summary=limitSummary(status),remaining=remainingCodeTime(status.codeExpiresAt,now)
 useEffect(()=>{if(view!=='join')return;discover();const timer=setInterval(discover,4000);return()=>clearInterval(timer)},[discover,view])
 useEffect(()=>{if(view!=='code')return;setNow(Date.now());const timer=setInterval(()=>setNow(Date.now()),1000);return()=>clearInterval(timer)},[view,status.code])
 useEffect(()=>{if(!editing&&status.transferLimitMb)setLimit(String(status.transferLimitMb))},[status.transferLimitMb,editing])
 useEffect(()=>{setConfirmForget(false);setEditing(false);setCopied(false);titleRef.current?.focus()},[view])
 useEffect(()=>{if(status.phase==='connected'){setAdding(false);setRoute('start')}},[status.phase])
 useEffect(()=>{const close=e=>{for(const menu of document.querySelectorAll('.pair-device-menu[open]'))if(e.type==='keydown'?e.key==='Escape':!menu.contains(e.target)){menu.open=false;if(e.type==='keydown')menu.querySelector('summary')?.focus()}};document.addEventListener('pointerdown',close);document.addEventListener('keydown',close);return()=>{document.removeEventListener('pointerdown',close);document.removeEventListener('keydown',close)}},[])
 useEffect(()=>{if(!copied)return;const timer=setTimeout(()=>setCopied(false),2000);return()=>clearTimeout(timer)},[copied])
 const run=async(command,args={})=>{
  if(!native||operation.current)return false
  operation.current=true;epoch.current++;setBusy(true);setError('');setNotice('')
  try{
   const result=await invoke(command,args);if(!alive.current)return false
   if(result?.phase)setStatus(result)
   if(command==='device_pairing_disconnect'||command==='device_pairing_forget')setStatus(s=>({...s,phase:'idle',linked:command==='device_pairing_forget'?false:s.linked,code:null,message:null,codeExpiresAt:null}))
   const fresh=await invoke('device_pairing_status');if(alive.current)setStatus(fresh);return true
  }catch(e){if(alive.current)setError(String(e));return false}
  finally{operation.current=false;if(alive.current)setBusy(false)}
 }
 const goBack=async()=>{
  if(['waiting','connecting'].includes(status.phase)&&!await run('device_pairing_disconnect'))return
  epoch.current++;setAdding(false);setRoute('start');setAddress('');setCode('');setManual(false);setError('');setNotice('')
 }
 const host=()=>{setRoute('host');run('device_pairing_host')}
 const join=()=>{epoch.current++;setError('');setNotice('');setRoute('join')}
 const copyCode=async()=>{try{await navigator.clipboard.writeText(status.code||'');if(alive.current)setCopied(true)}catch{if(alive.current)setError('No se pudo copiar el código. Podés ingresarlo manualmente en la otra PC.')}}
 const submit=async e=>{e.preventDefault();if(canPair(address,code,peers,manual))await run('device_pairing_join',{address:address.trim(),code,management:peers.find(p=>p.address===address)?.management||false})}
 const saveLimit=async e=>{e.preventDefault();if(validTransferLimit(limit)&&await run('device_pairing_set_limit',{mb:Number(limit)})){setEditing(false);setNotice('Límite guardado.')}}
 const restoreLimit=async()=>{if(await run('device_pairing_set_limit',{mb:null})){setLimit('30');setEditing(false);setNotice('Se restableció el límite por defecto.')} }
 const forget=async()=>{if(await run('device_pairing_forget',{id:confirmForget?.id})){setRoute('start');setAddress('');setCode('');setPeers([]);setManual(false);setConfirmForget(false)}else setConfirmForget(false)}
 const connected=view==='connected',linkedView=connected||view==='reconnecting'
 const transferSettings=<div className="pair-limit-card">
  <div className="pair-limit-heading"><strong>{editing&&summary.inherited?'Tu preferencia':'Máximo por envío'}</strong><PairInfo title="Límite de archivos" above><p>De 1 a 250 MB por selección, incluyendo el total de una carpeta. Por defecto: 30 MB.</p><p>Si la PC que generó el código guardó un límite, ambas lo usan mientras estén conectadas. Si no, cada una usa el suyo al enviar.</p><p>Tu preferencia se conserva al desconectar. Restablecer elimina tu personalización. El ajuste no reserva memoria.</p><p>Las imágenes copiadas como imagen tienen un límite independiente.</p></PairInfo></div>
  {!editing?<><div className="pair-limit-value"><strong>{summary.value} MB</strong><button type="button" disabled={!native||busy} onClick={()=>{setLimit(String(status.transferLimitMb||30));setEditing(true)}}>{summary.editLabel}</button></div><small>{summary.source}</small></>:<form className="pair-limit-editor" onSubmit={saveLimit}>
   {summary.inherited&&<small>Ahora se usan {summary.value} MB de {status.peer||'la PC principal'}. Esta elección queda guardada como tu preferencia propia.</small>}
   <label>Límite por envío (MB)<input type="number" min="1" max="250" step="1" value={limit} onChange={e=>setLimit(e.target.value)} aria-invalid={!validTransferLimit(limit)} aria-describedby="pair-limit-range"/></label><small id="pair-limit-range">Entre 1 y 250 MB.</small>
   <div className="pair-editor-actions"><button type="button" className="pair-text-button" disabled={busy||!status.limitConfigured} onClick={restoreLimit}>Restablecer por defecto</button><button type="button" disabled={busy} onClick={()=>setEditing(false)}>Cancelar</button><button type="submit" className="pair-primary" disabled={busy||!validTransferLimit(limit)}>Guardar</button></div>
  </form>}
 </div>
 const connectionInfo=<PairInfo title="Sobre la conexión"><p>Ambas PCs deben estar en la misma red local.</p><p>Se sincronizan el portapapeles y los elementos de Archivos a mano. Los equipos vinculados se reconectan al abrir Bimu.</p><p>En Archivos a mano, los archivos se descargan al abrirlos desde el otro equipo.</p></PairInfo>
 const connectKnown=async id=>{if(status.phase==='connected'&&!await run('device_pairing_disconnect'))return;if(await run('device_pairing_connect',{id})){setRoute('start');setNotice('Se conectará cuando ambas PCs estén disponibles.')}}
 const addKnown=async()=>{if(['connected','connecting','waiting'].includes(status.phase)&&!await run('device_pairing_disconnect'))return;setAdding(true);setRoute('choose');setError('');setNotice('')}
 const known=status.devices||[],showKnown=known.length>0&&route==='start'
 const devicesPanel=<>
  <p className="pair-helper">Esta PC: {status.localName||'Este equipo'}</p>
  <div className="pair-saved-list" aria-label="Equipos guardados">{known.map(device=><article className="pair-saved-device" key={device.id}>
   <Monitor size={23}/><div className="pair-saved-name"><strong>{device.name}</strong><small>{device.pendingForget?'Vínculo revocado · aviso pendiente':device.connected?'Conectada':device.available?'Disponible':device.lastConnected?`Última conexión: ${new Date(device.lastConnected*1000).toLocaleDateString()}`:'Sin conexión'}</small></div>
   {!device.pendingForget&&<button className="pair-icon-button pair-favorite" aria-label={device.favorite?'Quitar de favoritos '+device.name:'Marcar como favorito '+device.name} aria-pressed={device.favorite} disabled={busy} onClick={()=>run('device_pairing_favorite',{id:device.favorite?null:device.id})}><Star size={16} fill={device.favorite?'currentColor':'none'}/></button>}
   {!device.pendingForget&&<button disabled={busy||!native} onClick={()=>device.connected?run('device_pairing_disconnect'):connectKnown(device.id)}>{device.connected?'Desconectar':'Conectar'}</button>}
   {!device.pendingForget&&<details className="pair-device-menu"><summary aria-label={'Opciones de '+device.name}><MoreHorizontal size={17}/></summary><button className="pair-danger pair-forget-action" disabled={busy} onClick={e=>{e.currentTarget.closest('details').open=false;setConfirmForget(device)}}><Unlink size={14}/>Olvidar equipo</button></details>}
  </article>)}</div>
  <button className="pair-primary" disabled={!native||busy} onClick={addKnown}>+ Vincular otra PC</button>
  <small className="pair-helper">Tu favorito tiene prioridad al reconectar. Solo se sincroniza con el equipo conectado.</small>
  {connected&&<div className="pair-sync-row"><span>Sincronización activa</span><span aria-label="Portapapeles y archivos"><Clipboard size={16}/><Files size={16}/></span></div>}
  {transferSettings}
 </>
 const back=<button type="button" className="pair-back" disabled={busy} onClick={goBack}><ChevronLeft size={14}/> Volver</button>
 const heading=text=><h2 ref={titleRef} tabIndex={-1}>{text}</h2>
 return <section className="behavior-card glass-card device-pairing pairing-flow" aria-label="Vincular computadoras">
  {(view==='start'||linkedView||view==='loading')&&<div className="pair-heading">{heading(linkedView?'Tus dispositivos':<><Link2 size={16}/> Vincular computadoras</>)}{connectionInfo}</div>}
  {error&&<p className="pair-feedback pair-error" role="alert">{error}</p>}
  {!error&&status.phase==='error'&&status.message&&<p className="pair-feedback pair-error" role="alert">{status.message}</p>}
  {notice&&<p className="pair-feedback" role="status">{notice}</p>}
  {adding&&view==='start'&&back}
  {view==='loading'?<div className="pair-progress" role="status"><LoaderCircle className="pair-spinner" size={18}/> Cargando dispositivos…</div>:view==='start'&&showKnown?devicesPanel:view==='start'?<>
   <p className="pair-helper">Ambas PCs deben estar en la misma red.</p>
   <div className="pair-choice"><Monitor size={25}/><div><h3>Empezar desde esta PC</h3><p>Generá un código para vincular tu otro equipo.</p><button type="button" className="pair-primary" disabled={!native||busy} onClick={host}>{busy?'Generando…':'Generar código'}</button></div></div>
   <div className="pair-choice"><Laptop size={25}/><div><h3>Ya tengo un código</h3><p>Buscá la PC que lo está mostrando.</p><button type="button" disabled={!native||busy} onClick={join}>Conectar con otra PC</button></div></div>
   <details className="pair-transfer-options"><summary>Opciones de transferencia</summary>{transferSettings}</details>
  </>:view==='code'?<>
   {back}<div className="pair-code-stage">{heading(remaining===0?'El código venció':'Ingresá este código en la otra PC')}
   <div className="pair-code-box"><strong aria-label={'Código '+(status.code||'').split('').join(' ')}>{(status.code||'').replace(/(\d{3})(\d{3})/,'$1 $2')}</strong><button type="button" className="pair-icon-button" aria-label={copied?'Código copiado':'Copiar código'} disabled={remaining===0} onClick={copyCode}>{copied?<Check size={16}/>:<Copy size={16}/>}</button></div>
   <div className="pair-code-meta"><span>Esta PC: <strong>{status.localName||'Este equipo'}</strong></span><span><Clock3 size={12}/>{codeTimeLabel(remaining)}</span></div>
   <ol className="pair-instructions"><li>Abrí Dispositivos en la otra PC.</li><li>Elegí <strong>{status.localName||'esta PC'}</strong> e ingresá el código.</li></ol><details className="pair-transfer-options pair-code-address"><summary>¿No aparece esta PC en el otro equipo?</summary><small>Ingresá esta dirección en la opción «¿No aparece tu PC?» del otro equipo.</small><code>{status.address}</code></details>
   {remaining===0?<button type="button" className="pair-primary" disabled={busy} onClick={host}>Generar otro código</button>:<div className="pair-progress" role="status"><LoaderCircle size={17} className="pair-spinner"/> Esperando a la otra PC…</div>}
   <button type="button" disabled={busy} onClick={goBack}>Cancelar</button></div>
  </>:view==='join'?<>
   <div className="pair-stage-heading">{back}{heading('Conectar con otra PC')}{connectionInfo}</div>
   <p className="pair-helper">Elegí la PC que está mostrando el código. Ambas deben estar en la misma red.</p>
   <form onSubmit={submit} className="pair-join-form">
    <div className="pair-peer-list" aria-label="Computadoras disponibles">
     {peers.map(peer=><button type="button" key={peer.address} className={'pair-peer'+(address===peer.address&&!manual?' is-selected':'')} aria-pressed={address===peer.address&&!manual} onClick={()=>{setAddress(peer.address);setCode('');setManual(false)}}><Monitor size={23}/><span><strong>{peer.name}</strong><small><span className="pair-status-dot"/> Disponible</small></span>{address===peer.address&&!manual&&<Check size={17}/>}</button>)}
     {!peers.length&&<p className="pair-empty" role="status">{searching?<><LoaderCircle size={16} className="pair-spinner"/> Buscando equipos…</>:searched?'Todavía no encontramos otra PC. Generá un código en ese equipo.':'Buscando equipos disponibles…'}</p>}
    </div>
    <label className="pair-field">Código de seis dígitos<input value={code} onChange={e=>setCode(e.target.value.replace(/\D/g,'').slice(0,6))} inputMode="numeric" autoComplete="one-time-code" pattern="[0-9]{6}" maxLength={6} placeholder="000000" required/></label>
    <button type="submit" className="pair-primary" disabled={!native||busy||!canPair(address,code,peers,manual)}>Vincular equipo</button>
    <div className="pair-join-footer"><button type="button" className="pair-text-button" disabled={searching||busy} onClick={()=>{setError('');discover()}}><RefreshCw size={13} className={searching?'pair-spinner':''}/>{searching?'Buscando…':'Actualizar búsqueda'}</button><button type="button" className="pair-text-button" aria-expanded={manual} onClick={()=>setManual(v=>!v)}>¿No aparece tu PC?</button></div>
    {manual&&<div className="pair-manual"><p>Revisá que ambas PCs estén en la misma red y que la otra esté mostrando un código. Si no aparece, ingresá su dirección local.</p><label className="pair-field">Dirección local<input value={address} onChange={e=>setAddress(e.target.value)} placeholder="192.168.1.10:12345" autoComplete="off"/></label></div>}
   </form>
  </>:linkedView?devicesPanel:view==='connecting'?<><div className="pair-progress-stage">{heading('Conectando los equipos')}<div className="pair-progress" role="status"><LoaderCircle size={20} className="pair-spinner"/><span>{status.message||'Verificando el código…'}</span></div></div><button type="button" disabled={busy} onClick={goBack}>Cancelar</button></>:<>{back}<div className="pair-progress-stage">{heading(busy?'Generando código…':'Generar un código')}<p className="pair-helper">Ambas PCs deben estar en la misma red.</p><button type="button" className="pair-primary" disabled={!native||busy} onClick={host}>{busy?<><LoaderCircle size={16} className="pair-spinner"/> Generando…</>:'Generar código'}</button></div></>}
  {confirmForget&&<ForgetDeviceDialog device={confirmForget} busy={busy} onCancel={()=>setConfirmForget(false)} onConfirm={forget}/>}
 </section>
}
function ForgetDeviceDialog({device,busy,onCancel,onConfirm}){
 const cancel=useRef(null),confirm=useRef(null),title=useId()
 useEffect(()=>{const previous=document.activeElement;cancel.current?.focus();return()=>previous?.focus?.()},[])
 return <div className="pair-dialog-backdrop" onKeyDown={e=>{if(e.key==='Escape'&&!busy){e.preventDefault();onCancel()}if(e.key==='Tab'){const target=e.shiftKey?(document.activeElement===cancel.current?confirm.current:cancel.current):(document.activeElement===confirm.current?cancel.current:confirm.current);e.preventDefault();target?.focus()}}}><section className="pair-dialog" role="alertdialog" aria-modal="true" aria-labelledby={title}><h3 id={title}>¿Olvidar {device.name}?</h3><p>El vínculo se eliminará también en la otra PC cuando esté disponible. Para volver a conectar vas a necesitar un código nuevo.</p><div><button ref={cancel} disabled={busy} onClick={onCancel}>Cancelar</button><button ref={confirm} className="pair-danger" disabled={busy} onClick={onConfirm}>{busy?'Olvidando…':'Olvidar equipo'}</button></div></section></div>
}
