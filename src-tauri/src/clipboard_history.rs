use std::{path::PathBuf,sync::{Arc,Mutex},time::{Duration,SystemTime,UNIX_EPOCH}};
use tauri::{AppHandle,Emitter,Manager,State};
use serde::Serialize;
use crate::clipboard_protocol::ClipboardPayload;

const MAX_ITEMS:usize=30;
const MAX_IMAGE_BYTES:usize=12*1024*1024;
const MAX_TOTAL_BYTES:usize=48*1024*1024;
#[derive(Clone)]enum Content{Text(String),Files(Vec<PathBuf>),Image{width:usize,height:usize,rgba:Vec<u8>,preview:String}}
impl Content{
 fn bytes(&self)->usize{match self{Self::Text(s)=>s.len(),Self::Files(v)=>v.iter().map(|p|p.to_string_lossy().len()).sum(),Self::Image{rgba,..}=>rgba.len()}}
 fn payload(&self)->Option<ClipboardPayload>{match self{Self::Text(text)=>Some(ClipboardPayload::Text(text.clone())),Self::Image{width,height,rgba,..}=>Some(ClipboardPayload::Image{width:*width,height:*height,rgba:rgba.clone()}),Self::Files(paths)=>Some(ClipboardPayload::Files(paths.clone()))}}
}
#[derive(Clone)]struct Entry{id:u64,at:u64,content:Content,origin:String,shareable:bool}
struct Data{items:Vec<Entry>,last_sequence:u32,next_id:u64,paused:bool}
impl Default for Data{fn default()->Self{Self{items:Vec::new(),last_sequence:0,next_id:0,paused:true}}}
#[derive(Default,Clone)]pub struct ClipboardHistory(Arc<Mutex<Data>>);
#[derive(Serialize)]#[serde(rename_all="camelCase")]pub struct Item{id:u64,kind:&'static str,summary:String,preview:Option<String>,origin:String,at:u64,available:bool}
fn view(item:&Entry)->Item{let (kind,summary,preview,available)=match &item.content{
 Content::Text(text)=>("text",text.chars().take(220).collect(),None,true),
 Content::Files(paths)=>("files",if paths.len()==1{paths[0].file_name().unwrap_or_default().to_string_lossy().to_string()}else{format!("{} archivos o carpetas",paths.len())},None,paths.iter().all(|p|p.exists())),
 Content::Image{width,height,preview,..}=>("image",format!("Imagen · {} × {}",width,height),Some(preview.clone()),true),
 };Item{id:item.id,kind,summary,preview,origin:item.origin.clone(),at:item.at,available}}
fn insert(data:&mut Data,content:Content,origin:String,shareable:bool){
 data.next_id=data.next_id.wrapping_add(1);let id=data.next_id;
 let at=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
 data.items.insert(0,Entry{id,at,content,origin,shareable});
 while data.items.len()>MAX_ITEMS||data.items.iter().map(|i|i.content.bytes()).sum::<usize>()>MAX_TOTAL_BYTES{data.items.pop();}
}
#[derive(Serialize)]#[serde(rename_all="camelCase")]pub struct ClipboardStatus{items:Vec<Item>,paused:bool,sync_message:Option<String>}
#[tauri::command]pub fn clipboard_history_list(app:AppHandle,state:State<'_,ClipboardHistory>)->Result<ClipboardStatus,String>{let data=state.0.lock().map_err(|_|"Portapapeles ocupado")?;let items=data.items.iter().map(view).collect();let paused=data.paused;drop(data);Ok(ClipboardStatus{items,paused,sync_message:crate::device_pairing::clipboard_notice(&app)})}
#[tauri::command]pub fn clipboard_history_remove(id:u64,state:State<'_,ClipboardHistory>)->Result<(),String>{let mut data=state.0.lock().map_err(|_|"Portapapeles ocupado")?;data.items.retain(|item|item.id!=id);Ok(())}
#[tauri::command]pub fn clipboard_history_clear(state:State<'_,ClipboardHistory>)->Result<(),String>{state.0.lock().map_err(|_|"Portapapeles ocupado")?.items.clear();Ok(())}
#[tauri::command]pub fn clipboard_history_pause(paused:bool,state:State<'_,ClipboardHistory>)->Result<(),String>{state.0.lock().map_err(|_|"Portapapeles ocupado")?.paused=paused;Ok(())}
#[cfg(windows)]fn sequence()->u32{#[link(name="user32")]extern "system"{fn GetClipboardSequenceNumber()->u32;}unsafe{GetClipboardSequenceNumber()}}
#[cfg(windows)]fn permits(format_name:&str)->bool{
 use std::ffi::c_void;
 #[link(name="user32")]extern "system"{fn RegisterClipboardFormatW(name:*const u16)->u32;fn IsClipboardFormatAvailable(format:u32)->i32;fn OpenClipboard(window:*mut c_void)->i32;fn CloseClipboard()->i32;fn GetClipboardData(format:u32)->*mut c_void;}
 #[link(name="kernel32")]extern "system"{fn GlobalLock(memory:*mut c_void)->*mut c_void;fn GlobalUnlock(memory:*mut c_void)->i32;fn GlobalSize(memory:*mut c_void)->usize;}
 unsafe{let name=|s:&str|s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<_>>();let exclude=RegisterClipboardFormatW(name("ExcludeClipboardContentFromMonitorProcessing").as_ptr());let flag=RegisterClipboardFormatW(name(format_name).as_ptr());if exclude==0||flag==0{return false}if IsClipboardFormatAvailable(exclude)!=0{return false}if IsClipboardFormatAvailable(flag)==0{return true}if OpenClipboard(std::ptr::null_mut())==0{return false}let mem=GetClipboardData(flag);let allowed=if mem.is_null()||GlobalSize(mem)<4{false}else{let p=GlobalLock(mem);if p.is_null(){false}else{let value=std::ptr::read_unaligned(p as *const u32);GlobalUnlock(mem);value!=0}};CloseClipboard();allowed}
}
#[cfg(windows)]fn preview_image(width:usize,height:usize,rgba:&[u8])->Option<String>{
 use base64::{engine::general_purpose::STANDARD,Engine};use std::io::Cursor;
 let image=image::RgbaImage::from_raw(width as u32,height as u32,rgba.to_vec())?;
 let thumb=image::DynamicImage::ImageRgba8(image).thumbnail(120,80);let mut out=Cursor::new(Vec::new());
 thumb.write_to(&mut out,image::ImageFormat::Png).ok()?;Some(format!("data:image/png;base64,{}",STANDARD.encode(out.into_inner())))
}
#[cfg(windows)]fn capture()->Option<Content>{
 let mut clip=arboard::Clipboard::new().ok()?;
 if let Ok(paths)=clip.get().file_list(){if !paths.is_empty()&&paths.len()<=100{return Some(Content::Files(paths))}}
 if let Ok(text)=clip.get_text(){if !text.is_empty()&&text.len()<=1024*1024{return Some(Content::Text(text))}}
 if let Ok(img)=clip.get_image(){let (width,height)=(img.width,img.height);let rgba=img.bytes.into_owned();if rgba.len()>MAX_IMAGE_BYTES||width==0||height==0||width>16384||height>16384||width.checked_mul(height)?.checked_mul(4)?!=rgba.len(){return None}let preview=preview_image(width,height,&rgba)?;return Some(Content::Image{width,height,rgba,preview})}None
}
#[tauri::command]pub fn clipboard_history_copy(id:u64,app:AppHandle,state:State<'_,ClipboardHistory>)->Result<(),String>{
 #[cfg(windows)]{
  let mut data=state.0.lock().map_err(|_|"Portapapeles ocupado")?;
  let entry=data.items.iter().find(|item|item.id==id).ok_or("El elemento ya no está en el historial")?.clone();
  match &entry.content{
   Content::Text(text)=>{let mut clip=arboard::Clipboard::new().map_err(|_|"No se pudo abrir el portapapeles")?;clip.set_text(text.clone()).map_err(|_|"No se pudo copiar el texto")?},
   Content::Files(paths)=>crate::clipboard_files_windows::replace(&app,paths)?,
   Content::Image{width,height,rgba,..}=>{let mut clip=arboard::Clipboard::new().map_err(|_|"No se pudo abrir el portapapeles")?;clip.set_image(arboard::ImageData{width:*width,height:*height,bytes:rgba.clone().into()}).map_err(|_|"No se pudo copiar la imagen")?},
  };
  data.last_sequence=sequence();drop(data);
  if entry.shareable&&crate::device_pairing::clipboard_connected(&app){crate::device_pairing::queue_clipboard(&app,entry.content.payload());}else{crate::device_pairing::queue_clipboard(&app,None);}
  return Ok(())
 }
 #[cfg(not(windows))]{let _=(id,app,state);Err("Disponible en Windows".into())}
}
/// Apply a validated message from the authenticated peer without re-emitting it.
#[cfg(windows)]pub(crate) fn apply_remote(app:&AppHandle,payload:ClipboardPayload,peer_name:&str,file_limit:usize)->Result<(),String>{
 let label:String=peer_name.chars().filter(|c|!c.is_control()).take(64).collect();
 let origin=if label.trim().is_empty(){"Otro equipo".to_string()}else{label};
 let history=app.state::<ClipboardHistory>();let mut data=history.0.lock().map_err(|_|"Portapapeles ocupado")?;
 let current=sequence();if current!=0&&data.last_sequence!=0&&current!=data.last_sequence{return Ok(())}
 let content=match payload{
  ClipboardPayload::Text(text)=>{if text.is_empty()||text.len()>1024*1024{return Err("Texto recibido fuera de los límites".into())}{let mut clip=arboard::Clipboard::new().map_err(|_|"No se pudo abrir el portapapeles")?;clip.set_text(text.clone()).map_err(|_|"No se pudo actualizar el portapapeles")?;}Content::Text(text)},
  ClipboardPayload::Image{width,height,rgba}=>{if width==0||height==0||width>16384||height>16384||rgba.len()>MAX_IMAGE_BYTES||width.checked_mul(height).and_then(|n|n.checked_mul(4))!=Some(rgba.len()){return Err("Imagen recibida inválida".into())}let preview=preview_image(width,height,&rgba).ok_or("No se pudo preparar la imagen")?;{let mut clip=arboard::Clipboard::new().map_err(|_|"No se pudo abrir el portapapeles")?;clip.set_image(arboard::ImageData{width,height,bytes:rgba.clone().into()}).map_err(|_|"No se pudo actualizar la imagen")?;}Content::Image{width,height,rgba,preview}},
  ClipboardPayload::FileBundle(bundle)=>{
   let root=app.path().app_data_dir().map_err(|_|"No se pudo ubicar la carpeta de Bimu")?.join("archivos-recibidos");
   std::fs::create_dir_all(&root).map_err(|_|"No se pudo crear la carpeta de archivos recibidos")?;
   let destination=root.join(format!("{:016x}",crate::random_core::token()?));
   let paths=crate::clipboard_files::unpack_with_limit(&bundle,&destination,file_limit)?;
   if crate::clipboard_files_windows::replace(app,&paths).is_err(){let _=std::fs::remove_dir_all(&destination);return Err("No se pudieron copiar los archivos recibidos".into())}
   Content::Files(paths)
  },
  ClipboardPayload::ConnectionControl{..}|ClipboardPayload::PreviewRequest{..}|ClipboardPayload::PreviewReply{..}|ClipboardPayload::Limits{..}|ClipboardPayload::Files(_)|ClipboardPayload::ShelfCatalog(_)|ClipboardPayload::ShelfRequest{..}|ClipboardPayload::ShelfReply{..}=>return Err("Formato de portapapeles recibido inválido".into()),
 };
 data.last_sequence=sequence();if !data.paused{insert(&mut data,content,origin,true)}
 drop(data);let _=app.emit("clipboard-history-changed",());Ok(())
}
#[cfg(not(windows))]pub(crate) fn apply_remote(_app:&AppHandle,_payload:ClipboardPayload,_peer_name:&str,_file_limit:usize)->Result<(),String>{Err("Disponible en Windows".into())}
pub fn start(app:AppHandle){
 #[cfg(windows)]std::thread::spawn(move||{let history=app.state::<ClipboardHistory>().inner().clone();loop{
  std::thread::sleep(Duration::from_millis(500));if app.get_webview_window("main").is_none(){break}
  let Ok(mut data)=history.0.lock()else{continue};let seq=sequence();if seq==0||seq==data.last_sequence{continue}
  let sync=crate::device_pairing::clipboard_connected(&app);
  if data.paused&&!sync{data.last_sequence=seq;continue}
  let history_allowed=!data.paused&&permits("CanIncludeInClipboardHistory");
  let share_allowed=permits("CanUploadToCloudClipboard");
  let sync_allowed=sync&&share_allowed;
  if !history_allowed&&!sync_allowed{data.last_sequence=seq;drop(data);if sync{crate::device_pairing::queue_clipboard(&app,None)}continue}
  let Some(content)=capture()else{data.last_sequence=seq;drop(data);if sync{crate::device_pairing::queue_clipboard(&app,None)}continue};
  if sequence()!=seq{continue}data.last_sequence=seq;
  let payload=if sync_allowed{content.payload()}else{None};
  if history_allowed{insert(&mut data,content,"Este equipo".into(),share_allowed)}
  drop(data);
  if sync{crate::device_pairing::queue_clipboard(&app,payload)}
  if history_allowed{let _=app.emit("clipboard-history-changed",());}
 }});
 #[cfg(not(windows))]let _=app;
}

#[tauri::command]pub async fn clipboard_history_thumbnail(id:u64,app:AppHandle)->Result<Option<String>,String>{
 let path={let state=app.state::<ClipboardHistory>();let data=state.0.lock().map_err(|_|"Portapapeles ocupado")?;data.items.iter().find(|item|item.id==id).and_then(|item|match &item.content{Content::Files(paths)=>paths.first().cloned(),_=>None})};
 let Some(path)=path else{return Ok(None)};
 Ok(crate::file_thumbnails::prepare(path).await)
}
