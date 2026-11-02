//! Explicit, session-only pairing. No clipboard or file access is granted here.
use crate::pairing_core::Role;
use crate::pairing_transport::{self, SecureChannel};
use crate::clipboard_protocol::{self, ClipboardPayload};
use rand::{rngs::OsRng, Rng, RngCore};
use crate::shelf_shared::ShelfMetadata;
use std::collections::{HashSet,VecDeque};
use tauri_plugin_opener::OpenerExt;
use serde::Serialize;
use std::{net::{Ipv4Addr, Shutdown, SocketAddr, SocketAddrV4, TcpListener, TcpStream, UdpSocket}, sync::{Arc, Mutex}, thread, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Clone, Serialize)]
#[serde(rename_all="camelCase")]
pub struct PairStatus { phase:String, code:Option<String>, address:Option<String>, peer:Option<String>, message:Option<String>, linked:bool, transfer_limit_mb:usize, effective_limit_mb:Option<usize>, limit_configured:bool, is_main:bool, uses_main_limit:bool, local_name:String, code_expires_at:Option<u64> }
impl Default for PairStatus { fn default()->Self { Self{phase:"idle".into(),code:None,address:None,peer:None,message:None,linked:false,transfer_limit_mb:crate::transfer_limits::current_mb(),effective_limit_mb:None,limit_configured:crate::transfer_limits::configured(),is_main:false,uses_main_limit:false,local_name:device_name(),code_expires_at:None} } }
#[derive(Default)]
struct Inner { generation:u64, status:PairStatus, socket:Option<TcpStream>, outgoing:Option<ClipboardPayload>, outgoing_revision:u64, preparing:bool, remote_shelf:Vec<ShelfMetadata>, remote_peer:Option<String>, hidden_shelf:HashSet<String>, shelf_messages:VecDeque<ClipboardPayload>, shelf_pending:Option<ShelfPending>, shelf_serving:bool, peer_limit_mb:Option<usize>, peer_limit_configured:bool, main:bool, limit_dirty:bool, shelf_dirty:bool }
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
  id:format!("peer:{}",item.id),name:item.name.clone(),path:String::new(),kind:item.kind.clone(),available:connected&&item.available,remote:true,online:connected,origin:inner.remote_peer.clone(),busy:inner.shelf_pending.as_ref().map(|p|p.id==item.id).unwrap_or(false)
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
 let name=item.name.clone();let token=OsRng.next_u64();
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
  let destination=root.join(format!("{:016x}",OsRng.next_u64()));
  let paths=crate::clipboard_files::unpack_with_limit(&bundle,&destination,receive_bytes(shared))?;
  if shared.0.lock().map(|i|i.generation!=generation||i.status.phase!="connected").unwrap_or(true){let _=std::fs::remove_dir_all(&destination);return Err("La otra PC se desconectó".into())}
  let path=paths.first().ok_or("No se recibió el archivo solicitado")?;
  app.opener().open_path(path.to_string_lossy().into_owned(),None::<&str>).map_err(|_|format!("Se descargó en {}, pero no se pudo abrir",path.display()))?;
  Ok(())
 })();
 let _=app.emit("shelf-changed",());if let Err(error)=result{let _=app.emit("shelf-error",error);}
}
fn handle_incoming(app:&AppHandle,shared:&DevicePairing,generation:u64,payload:ClipboardPayload,peer:&str)->Result<(),String>{
 match payload{
  ClipboardPayload::Limits{mb,configured}=>{
   crate::transfer_limits::validate(mb)?;
   if let Ok(mut inner)=shared.0.lock(){if inner.generation==generation{inner.peer_limit_mb=Some(mb);inner.peer_limit_configured=configured;inner.outgoing=None;inner.outgoing_revision=inner.outgoing_revision.wrapping_add(1);inner.preparing=false;inner.shelf_messages.retain(|m|!matches!(m,ClipboardPayload::ShelfReply{..}));}}
   Ok(())
  },
  ClipboardPayload::ShelfCatalog(items)=>{
   crate::shelf_shared::validate(&items)?;
   let mut changed=false;if let Ok(mut inner)=shared.0.lock(){if inner.generation==generation&&inner.remote_shelf!=items{inner.hidden_shelf.retain(|id|items.iter().any(|item|&item.id==id));inner.remote_shelf=items;changed=true}}
   if changed{let _=app.emit("shelf-changed",());}Ok(())
  },
  ClipboardPayload::ShelfRequest{id,token}=>{handle_shelf_request(app,shared,generation,id,token);Ok(())},
  ClipboardPayload::ShelfReply{id,token,bundle,error}=>{handle_shelf_reply(app,shared,generation,id,token,bundle,error);Ok(())},
  other=>{if !has_outgoing(shared,generation){crate::clipboard_history::apply_remote(app,other,peer,receive_bytes(shared))?;}Ok(())}
 }
}
fn next_shelf_or_clipboard(app:&AppHandle,shared:&DevicePairing,generation:u64,last:&mut Option<Vec<ShelfMetadata>>,last_check:&mut Instant)->Result<Option<ClipboardPayload>,String>{
 if let Ok(mut inner)=shared.0.lock(){if inner.generation!=generation{return Ok(None)}if inner.limit_dirty{inner.limit_dirty=false;return Ok(Some(ClipboardPayload::Limits{mb:crate::transfer_limits::current_mb(),configured:crate::transfer_limits::configured()}))}if let Some(message)=inner.shelf_messages.pop_front(){return Ok(Some(message))}}
 // Catalog publication is independent of the file-size limit and takes a
 // bounded turn before clipboard payloads. Re-advertise every three seconds so
 // a stale cache can recover without disconnecting or touching local limits.
 let dirty={let mut inner=shared.0.lock().map_err(|_|"Conexión ocupada")?;let dirty=inner.shelf_dirty;inner.shelf_dirty=false;dirty};
 if crate::shelf_shared::catalog_due(last.is_none(),dirty,last_check.elapsed()){
  *last_check=Instant::now();let snapshot=crate::shelf::shared_snapshot(app)?;
  *last=Some(snapshot.clone());return Ok(Some(ClipboardPayload::ShelfCatalog(snapshot)))
 }
 Ok(take_outgoing(shared,generation))
}
fn rollback_trust(app:&AppHandle,shared:&DevicePairing,trust:&crate::trusted_pairing::Trust){
 if let Ok(_inner)=shared.0.lock(){if crate::trusted_pairing::load(app).ok().flatten().map(|t|t.key)==Some(trust.key){let _=crate::trusted_pairing::forget(app);}}
}
fn device_name()->String {
 let value=std::env::var("COMPUTERNAME").unwrap_or_else(|_|"Otra PC".into());
 let clean:String=value.chars().filter(|c|!c.is_control()).take(64).collect();
 if clean.trim().is_empty(){"Otra PC".into()}else{clean}
}
fn session(app:AppHandle, shared:DevicePairing, generation:u64, mut channel:SecureChannel, role:Role){
 let socket=match channel.control_stream(){Ok(s)=>s,Err(_)=>{fail(&shared,generation,"No se pudo mantener la conexión.");return}};
 {let Ok(mut inner)=shared.0.lock() else{return};if inner.generation!=generation{return}inner.socket=Some(socket);}
 let own=crate::transfer_limits::hello(&device_name(),crate::transfer_limits::current_mb(),crate::transfer_limits::configured()).unwrap();
 let exchange=(||->Result<(String,usize,bool),String>{
  let received=match role {
   Role::Client=>{channel.send(own.as_bytes())?;channel.receive()?},
   Role::Host=>{let value=channel.receive()?;channel.send(own.as_bytes())?;value},
  };
  if received.len()>256{return Err("Nombre de equipo inválido".into())}
  let versioned=String::from_utf8(received).map_err(|_|"Nombre de equipo inválido")?;
  crate::transfer_limits::parse_hello(&versioned)
 })();
 let (peer,peer_limit,peer_configured)=match exchange{Ok(peer)=>peer,Err(_)=>{fail(&shared,generation,"No se pudo verificar el equipo. Actualizá Bimu en ambas PCs para usar los límites configurables.");return}};
 {let Ok(mut inner)=shared.0.lock()else{return};if inner.generation!=generation{return}
  if inner.remote_peer.as_deref()!=Some(peer.as_str()){inner.remote_shelf.clear();inner.hidden_shelf.clear();}
  inner.remote_peer=Some(peer.clone());inner.peer_limit_mb=Some(peer_limit);inner.peer_limit_configured=peer_configured;inner.main=matches!(role,Role::Host);}
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
 let trust=crate::trusted_pairing::load(&app)?;status.linked=trust.is_some();if status.phase!="connected"{status.is_main=trust.map(|t|t.host).unwrap_or(false)}Ok(status)
}
#[tauri::command]
pub fn device_pairing_disconnect(app:AppHandle,state:State<'_,DevicePairing>)->Result<(),String>{let mut inner=state.0.lock().map_err(|_|"Conexión ocupada")?;reset(&mut inner);drop(inner);let _=app.emit("shelf-changed",());Ok(())}
#[tauri::command]
pub fn device_pairing_forget(app:AppHandle,state:State<'_,DevicePairing>)->Result<(),String>{let mut inner=state.0.lock().map_err(|_|"Conexión ocupada")?;crate::trusted_pairing::forget(&app)?;reset(&mut inner);drop(inner);let _=app.emit("shelf-changed",());Ok(())}
#[tauri::command]
pub fn device_pairing_host(app:AppHandle,state:State<'_,DevicePairing>)->Result<PairStatus,String>{
 let ip=local_ip()?;
 let listener=TcpListener::bind("0.0.0.0:0").map_err(|_|"No se pudo abrir la conexión local")?;
 listener.set_nonblocking(true).map_err(|_|"No se pudo iniciar la conexión")?;
 let port=listener.local_addr().map_err(|_|"No se pudo obtener el puerto")?.port();
 let code=format!("{:06}",OsRng.gen_range(0..1_000_000u32));
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
      Ok(mut channel) if Instant::now()<expiry=>match crate::trusted_pairing::establish(&mut channel,Role::Host,|trust|{let inner=shared.0.lock().map_err(|_|"Conexión ocupada")?;if inner.generation!=generation{return Err("Vinculación cancelada".into())}crate::trusted_pairing::save(&app,trust)},|trust|rollback_trust(&app,&shared,trust)){Ok(())=>session(app,shared,generation,channel,Role::Host),Err(e)=>fail(&shared,generation,&e)},
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
pub fn device_pairing_join(address:String,code:String,app:AppHandle,state:State<'_,DevicePairing>)->Result<(),String>{
 let addr:SocketAddrV4=address.trim().parse().map_err(|_|"Ingresá la dirección local y el puerto que muestra la otra PC")?;
 if !private_ip(*addr.ip())||addr.port()==0{return Err("Ingresá una dirección IPv4 de red local válida".into())}
 if code.len()!=6||!code.bytes().all(|b|b.is_ascii_digit()){return Err("El código debe tener seis números".into())}
 let shared=state.inner().clone();
 let generation={let mut inner=shared.0.lock().map_err(|_|"Conexión ocupada")?;reset(&mut inner);inner.status=PairStatus{phase:"connecting".into(),message:Some("Conectando…".into()),..Default::default()};inner.generation};
 let _=app.emit("shelf-changed",());
 thread::spawn(move||{
  match TcpStream::connect_timeout(&SocketAddr::V4(addr),Duration::from_secs(5)){
   Ok(stream)=>match pairing_transport::pair(stream,Role::Client,&code){
    Ok(mut channel)=>match crate::trusted_pairing::establish(&mut channel,Role::Client,|trust|{let inner=shared.0.lock().map_err(|_|"Conexión ocupada")?;if inner.generation!=generation{return Err("Vinculación cancelada".into())}crate::trusted_pairing::save(&app,trust)},|trust|rollback_trust(&app,&shared,trust)){Ok(())=>session(app,shared,generation,channel,Role::Client),Err(e)=>fail(&shared,generation,&e)},
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


// Reconnection advertisements are hints; Noise PSK authentication is mandatory.
pub fn start_reconnect(app:AppHandle){
 let shared=app.state::<DevicePairing>().inner().clone();
 thread::spawn(move||loop{
  let trust=match crate::trusted_pairing::load(&app){Ok(Some(t))=>t,_=>{thread::sleep(Duration::from_secs(2));continue}};
  let available=||shared.0.lock().map(|i|matches!(i.status.phase.as_str(),"idle"|"error")).unwrap_or(false);
  if !available(){thread::sleep(Duration::from_millis(500));continue}
  match trust.role(){
   Role::Host=>{
    let listener=match TcpListener::bind("0.0.0.0:0"){Ok(x)=>x,Err(_)=>{thread::sleep(Duration::from_secs(2));continue}};
    let _=listener.set_nonblocking(true);let port=match listener.local_addr(){Ok(a)=>a.port(),Err(_)=>continue};
    let udp=match UdpSocket::bind("0.0.0.0:0"){Ok(s)=>s,Err(_)=>continue};let _=udp.set_broadcast(true);
    let message=format!("bimu-trusted-v1:{}:{}",trust.tag(),port);let mut last=Instant::now()-Duration::from_secs(1);let mut checked=Instant::now();
    while available(){
     if checked.elapsed()>=Duration::from_secs(2){checked=Instant::now();if crate::trusted_pairing::load(&app).ok().flatten().map(|x|x.key)!=Some(trust.key){break}}
     if last.elapsed()>=Duration::from_millis(600){let _=udp.send_to(message.as_bytes(),(Ipv4Addr::BROADCAST,45949));last=Instant::now();}
     match listener.accept(){
      Ok((stream,SocketAddr::V4(addr))) if private_ip(*addr.ip())=>{
       if let Ok(channel)=pairing_transport::reconnect(stream,Role::Host,&trust.key){
        let generation={let Ok(mut inner)=shared.0.lock()else{break};if !matches!(inner.status.phase.as_str(),"idle"|"error")||crate::trusted_pairing::load(&app).ok().flatten().map(|t|t.key)!=Some(trust.key){break}reset(&mut inner);inner.status.phase="connecting".into();inner.status.message=Some("Reconectando…".into());inner.generation};
        session(app.clone(),shared.clone(),generation,channel,Role::Host);break
       }
      },
      Ok(_)=>{},Err(e) if e.kind()==std::io::ErrorKind::WouldBlock=>thread::sleep(Duration::from_millis(100)),Err(_)=>break
     }
    }
   },
   Role::Client=>{
    let udp=match UdpSocket::bind("0.0.0.0:45949"){Ok(s)=>s,Err(_)=>{thread::sleep(Duration::from_secs(2));continue}};
    let _=udp.set_read_timeout(Some(Duration::from_millis(500)));let mut buf=[0u8;256];let prefix=format!("bimu-trusted-v1:{}:",trust.tag());let mut checked=Instant::now();
    while available(){
     if checked.elapsed()>=Duration::from_secs(2){checked=Instant::now();if crate::trusted_pairing::load(&app).ok().flatten().map(|x|x.key)!=Some(trust.key){break}}
     if let Ok((n,SocketAddr::V4(addr)))=udp.recv_from(&mut buf){
      if !private_ip(*addr.ip()){continue}let Ok(msg)=std::str::from_utf8(&buf[..n])else{continue};
      let Some(port)=msg.strip_prefix(&prefix).and_then(|p|p.parse::<u16>().ok()).filter(|p|*p!=0)else{continue};
      let target=SocketAddr::V4(SocketAddrV4::new(*addr.ip(),port));
      if let Ok(stream)=TcpStream::connect_timeout(&target,Duration::from_secs(2)){
       if let Ok(channel)=pairing_transport::reconnect(stream,Role::Client,&trust.key){
        let generation={let Ok(mut inner)=shared.0.lock()else{break};if !matches!(inner.status.phase.as_str(),"idle"|"error")||crate::trusted_pairing::load(&app).ok().flatten().map(|t|t.key)!=Some(trust.key){break}reset(&mut inner);inner.status.phase="connecting".into();inner.status.message=Some("Reconectando…".into());inner.generation};
        session(app.clone(),shared.clone(),generation,channel,Role::Client);break
       }
      }
     }
    }
   }
  }
  thread::sleep(Duration::from_millis(500));
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
