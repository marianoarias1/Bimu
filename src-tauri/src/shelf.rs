use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, sync::Mutex};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;
use crate::shelf_shared::ShelfMetadata;

#[derive(Default)]pub struct Shelf(pub Mutex<()>);
#[derive(Clone, Deserialize, Serialize)]struct StoredItem { id: String, path: PathBuf }
#[derive(Serialize)]pub struct ShelfItem { pub id:String, pub name:String, pub path:String, pub kind:String, pub available:bool, pub remote:bool, pub online:bool, pub origin:Option<String>, pub busy:bool }
fn location(app:&AppHandle)->Result<PathBuf,String>{let dir=app.path().app_data_dir().map_err(|e|e.to_string())?;fs::create_dir_all(&dir).map_err(|e|e.to_string())?;Ok(dir.join("shelf.json"))}
fn read(app:&AppHandle)->Result<Vec<StoredItem>,String>{match fs::read(location(app)?){Ok(data)=>serde_json::from_slice(&data).map_err(|_|"No se pudo leer la bandeja guardada.".into()),Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(vec![]),Err(e)=>Err(e.to_string())}}
fn write(app:&AppHandle,entries:&[StoredItem])->Result<(),String>{fs::write(location(app)?,serde_json::to_vec(entries).map_err(|e|e.to_string())?).map_err(|e|e.to_string())}
fn view(items:Vec<StoredItem>)->Vec<ShelfItem>{items.into_iter().map(|item|{let meta=fs::metadata(&item.path);ShelfItem{id:item.id,name:item.path.file_name().unwrap_or(item.path.as_os_str()).to_string_lossy().into_owned(),path:item.path.to_string_lossy().into_owned(),kind:if meta.as_ref().map(|m|m.is_dir()).unwrap_or(false){"folder"}else{"file"}.into(),available:meta.is_ok(),remote:false,online:true,origin:None,busy:false}}).collect()}
fn combined(app:&AppHandle)->Result<Vec<ShelfItem>,String>{
 let local={let state=app.state::<Shelf>();let _guard=state.0.lock().map_err(|_|"Bandeja ocupada")?;view(read(app)?)};
 let mut all=local;all.extend(crate::device_pairing::remote_shelf(app));Ok(all)
}
/// Only metadata leaves the device. Absolute paths remain private.
pub(crate) fn shared_snapshot(app:&AppHandle)->Result<Vec<ShelfMetadata>,String>{
 let state=app.state::<Shelf>();let _guard=state.0.lock().map_err(|_|"Bandeja ocupada")?;
 let mut items=Vec::new();for item in view(read(app)?){let metadata=ShelfMetadata{id:item.id,name:item.name,kind:item.kind,available:item.available};if crate::shelf_shared::validate(std::slice::from_ref(&metadata)).is_ok(){items.push(metadata)}}Ok(items)
}
/// Resolve only a currently saved shelf id; peers never supply filesystem paths.
pub(crate) fn path_for_share(app:&AppHandle,id:&str)->Result<PathBuf,String>{
 if !crate::shelf_shared::valid_id(id){return Err("Identificador de archivo inválido".into())}
 let state=app.state::<Shelf>();let _guard=state.0.lock().map_err(|_|"Bandeja ocupada")?;
 let item=read(app)?.into_iter().find(|item|item.id==id).ok_or("El archivo ya no está en la bandeja")?;
 if !item.path.exists(){return Err("El archivo ya no está disponible en la otra PC".into())}Ok(item.path)
}
pub fn add_paths(app:&AppHandle,paths:Vec<PathBuf>)->Result<Vec<ShelfItem>,String>{
 {let state=app.state::<Shelf>();let _guard=state.0.lock().map_err(|_|"Bandeja ocupada.")?;let mut items=read(app)?;
 for path in paths{if !path.is_absolute()||!path.exists(){continue}let path=path.canonicalize().map_err(|e|e.to_string())?;if items.iter().any(|item|item.path==path){continue}if items.len()>=100{return Err("La bandeja admite hasta 100 elementos.".into())}let id=format!("{:x}",Sha256::digest(format!("{}:{}",path.to_string_lossy(),rand::random::<u64>()).as_bytes()));items.push(StoredItem{id,path});}write(app,&items)?;}
 crate::device_pairing::shelf_changed(app);let _=app.emit("shelf-changed",());combined(app)
}
#[tauri::command]pub fn shelf_list(app:AppHandle,_state:State<'_,Shelf>)->Result<Vec<ShelfItem>,String>{combined(&app)}
#[tauri::command]pub fn shelf_remove(id:String,app:AppHandle,_state:State<'_,Shelf>)->Result<Vec<ShelfItem>,String>{
 if let Some(remote)=id.strip_prefix("peer:"){crate::device_pairing::hide_remote_shelf(&app,remote)?;}
 else{let state=app.state::<Shelf>();let _guard=state.0.lock().map_err(|_|"Bandeja ocupada")?;let mut items=read(&app)?;items.retain(|item|item.id!=id);write(&app,&items)?;}
 crate::device_pairing::shelf_changed(&app);let _=app.emit("shelf-changed",());combined(&app)
}
#[tauri::command]pub fn shelf_open(id:String,app:AppHandle,_state:State<'_,Shelf>)->Result<(),String>{
 if let Some(remote)=id.strip_prefix("peer:"){return crate::device_pairing::request_remote_shelf(&app,remote)}
 let path={let state=app.state::<Shelf>();let _guard=state.0.lock().map_err(|_|"Bandeja ocupada")?;let item=read(&app)?.into_iter().find(|item|item.id==id).ok_or("El elemento ya no está en la bandeja.")?;item.path};
 if !path.exists(){return Err("No encontré el archivo o carpeta en su ubicación original.".into())}app.opener().open_path(path.to_string_lossy().into_owned(),None::<&str>).map_err(|e|format!("No se pudo abrir: {e}"))
}
