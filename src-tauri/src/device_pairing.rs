//! Explicit PAKE pairing, persistent authenticated reconnection and bounded LAN synchronization.
use crate::pairing_core::Role;
use crate::pairing_transport::{self, SecureChannel};
use crate::clipboard_protocol::{self, ClipboardPayload};
use crate::shelf_shared::ShelfMetadata;
use std::collections::{HashSet,HashMap,VecDeque};
use tauri_plugin_opener::OpenerExt;
use serde::Serialize;
use std::{net::{Ipv4Addr, Shutdown, SocketAddr, SocketAddrV4, TcpListener, TcpStream, UdpSocket}, sync::{Arc, Mutex}, thread, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Clone,Serialize)]#[serde(rename_all="camelCase")]pub struct SavedDeviceView{pub id:String,pub name:String,pub favorite:bool,pub connected:bool,pub available:bool,pub last_connected:u64,pub pending_forget:bool,pub management:bool}
#[derive(Clone, Serialize)]
#[serde(rename_all="camelCase")]
pub struct PairStatus { devices:Vec<SavedDeviceView>, automatic:bool, active_id:Option<String>, phase:String, code:Option<String>, address:Option<String>, peer:Option<String>, message:Option<String>, linked:bool, transfer_limit_mb:usize, effective_limit_mb:Option<usize>, limit_configured:bool, is_main:bool, uses_main_limit:bool, local_name:String, code_expires_at:Option<u64> }
impl Default for PairStatus { fn default()->Self { Self{devices:vec![],automatic:true,active_id:None,phase:"idle".into(),code:None,address:None,peer:None,message:None,linked:false,transfer_limit_mb:crate::transfer_limits::current_mb(),effective_limit_mb:None,limit_configured:crate::transfer_limits::configured(),is_main:false,uses_main_limit:false,local_name:device_name(),code_expires_at:None} } }
#[derive(Default)]
struct Inner { active_id:Option<String>, management:bool, requested_id:Option<String>, control:Option<String>, close_after_send:bool, previews:HashMap<String,Option<String>>, preview_sequence:u64,pending_previews:HashMap<String,(u64,Instant)>, serving_previews:HashSet<(String,u64)>, seen:HashMap<String,Instant>, generation:u64, status:PairStatus, socket:Option<TcpStream>, outgoing:Option<ClipboardPayload>, outgoing_revision:u64, preparing:bool, remote_shelf:Vec<ShelfMetadata>, remote_peer:Option<String>, hidden_shelf:HashSet<String>, shelf_messages:VecDeque<ClipboardPayload>, shelf_pending:Option<ShelfPending>, shelf_serving:bool, peer_limit_mb:Option<usize>, peer_limit_configured:bool, main:bool, limit_dirty:bool, shelf_dirty:bool }
struct ShelfPending {id:String,token:u64,name:String,started:Instant}
#[derive(Default,Clone)]
pub struct DevicePairing(Arc<Mutex<Inner>>);
fn private_ip(ip:Ipv4Addr)->bool { ip.is_private() || ip.is_link_local() }
fn local_ip()->Result<Ipv4Addr,String>{
 let socket=UdpSocket::bind("0.0.0.0:0").map_err(|_|"No se pudo detectar la red local")?;
 socket.connect("192.0.2.1:9").map_err(|_|"No se pudo detectar la dirección local")?;
 match socket.local_addr().map_err(|_|"No se pudo detectar la dirección local")? { SocketAddr::V4(addr) if private_ip(*addr.ip())=>Ok(*addr.ip()), _=>Err("No se encontró una dirección IPv4 de red local".into()) }
}
fn reset(inner:&mut Inner){
 inner.generation=inner.generation.wrapping_add(1);
 if let Some(socket)=inner.socket.take(){let _=socket.shutdown(Shutdown::Both);}
 inner.active_id=None;inner.management=false;inner.control=None;inner.close_after_send=false;inner.previews.clear();inner.pending_previews.clear();inner.serving_previews.clear();inner.remote_shelf.clear();inner.remote_peer=None;inner.hidden_shelf.clear();
 inner.shelf_dirty=true;inner.peer_limit_mb=None;inner.peer_limit_configured=false;inner.main=false;inner.limit_dirty=false;
 inner.outgoing=None;inner.outgoing_revision=inner.outgoing_revision.wrapping_add(1);inner.preparing=false;
 inner.shelf_messages.clear();inner.shelf_pending=None;inner.shelf_serving=false;
 inner.status=PairStatus::default();
}
fn update(shared:&DevicePairing, generation:u64, status:PairStatus)->bool {
 let Ok(mut inner)=shared.0.lock() else{return false};
 if inner.generation!=generation{return false} inner.status=status;true
}
fn fail(shared:&DevicePairing, generation:u64, msg:&str){
 if let Ok(mut inner)=shared.0.lock(){
  if inner.generation!=generation{return}
  if let Some(socket)=inner.socket.take(){let _=socket.shutdown(Shutdown::Both);}
  inner.active_id=None;inner.management=false;inner.control=None;inner.close_after_send=false;inner.previews.clear();inner.pending_previews.clear();inner.serving_previews.clear();inner.remote_shelf.clear();inner.remote_peer=None;inner.hidden_shelf.clear();
 inner.shelf_dirty=true;inner.peer_limit_mb=None;inner.peer_limit_configured=false;inner.main=false;inner.limit_dirty=false;
  inner.outgoing=None;inner.outgoing_revision=inner.outgoing_revision.wrapping_add(1);inner.preparing=false;
  inner.shelf_messages.clear();inner.shelf_pending=None;inner.shelf_serving=false;
  inner.status=PairStatus{phase:"error".into(),message:Some(msg.into()),..Default::default()};
 }
}
pub(crate) fn clipboard_connected(app:&AppHandle)->bool {
 let state=app.state::<DevicePairing>();
 let connected=state.0.lock().map(|i|i.status.phase=="connected").unwrap_or(false);connected
}
pub(crate) fn clipboard_notice(app:&AppHandle)->Option<String>{
 let state=app.state::<DevicePairing>();
 let notice=state.0.lock().ok().and_then(|inner|if inner.status.phase=="connected"{inner.status.message.clone()}else{None});notice
}
pub(crate) fn queue_clipboard(app:&AppHandle,payload:Option<ClipboardPayload>){
 let shared=app.state::<DevicePairing>().inner().clone();
 let mut inner=match shared.0.lock(){Ok(inner)=>inner,Err(_)=>return};
 if inner.status.phase!="connected"{return}
 inner.outgoing_revision=inner.outgoing_revision.wrapping_add(1);
 let revision=inner.outgoing_revision;let generation=inner.generation;
 inner.outgoing=None;inner.preparing=false;inner.status.message=None;
 match payload{
  Some(ClipboardPayload::Files(paths))=>{
   inner.preparing=true;inner.status.message=Some("Preparando archivos para enviar…".into());drop(inner);
   let app=app.clone();
   thread::spawn(move||{
    let limit=effective_bytes(&shared);
    let result=crate::clipboard_files::pack_with_limit(&paths,limit,&||shared.0.lock().map(|i|i.generation!=generation||i.outgoing_revision!=revision||i.status.phase!="connected").unwrap_or(true));
    let Ok(mut inner)=shared.0.lock() else{return};
    if inner.generation!=generation||inner.outgoing_revision!=revision||inner.status.phase!="connected"{return}
    inner.preparing=false;
    match result{Ok(bundle)=>{inner.outgoing=Some(ClipboardPayload::FileBundle(bundle));inner.status.message=None},Err(e)=>inner.status.message=Some(format!("No se pudieron compartir los archivos: {e}"))}
    drop(inner);let _=app.emit("clipboard-history-changed",());
   });
  },
  other=>{inner.outgoing=other;}
 }
}
fn session_limits(inner:&Inner)->(usize,usize){crate::transfer_limits::session_mb(crate::transfer_limits::current_mb(),inner.peer_limit_mb.unwrap_or(1),inner.main,crate::transfer_limits::configured(),inner.peer_limit_configured)}
fn effective_bytes(shared:&DevicePairing)->usize{shared.0.lock().map(|i|session_limits(&i).0*crate::transfer_limits::MIB).unwrap_or(0)}
fn receive_bytes(shared:&DevicePairing)->usize{shared.0.lock().map(|i|session_limits(&i).1*crate::transfer_limits::MIB).unwrap_or(0)}
fn take_outgoing(shared:&DevicePairing,generation:u64)->Option<ClipboardPayload>{
 shared.0.lock().ok().and_then(|mut inner|if inner.generation==generation{inner.outgoing.take()}else{None})
}
fn has_outgoing(shared:&DevicePairing,generation:u64)->bool{
 shared.0.lock().map(|inner|inner.generation==generation&&(inner.preparing||inner.outgoing.is_some())).unwrap_or(false)
}
/// Called after local shelf writes, after releasing its lock. Metadata changes
/// must not sit behind continuous clipboard traffic or a prepared file payload.
pub(crate) fn shelf_changed(app:&AppHandle){let shared=app.state::<DevicePairing>();if let Ok(mut inner)=shared.0.lock(){inner.shelf_dirty=true;};}
pub(crate) fn remote_shelf(app:&AppHandle)->Vec<crate::shelf::ShelfItem>{
 let state=app.state::<DevicePairing>();let Ok(inner)=state.0.lock() else{return vec![]};
 let connected=inner.status.phase=="connected";
 inner.remote_shelf.iter().filter(|item|!inner.hidden_shelf.contains(&item.id)).map(|item|crate::shelf::ShelfItem{
  preview_version:item.revision.clone(),peer_id:inner.active_id.clone(),id:format!("peer:{}",item.id),name:item.name.clone(),path:String::new(),kind:item.kind.clone(),available:connected&&item.available,remote:true,online:connected,origin:inner.remote_peer.clone(),busy:inner.shelf_pending.as_ref().map(|p|p.id==item.id).unwrap_or(false)
 }).collect()
}
pub(crate) fn hide_remote_shelf(app:&AppHandle,id:&str)->Result<(),String>{
 let state=app.state::<DevicePairing>();let mut inner=state.0.lock().map_err(|_|"Conexión ocupada")?;
 if !inner.remote_shelf.iter().any(|item|item.id==id){return Err("El elemento ya no está en la bandeja".into())}
 inner.hidden_shelf.insert(id.to_owned());Ok(())
}
pub(crate) fn request_remote_shelf(app:&AppHandle,id:&str)->Result<(),String>{
 if !crate::shelf_shared::valid_id(id){return Err("Archivo remoto inválido".into())}
 let state=app.state::<DevicePairing>();let mut inner=state.0.lock().map_err(|_|"Conexión ocupada")?;
 if inner.status.phase!="connected"{return Err("La otra PC está desconectada".into())}
 if inner.shelf_pending.is_some(){return Err("Ya hay un archivo en descarga".into())}
 let item=inner.remote_shelf.iter().find(|item|item.id==id).ok_or("El archivo ya no está en la otra bandeja")?;
 if !item.available{return Err("El archivo ya no está disponible en la otra PC".into())}
 let name=item.name.clone();let token=crate::random_core::token()?;
 inner.shelf_pending=Some(ShelfPending{id:id.into(),token,name,started:Instant::now()});
 inner.shelf_messages.push_back(ClipboardPayload::ShelfRequest{id:id.into(),token});drop(inner);
 let _=app.emit("shelf-changed",());Ok(())
}
fn queue_shelf_reply(shared:&DevicePairing,generation:u64,message:ClipboardPayload){
 if let Ok(mut inner)=shared.0.lock(){if inner.generation==generation&&inner.status.phase=="connected"&&inner.shelf_messages.len()<8{inner.shelf_messages.push_back(message);}}
}
fn handle_shelf_request(app:&AppHandle,shared:&DevicePairing,generation:u64,id:String,token:u64){
 let accepted={let Ok(mut inner)=shared.0.lock()else{return};if inner.generation!=generation||inner.status.phase!="connected"{return}if inner.shelf_serving{false}else{inner.shelf_serving=true;true}};
 if !accepted{queue_shelf_reply(shared,generation,ClipboardPayload::ShelfReply{id,token,bundle:None,error:Some("La otra PC ya está preparando un archivo".into())});return}
 let app=app.clone();let shared=shared.clone();
 thread::spawn(move||{
  let limit=effective_bytes(&shared);
  let result=crate::shelf::path_for_share(&app,&id).and_then(|path|crate::clipboard_files::pack_with_limit(&[path],limit,&||shared.0.lock().map(|i|i.generation!=generation||i.status.phase!="connected"||session_limits(&i).0*crate::transfer_limits::MIB<limit).unwrap_or(true)));
  let Ok(mut inner)=shared.0.lock()else{return};if inner.generation!=generation||inner.status.phase!="connected"{return}inner.shelf_serving=false;
  if inner.shelf_messages.len()>=8{return}
  let (bundle,error)=match result{Ok(bundle)=>(Some(bundle),None),Err(error)=>(None,Some(error.chars().take(45).collect()))};
  inner.shelf_messages.push_back(ClipboardPayload::ShelfReply{id,token,bundle,error});
 });
}
fn handle_shelf_reply(app:&AppHandle,shared:&DevicePairing,generation:u64,id:String,token:u64,bundle:Option<crate::clipboard_files::FileBundle>,error:Option<String>){
 let pending={let Ok(mut inner)=shared.0.lock()else{return};if inner.generation!=generation{return}
  if !inner.shelf_pending.as_ref().map(|p|p.id==id&&p.token==token).unwrap_or(false){return}inner.shelf_pending.take().unwrap()};
 let result=(||->Result<(),String>{
  if let Some(error)=error{return Err(error)}
  let bundle=bundle.ok_or("No se recibieron archivos")?;
  if bundle.single_root()!=Some(pending.name.as_str()){return Err("El archivo recibido no coincide con el solicitado".into())}
  if shared.0.lock().map(|i|i.generation!=generation||i.status.phase!="connected").unwrap_or(true){return Err("La otra PC se desconectó".into())}
  let root=app.path().app_data_dir().map_err(|_|"No se pudo ubicar la carpeta de Bimu")?.join("archivos-a-mano-recibidos");
  std::fs::create_dir_all(&root).map_err(|_|"No se pudo crear la carpeta de recepción")?;
  let destination=root.join(format!("{:016x}",crate::random_core::token()?));
  let paths=crate::clipboard_files::unpack_with_limit(&bundle,&destination,receive_bytes(shared))?;
  if shared.0.lock().map(|i|i.generation!=generation||i.status.phase!="connected").unwrap_or(true){let _=std::fs::remove_dir_all(&destination);return Err("La otra PC se desconectó".into())}
  let path=paths.first().ok_or("No se recibió el archivo solicitado")?;
  app.opener().open_path(path.to_string_lossy().into_owned(),None::<&str>).map_err(|_|format!("Se descargó en {}, pero no se pudo abrir",path.display()))?;
  Ok(())
 })();
 let _=app.emit("shelf-changed",());if let Err(error)=result{let _=app.emit("shelf-error",error);}
}
fn handle_incoming(app:&AppHandle,shared:&DevicePairing,generation:u64,payload:ClipboardPayload,peer:&str)->Result<(),String>{
 if !matches!(&payload,ClipboardPayload::ConnectionControl{..})&&shared.0.lock().map(|i|i.generation!=generation||i.status.phase!="connected").unwrap_or(true){return Ok(())}
 match payload{
  ClipboardPayload::ConnectionControl{action}=>{
   let mut inner=shared.0.lock().map_err(|_|"Conexión ocupada")?;if inner.generation!=generation||!inner.management{return Err("Control de conexión no habilitado".into())}
   if action=="disconnect"{crate::trusted_pairing::edit(app,|b|{b.automatic=false;if let Some(d)=b.devices.iter_mut().find(|d|Some(&d.id)==inner.active_id.as_ref()){d.paused=true;d.pause_pending=false}Ok(())})?;inner.control=Some("disconnected".into());inner.close_after_send=true;inner.outgoing=None;inner.status.phase="disconnecting".into();}
   else{crate::trusted_pairing::edit(app,|b|{if let Some(d)=b.devices.iter_mut().find(|d|Some(&d.id)==inner.active_id.as_ref()){d.paused=true;d.pause_pending=false}Ok(())})?;inner.close_after_send=true;inner.outgoing=None;inner.status.phase="disconnecting".into();}Ok(())
  },
  ClipboardPayload::PreviewRequest{id,token}=>{
   {let mut inner=shared.0.lock().map_err(|_|"Conexión ocupada")?;if inner.generation!=generation||!inner.management{return Err("Vista previa no habilitada".into())}if inner.serving_previews.len()>=8||!inner.serving_previews.insert((id.clone(),token)){return Ok(())}}
   let path=crate::shelf::path_for_share(app,&id).ok();let shared=shared.clone();
   tauri::async_runtime::spawn(async move{let preview=match path{Some(p)=>crate::file_thumbnails::prepare(p).await,None=>None};if let Ok(mut inner)=shared.0.lock(){if inner.generation==generation{inner.serving_previews.remove(&(id.clone(),token));if inner.shelf_messages.len()<24{inner.shelf_messages.push_back(ClipboardPayload::PreviewReply{id,token,preview});}}}});Ok(())
  },
  ClipboardPayload::PreviewReply{id,token,preview}=>{let mut inner=shared.0.lock().map_err(|_|"Conexión ocupada")?;if inner.generation==generation&&inner.management&&inner.pending_previews.get(&id).map(|(value,_)|*value)==Some(token){inner.pending_previews.remove(&id);let ready=preview.is_some();if ready{inner.previews.insert(id,preview);}drop(inner);if ready{let _=app.emit("file-preview-ready",());}}Ok(())},
  ClipboardPayload::Limits{mb,configured}=>{
   crate::transfer_limits::validate(mb)?;
   if let Ok(mut inner)=shared.0.lock(){if inner.generation==generation{inner.peer_limit_mb=Some(mb);inner.peer_limit_configured=configured;inner.outgoing=None;inner.outgoing_revision=inner.outgoing_revision.wrapping_add(1);inner.preparing=false;inner.shelf_messages.retain(|m|!matches!(m,ClipboardPayload::ShelfReply{..}));}}
   Ok(())
  },
  ClipboardPayload::ShelfCatalog(items)=>{
   crate::shelf_shared::validate(&items)?;
   let mut changed=false;if let Ok(mut inner)=shared.0.lock(){if inner.generation==generation&&inner.remote_shelf!=items{let invalidated:Vec<String>=inner.remote_shelf.iter().filter(|old|!items.iter().any(|new|new.id==old.id&&new.revision==old.revision)).map(|old|old.id.clone()).collect();for id in invalidated{inner.previews.remove(&id);inner.pending_previews.remove(&id);}inner.hidden_shelf.retain(|id|items.iter().any(|item|&item.id==id));inner.remote_shelf=items;changed=true}}
   if changed{let _=app.emit("shelf-changed",());}Ok(())
  },
  ClipboardPayload::ShelfRequest{id,token}=>{handle_shelf_request(app,shared,generation,id,token);Ok(())},
  ClipboardPayload::ShelfReply{id,token,bundle,error}=>{handle_shelf_reply(app,shared,generation,id,token,bundle,error);Ok(())},
  other=>{if !has_outgoing(shared,generation){crate::clipboard_history::apply_remote(app,other,peer,receive_bytes(shared))?;}Ok(())}
 }
}
fn next_shelf_or_clipboard(app:&AppHandle,shared:&DevicePairing,generation:u64,last:&mut Option<Vec<ShelfMetadata>>,last_check:&mut Instant)->Result<Option<ClipboardPayload>,String>{
 {let mut inner=shared.0.lock().map_err(|_|"Conexión ocupada")?;if inner.generation!=generation{return Ok(None)}if let Some(action)=inner.control.take(){return Ok(Some(ClipboardPayload::ConnectionControl{action}))}if inner.close_after_send{return Ok(None)}}

 if let Ok(mut inner)=shared.0.lock(){if inner.generation!=generation{return Ok(None)}if inner.limit_dirty{inner.limit_dirty=false;return Ok(Some(ClipboardPayload::Limits{mb:crate::transfer_limits::current_mb(),configured:crate::transfer_limits::configured()}))}if let Some(message)=inner.shelf_messages.pop_front(){return Ok(Some(message))}}
 // Catalog publication is independent of the file-size limit and takes a
 // bounded turn before clipboard payloads. Re-advertise every three seconds so
 // a stale cache can recover without disconnecting or touching local limits.
 let dirty={let mut inner=shared.0.lock().map_err(|_|"Conexión ocupada")?;let dirty=inner.shelf_dirty;inner.shelf_dirty=false;dirty};
 if crate::shelf_shared::catalog_due(last.is_none(),dirty,last_check.elapsed()){
  *last_check=Instant::now();let capable=shared.0.lock().map(|i|i.management).unwrap_or(false);let snapshot=crate::shelf_shared::for_peer(crate::shelf::shared_snapshot(app)?,capable);
  *last=Some(snapshot.clone());return Ok(Some(ClipboardPayload::ShelfCatalog(snapshot)))
 }
 Ok(take_outgoing(shared,generation))
}
fn rollback_trust(app:&AppHandle,_shared:&DevicePairing,trust:&crate::trusted_pairing::Trust){let _=crate::trusted_pairing::remove_failed(app,&trust.tag());}
fn device_name()->String {
 let value=std::env::var("COMPUTERNAME").unwrap_or_else(|_|"Otra PC".into());
 let clean:String=value.chars().filter(|c|!c.is_control()).take(64).collect();
 if clean.trim().is_empty(){"Otra PC".into()}else{clean}
}
fn session(app:AppHandle,shared:DevicePairing,generation:u64,channel:SecureChannel,role:Role,management:bool){
 if shared.0.lock().map(|i|i.generation!=generation).unwrap_or(true){return}
 let trust=match crate::trusted_pairing::load(&app){Ok(Some(t))=>t,_=>{fail(&shared,generation,"No se pudo leer el vínculo autenticado");return}};
 session_with_trust(app,shared,generation,channel,role,trust,management,true)
}
fn session_with_trust(app:AppHandle, shared:DevicePairing, generation:u64, mut channel:SecureChannel, role:Role,trust:crate::trusted_pairing::Trust,management:bool,resume:bool){
 let socket=match channel.control_stream(){Ok(s)=>s,Err(_)=>{fail(&shared,generation,"No se pudo mantener la conexión.");return}};
 {let Ok(mut inner)=shared.0.lock() else{return};if inner.generation!=generation{return}inner.socket=Some(socket);}
 let base_hello=crate::transfer_limits::hello(&device_name(),crate::transfer_limits::current_mb(),crate::transfer_limits::configured()).unwrap();
 let mut capable=management;
 let exchange=(||->Result<(String,usize,bool),String>{
  let received=match role{
   Role::Client=>{let hello=if capable{base_hello.replacen("bimu-sync-v5","bimu-sync-v6",1)}else{base_hello.clone()};channel.send(hello.as_bytes())?;channel.receive()?},
   Role::Host=>{let bytes=channel.receive()?;capable=bytes.starts_with(b"bimu-sync-v6\n");let hello=if capable{base_hello.replacen("bimu-sync-v5","bimu-sync-v6",1)}else{base_hello.clone()};channel.send(hello.as_bytes())?;bytes}
  };
  if received.len()>512{return Err("Nombre de equipo inválido".into())}let value=String::from_utf8(received).map_err(|_|"Nombre inválido")?;capable&=value.starts_with("bimu-sync-v6\n");crate::transfer_limits::parse_hello(&value)
 })();
 let (peer,peer_limit,peer_configured)=match exchange{Ok(peer)=>peer,Err(_)=>{fail(&shared,generation,"No se pudo verificar el equipo. Actualizá Bimu en ambas PCs para usar los límites configurables.");return}};
 let id=trust.tag();let book=match crate::trusted_pairing::book(&app){Ok(b)=>b,Err(e)=>{fail(&shared,generation,&e);return}};
 let Some(record)=book.devices.iter().find(|d|d.id==id)else{fail(&shared,generation,"El vínculo ya no está guardado");return};
 if capable{
  let intent=crate::connection_control::Intent{version:1,revoked:record.revoked,paused:!book.automatic||record.paused||record.pause_pending,resume};
  match crate::connection_control::exchange(&mut channel,role,intent){
   Ok(crate::connection_control::Decision::Forget)=>{if let Err(e)=crate::trusted_pairing::edit(&app,|b|{let stop=b.selected.as_deref()==Some(id.as_str());b.revoke(&id,true)?;if stop{b.automatic=false}Ok(())}){fail(&shared,generation,&e);return}if let Ok(mut inner)=shared.0.lock(){if inner.generation==generation{reset(&mut inner);}}let _=app.emit("shelf-changed",());return},
   Ok(crate::connection_control::Decision::Pause)=>{let _=crate::trusted_pairing::edit(&app,|b|{if b.selected.as_deref()==Some(id.as_str()){b.automatic=false}if let Some(d)=b.devices.iter_mut().find(|d|d.id==id){d.paused=true;d.pause_pending=false;}Ok(())});if let Ok(mut inner)=shared.0.lock(){if inner.generation==generation{reset(&mut inner);}}return},
   Ok(crate::connection_control::Decision::Sync)=>{},Err(e)=>{fail(&shared,generation,&e);return}
  }
 }else if record.revoked||record.paused||!book.automatic{if let Ok(mut inner)=shared.0.lock(){if inner.generation==generation{reset(&mut inner);}}return}
 let saved=crate::trusted_pairing::edit(&app,|b|{let d=b.devices.iter_mut().find(|d|d.id==id&&!d.revoked).ok_or("El vínculo fue revocado")?;d.name=peer.clone();d.management=capable;d.last_connected=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();d.paused=false;d.pause_pending=false;b.selected=Some(id.clone());b.automatic=true;Ok(())});
 if let Err(e)=saved{fail(&shared,generation,&e);return}
 {let Ok(mut inner)=shared.0.lock()else{return};if inner.generation!=generation{return}
  if inner.remote_peer.as_deref()!=Some(peer.as_str()){inner.remote_shelf.clear();inner.hidden_shelf.clear();}
  inner.remote_peer=Some(peer.clone());inner.peer_limit_mb=Some(peer_limit);inner.peer_limit_configured=peer_configured;inner.main=trust.host;inner.active_id=Some(id.clone());inner.management=capable;inner.requested_id=None;}
 if !update(&shared,generation,PairStatus{phase:"connected".into(),peer:Some(peer.clone()),..Default::default()}){return}
 let _=app.emit("shelf-changed",());
 let mut last_shelf:Option<Vec<ShelfMetadata>>=None;let mut shelf_checked=Instant::now();
 loop {
  thread::sleep(Duration::from_millis(350));
  if shared.0.lock().map(|i|i.generation!=generation).unwrap_or(true){return}
  let result=(||->Result<(),String>{
   match role{
    Role::Client=>{
     clipboard_protocol::send_turn(&mut channel,next_shelf_or_clipboard(&app,&shared,generation,&mut last_shelf,&mut shelf_checked)?)?;
     if let Some(payload)=clipboard_protocol::receive_turn_with_limit(&mut channel,receive_bytes(&shared))?{handle_incoming(&app,&shared,generation,payload,&peer)?;}
    },
    Role::Host=>{
     if let Some(payload)=clipboard_protocol::receive_turn_with_limit(&mut channel,receive_bytes(&shared))?{handle_incoming(&app,&shared,generation,payload,&peer)?;}
     clipboard_protocol::send_turn(&mut channel,next_shelf_or_clipboard(&app,&shared,generation,&mut last_shelf,&mut shelf_checked)?)?;
    },
   }Ok(())
  })();
  if let Err(e)=result{let had_pending=shared.0.lock().map(|i|i.generation==generation&&i.shelf_pending.is_some()).unwrap_or(false);fail(&shared,generation,&format!("Se interrumpió la sincronización: {e}. Intentando reconectar."));let _=app.emit("shelf-changed",());if had_pending{let _=app.emit("shelf-error","Se perdió la conexión durante la descarga.");}return}
  {if let Ok(mut inner)=shared.0.lock(){if inner.generation==generation&&inner.close_after_send&&inner.control.is_none(){reset(&mut inner);let _=app.emit("shelf-changed",());return}}}
  let timed_out={if let Ok(mut inner)=shared.0.lock(){if inner.generation==generation&&inner.shelf_pending.as_ref().map(|p|p.started.elapsed()>Duration::from_secs(60)).unwrap_or(false){inner.shelf_pending=None;true}else{false}}else{false}};
  if timed_out{let _=app.emit("shelf-changed",());let _=app.emit("shelf-error","La descarga tardó demasiado. Intentá nuevamente.");}
 }
}
#[tauri::command]
pub fn device_pairing_status(app:AppHandle,state:State<'_,DevicePairing>)->Result<PairStatus,String>{
 let inner=state.0.lock().map_err(|_|"Conexión ocupada")?;let mut status=inner.status.clone();
 status.transfer_limit_mb=crate::transfer_limits::current_mb();status.limit_configured=crate::transfer_limits::configured();
 status.is_main=inner.main;status.uses_main_limit=inner.status.phase=="connected"&&if inner.main{status.limit_configured}else{inner.peer_limit_configured};
 status.effective_limit_mb=if inner.status.phase=="connected"{Some(session_limits(&inner).0)}else{None};drop(inner);
 let book=crate::trusted_pairing::book(&app)?;let inner=state.0.lock().map_err(|_|"Conexión ocupada")?;
 status.active_id=inner.active_id.clone();status.automatic=book.automatic;status.linked=book.devices.iter().any(|d|!d.revoked);
 if status.phase!="connected"{status.is_main=book.selected().map(|d|d.trust.host).unwrap_or(false)}
 status.devices=book.ranked().iter().filter(|d|!d.revoked||!d.notified).map(|d|SavedDeviceView{id:d.id.clone(),name:d.name.clone(),favorite:book.favorite.as_ref()==Some(&d.id),connected:inner.status.phase=="connected"&&inner.active_id.as_ref()==Some(&d.id),available:inner.seen.get(&d.id).map(|t|t.elapsed()<Duration::from_secs(5)).unwrap_or(false),last_connected:d.last_connected,pending_forget:d.revoked,management:d.management}).collect();Ok(status)
}
#[tauri::command]
pub async fn device_pairing_disconnect(app:AppHandle,state:State<'_,DevicePairing>)->Result<(),String>{
 let id={let mut inner=state.0.lock().map_err(|_|"Conexión ocupada")?;inner.requested_id=None;let id=inner.active_id.clone();crate::trusted_pairing::edit(&app,|b|{b.automatic=false;if let Some(d)=b.devices.iter_mut().find(|d|Some(&d.id)==id.as_ref()){d.paused=true;d.pause_pending=true}Ok(())})?;
 if inner.status.phase=="connected"&&inner.management{inner.control=Some("disconnect".into());inner.outgoing=None;inner.status.phase="disconnecting".into();}else{reset(&mut inner);}id};
 for _ in 0..50{tokio::time::sleep(Duration::from_millis(100)).await;let mut inner=state.0.lock().map_err(|_|"Conexión ocupada")?;if inner.status.phase!="disconnecting"{break}if inner.active_id!=id{break}}
 {let mut inner=state.0.lock().map_err(|_|"Conexión ocupada")?;if inner.status.phase=="disconnecting"{reset(&mut inner);}}
 let _=app.emit("shelf-changed",());Ok(())
}
#[tauri::command]
pub fn device_pairing_forget(id:Option<String>,app:AppHandle,state:State<'_,DevicePairing>)->Result<(),String>{
 let mut inner=state.0.lock().map_err(|_|"Conexión ocupada")?;let id=id.or_else(||inner.active_id.clone()).or_else(||crate::trusted_pairing::book(&app).ok().and_then(|b|b.selected)).ok_or("Elegí un equipo")?;
 if inner.status.phase=="connected"&&inner.active_id.as_ref()==Some(&id)&&!inner.management{return Err("Actualizá Bimu en ambas PCs para olvidar el vínculo en los dos equipos.".into())}
 crate::trusted_pairing::edit(&app,|b|{b.revoke(&id,false)?;if inner.active_id.as_ref()==Some(&id){b.automatic=false;}Ok(())})?;
 if inner.active_id.as_ref()==Some(&id){reset(&mut inner);inner.requested_id=None;}drop(inner);let _=app.emit("shelf-changed",());Ok(())
}
#[tauri::command]
pub fn device_pairing_connect(id:String,app:AppHandle,state:State<'_,DevicePairing>)->Result<(),String>{
 let mut inner=state.0.lock().map_err(|_|"Conexión ocupada")?;if inner.status.phase=="connected"{return Err("Desconectá el equipo actual antes de cambiar.".into())}
 crate::trusted_pairing::edit(&app,|b|{if !b.devices.iter().any(|d|d.id==id&&!d.revoked){return Err("Equipo no disponible".into())}if let Some(d)=b.devices.iter_mut().find(|d|d.id==id){d.paused=false;}b.selected=Some(id.clone());b.automatic=true;Ok(())})?;
 reset(&mut inner);inner.requested_id=Some(id);Ok(())
}
#[tauri::command]
pub fn device_pairing_favorite(id:Option<String>,app:AppHandle)->Result<(),String>{crate::trusted_pairing::edit(&app,|b|{if let Some(id)=&id{if !b.devices.iter().any(|d|&d.id==id&&!d.revoked){return Err("Equipo no disponible".into())}}b.favorite=id;Ok(())})}
pub(crate)fn request_preview(app:&AppHandle,id:&str)->Result<Option<String>,String>{
 if !crate::shelf_shared::valid_id(id){return Err("Archivo inválido".into())}let state=app.state::<DevicePairing>();let mut inner=state.0.lock().map_err(|_|"Conexión ocupada")?;
 if !inner.remote_shelf.iter().any(|d|d.id==id){return Ok(None)}if let Some(value)=inner.previews.get(id){return Ok(value.clone())}
 if inner.pending_previews.get(id).map(|(_,at)|at.elapsed()<Duration::from_secs(5)).unwrap_or(false){return Ok(None)}
 inner.pending_previews.remove(id);
 if inner.management&&inner.status.phase=="connected"&&inner.pending_previews.len()<4&&inner.shelf_messages.len()<24{inner.preview_sequence=inner.preview_sequence.wrapping_add(1);let token=inner.preview_sequence;inner.pending_previews.insert(id.into(),(token,Instant::now()));inner.shelf_messages.push_back(ClipboardPayload::PreviewRequest{id:id.into(),token});}Ok(None)
}
#[tauri::command]
pub fn device_pairing_host(app:AppHandle,state:State<'_,DevicePairing>)->Result<PairStatus,String>{
 if state.0.lock().map(|i|matches!(i.status.phase.as_str(),"connected"|"disconnecting")).unwrap_or(true){return Err("Desconectá el equipo actual antes de vincular otro.".into())}
 let ip=local_ip()?;
 let listener=TcpListener::bind("0.0.0.0:0").map_err(|_|"No se pudo abrir la conexión local")?;
 listener.set_nonblocking(true).map_err(|_|"No se pudo iniciar la conexión")?;
 let port=listener.local_addr().map_err(|_|"No se pudo obtener el puerto")?.port();
 let code=crate::random_core::pairing_code()?;
 let shared=state.inner().clone();
 let (generation,status)={let mut inner=shared.0.lock().map_err(|_|"Conexión ocupada")?;reset(&mut inner);inner.status=PairStatus{phase:"waiting".into(),code:Some(code.clone()),address:Some(SocketAddrV4::new(ip,port).to_string()),code_expires_at:Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()+120),..Default::default()};(inner.generation,inner.status.clone())};
 let _=app.emit("shelf-changed",());
 thread::spawn(move||{
  let discovery=crate::pairing_discovery::advertise(&device_name(),port).ok();
  let mut announced=Instant::now();
  let expiry=Instant::now()+Duration::from_secs(120);
  loop {
   if shared.0.lock().map(|i|i.generation!=generation).unwrap_or(true){return}
   if Instant::now()>=expiry{fail(&shared,generation,"El código venció. Generá uno nuevo.");return}
   if announced.elapsed()>=Duration::from_millis(400){if let Some(socket)=&discovery{crate::pairing_discovery::send(socket,&device_name(),port)}announced=Instant::now();}
   match listener.accept(){
    Ok((stream,addr))=>{
     if !matches!(addr,SocketAddr::V4(v) if private_ip(*v.ip())){
      if let Ok(mut inner)=shared.0.lock(){if inner.generation==generation{
       inner.status.message=Some(format!("Se rechazó una conexión desde {addr}: su dirección no figura como red local privada."));
      }}
      continue
     }
     // Each code permits one attempt, including failed or stalled handshakes.
     update(&shared,generation,PairStatus{phase:"connecting".into(),message:Some("Verificando el código…".into()),..Default::default()});
     match pairing_transport::pair(stream,Role::Host,&code){
      Ok(mut channel) if Instant::now()<expiry=>match crate::trusted_pairing::establish(&mut channel,Role::Host,|trust|{let inner=shared.0.lock().map_err(|_|"Conexión ocupada")?;if inner.generation!=generation{return Err("Vinculación cancelada".into())}crate::trusted_pairing::save(&app,trust)},|trust|rollback_trust(&app,&shared,trust)){Ok(())=>session(app,shared,generation,channel,Role::Host,true),Err(e)=>fail(&shared,generation,&e)},
      Ok(_)=>fail(&shared,generation,"El código venció durante la vinculación. Generá uno nuevo."),
      Err(e)=>fail(&shared,generation,&format!("Falló la verificación entre equipos: {e}. Generá un código nuevo.")),
     }return
    },
    Err(e) if e.kind()==std::io::ErrorKind::WouldBlock=>thread::sleep(Duration::from_millis(100)),
    Err(_)=>{fail(&shared,generation,"Se interrumpió la espera de conexión.");return}
   }
  }
 });Ok(status)
}
#[tauri::command]
pub fn device_pairing_join(address:String,code:String,management:Option<bool>,app:AppHandle,state:State<'_,DevicePairing>)->Result<(),String>{
 if state.0.lock().map(|i|matches!(i.status.phase.as_str(),"connected"|"disconnecting")).unwrap_or(true){return Err("Desconectá el equipo actual antes de vincular otro.".into())}
 let addr:SocketAddrV4=address.trim().parse().map_err(|_|"Ingresá la dirección local y el puerto que muestra la otra PC")?;
 if !private_ip(*addr.ip())||addr.port()==0{return Err("Ingresá una dirección IPv4 de red local válida".into())}
 if code.len()!=6||!code.bytes().all(|b|b.is_ascii_digit()){return Err("El código debe tener seis números".into())}
 let shared=state.inner().clone();
 let generation={let mut inner=shared.0.lock().map_err(|_|"Conexión ocupada")?;reset(&mut inner);inner.status=PairStatus{phase:"connecting".into(),message:Some("Conectando…".into()),..Default::default()};inner.generation};
 let _=app.emit("shelf-changed",());
 thread::spawn(move||{
  match TcpStream::connect_timeout(&SocketAddr::V4(addr),Duration::from_secs(5)){
   Ok(stream)=>match pairing_transport::pair(stream,Role::Client,&code){
    Ok(mut channel)=>match crate::trusted_pairing::establish(&mut channel,Role::Client,|trust|{let inner=shared.0.lock().map_err(|_|"Conexión ocupada")?;if inner.generation!=generation{return Err("Vinculación cancelada".into())}crate::trusted_pairing::save(&app,trust)},|trust|rollback_trust(&app,&shared,trust)){Ok(())=>session(app,shared,generation,channel,Role::Client,management.unwrap_or(false)),Err(e)=>fail(&shared,generation,&e)},
    Err(e)=>fail(&shared,generation,&format!("Falló la verificación entre equipos: {e}. Generá un código nuevo.")),
   },
   Err(e)=>fail(&shared,generation,&format!("No se pudo contactar a la otra PC: {e}. Revisá la dirección y la red local.")),
  }
 });Ok(())
}

#[tauri::command]
pub async fn device_pairing_discover()->Result<Vec<crate::pairing_discovery::DiscoveredPeer>,String>{
 tauri::async_runtime::spawn_blocking(crate::pairing_discovery::scan).await.map_err(|_|"No se pudo buscar equipos".to_string())?
}


// All discovery hints are untrusted. Every candidate still proves the stored PSK.
pub fn start_reconnect(app:AppHandle){
 let shared=app.state::<DevicePairing>().inner().clone();
 thread::spawn(move||{
  let mut listeners:HashMap<String,TcpListener>=HashMap::new();let mut hints:HashMap<String,(SocketAddrV4,bool,Instant)>=HashMap::new();let mut attempts:HashMap<String,Instant>=HashMap::new();
  let instance=match crate::random_core::token(){Ok(n)=>format!("{n:x}"),Err(_)=>return};let mut receiver:Option<UdpSocket>=None;let mut sender:Option<UdpSocket>=None;let mut announced=Instant::now()-Duration::from_secs(1);
  loop{
   if app.get_webview_window("main").is_none(){return}
   let book=match crate::trusted_pairing::book(&app){Ok(b)=>b,Err(_)=>{thread::sleep(Duration::from_secs(2));continue}};
   let records=book.ranked();listeners.retain(|id,_|records.iter().any(|d|&d.id==id));hints.retain(|_,(_,_,t)|t.elapsed()<Duration::from_secs(5));
   if receiver.is_none(){receiver=UdpSocket::bind("0.0.0.0:45949").ok();if let Some(udp)=&receiver{let _=udp.set_nonblocking(true);}}
   if sender.is_none(){sender=UdpSocket::bind("0.0.0.0:0").ok();if let Some(udp)=&sender{let _=udp.set_broadcast(true);}}
   for d in &records{if !listeners.contains_key(&d.id){if let Ok(listener)=TcpListener::bind("0.0.0.0:0"){let _=listener.set_nonblocking(true);listeners.insert(d.id.clone(),listener);}}}
   let idle=shared.0.lock().map(|i|matches!(i.status.phase.as_str(),"idle"|"error")).unwrap_or(false);
   if idle&&announced.elapsed()>=Duration::from_millis(600){
    if let Some(udp)=&sender{for d in &records{if let Some(port)=listeners.get(&d.id).and_then(|l|l.local_addr().ok()).map(|a|a.port()){
     let message=format!("bimu-trusted-v2:{}:{port}:{instance}",d.id);let _=udp.send_to(message.as_bytes(),(Ipv4Addr::BROADCAST,45949));
     if d.trust.host&&!d.revoked&&!d.paused&&book.automatic{let legacy=format!("bimu-trusted-v1:{}:{port}",d.id);let _=udp.send_to(legacy.as_bytes(),(Ipv4Addr::BROADCAST,45949));}
    }}}announced=Instant::now();
   }
   if let Some(udp)=&receiver{let mut buf=[0u8;257];for _ in 0..128{
    let Ok((n,SocketAddr::V4(source)))=udp.recv_from(&mut buf)else{break};if n>256||!private_ip(*source.ip()){continue}let Ok(value)=std::str::from_utf8(&buf[..n])else{continue};let parts:Vec<_>=value.split(':').collect();
    let (capable,tag,port)=match parts.as_slice(){["bimu-trusted-v2",tag,port,nonce]if *nonce!=instance=>(true,*tag,*port),["bimu-trusted-v1",tag,port]=>(false,*tag,*port),_=>continue};
    let Some(record)=records.iter().find(|d|d.id==tag)else{continue};let Ok(port)=port.parse::<u16>()else{continue};if port==0{continue}
    if !capable&&listeners.get(tag).and_then(|l|l.local_addr().ok()).map(|a|a.port())==Some(port)&&local_ip().ok()==Some(*source.ip()){continue}
    if record.revoked&&!capable{continue}
    if capable||!hints.get(tag).map(|(_,v,_)|*v).unwrap_or(false){hints.insert(tag.into(),(SocketAddrV4::new(*source.ip(),port),capable,Instant::now()));}if let Ok(mut inner)=shared.0.lock(){inner.seen.insert(tag.into(),Instant::now());}
   }}
   if !idle{thread::sleep(Duration::from_millis(200));continue}
   let requested=shared.0.lock().ok().and_then(|i|i.requested_id.clone());
   let mut choices=records.clone();if let Some(id)=&requested{choices.sort_by_key(|d|d.id!=*id)}
   let mut connected=false;
   for d in &choices{
    if requested.as_ref().map(|id|id!=&d.id).unwrap_or(false)&&!d.revoked{continue}
    if attempts.get(&d.id).map(|t|t.elapsed()<Duration::from_secs(2)).unwrap_or(false){continue}
    // A user can connect in either direction; automatic clients retain the
    // original role to avoid simultaneous outgoing handshakes on both PCs.
    let outgoing=hints.get(&d.id).filter(|(_,capable,t)|t.elapsed()<Duration::from_secs(5)&&(!d.revoked||*capable)&&(!d.trust.host||requested.as_ref()==Some(&d.id)||d.revoked||d.pause_pending));
    let incoming=listeners.get(&d.id).and_then(|l|match l.accept(){Ok((socket,SocketAddr::V4(addr)))if private_ip(*addr.ip())=>Some(socket),_=>None});
    let candidate=if let Some(socket)=incoming{Some((socket,Role::Host,true))}else if ((book.automatic&&!d.revoked&&!d.paused)||requested.as_ref()==Some(&d.id)||(d.revoked&&!d.notified)||d.pause_pending)&&outgoing.is_some(){let (addr,capable,_)=outgoing.unwrap();TcpStream::connect_timeout(&SocketAddr::V4(*addr),Duration::from_secs(2)).ok().map(|s|(s,Role::Client,*capable))}else{None};
    let Some((socket,role,capable))=candidate else{continue};attempts.insert(d.id.clone(),Instant::now());
    let generation={let Ok(mut inner)=shared.0.lock()else{break};if !matches!(inner.status.phase.as_str(),"idle"|"error"){break}reset(&mut inner);inner.status.phase="connecting".into();inner.status.message=Some(format!("Conectando con {}…",d.name));inner.generation};
    match pairing_transport::reconnect(socket,role,&d.trust.key){
     Ok(channel)=>{session_with_trust(app.clone(),shared.clone(),generation,channel,role,d.trust.clone(),capable,requested.as_ref()==Some(&d.id));connected=true;},
     Err(_)=>{fail(&shared,generation,"No se pudo autenticar el equipo guardado. Se volverá a intentar.");}
    }break
   }
   if connected{hints.clear();}thread::sleep(Duration::from_millis(200));
  }
 });
}

// Persist just a small integer. No file buffers exist merely by setting a limit.
fn limit_path(app:&AppHandle)->Result<std::path::PathBuf,String>{Ok(app.path().app_config_dir().map_err(|_|"No se pudo ubicar la configuración")?.join("transfer-limit.json"))}
pub fn load_transfer_limit(app:&AppHandle){
 if let Ok(path)=limit_path(app){if let Ok(meta)=std::fs::metadata(&path){if meta.len()<=128{if let Ok(bytes)=std::fs::read(path){if let Ok(Some(mb))=serde_json::from_slice::<Option<usize>>(&bytes){let _=crate::transfer_limits::set_mb(mb);}}}}}
}
#[tauri::command]
pub fn device_pairing_set_limit(mb:Option<usize>,app:AppHandle,state:State<'_,DevicePairing>)->Result<(),String>{
 if let Some(mb)=mb{crate::transfer_limits::validate(mb)?;}
 let mut inner=state.0.lock().map_err(|_|"Conexión ocupada")?;
 crate::config_migration::write_atomic(&limit_path(&app)?,serde_json::to_string(&mb).map_err(|_|"Límite inválido")?.as_bytes())?;
 if let Some(mb)=mb{crate::transfer_limits::set_mb(mb)?;}else{crate::transfer_limits::clear();}
 inner.outgoing=None;inner.outgoing_revision=inner.outgoing_revision.wrapping_add(1);inner.preparing=false;
 inner.shelf_messages.retain(|m|!matches!(m,ClipboardPayload::ShelfReply{..}));
 inner.limit_dirty=inner.status.phase=="connected";
 Ok(())
}
