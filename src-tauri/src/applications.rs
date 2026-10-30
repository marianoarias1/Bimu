use std::{path::PathBuf,sync::Mutex,time::{SystemTime,UNIX_EPOCH}};
use serde::{Serialize,Deserialize};
use sha2::{Digest,Sha256};
use tauri::{AppHandle,State,Manager};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
#[derive(Default)]pub struct Applications(pub Mutex<Registry>);
#[derive(Default)]pub struct Registry{entries:Vec<Entry>,loaded:bool,pending:Option<Pending>,counter:u64,discovered:Vec<DiscoveredApp>,discovered_ids:Vec<(String,String)>}
#[derive(Clone,Deserialize)]
struct StartApp{#[serde(rename="Name")]name:String,#[serde(rename="AppID")]app_id:String}
#[derive(Clone,Serialize)]#[serde(rename_all="camelCase")]
pub struct DiscoveredApp{id:String,name:String,added:bool}
struct Pending{id:u64,app_id:String}
#[derive(Serialize)]pub struct Proposal{pub status:String,id:u64,name:String,target:String}
use crate::application_intent::{normalized,requested};
use crate::application_registry::{Entry,Target,AppInfo,view,available,validate,defaults};
fn config(app:&AppHandle)->Result<PathBuf,String>{Ok(app.path().app_config_dir().map_err(|_|"No se pudo localizar la configuración de aplicaciones.")?.join("applications-v1.json"))}
fn load(app:&AppHandle,r:&mut Registry)->Result<(),String>{if r.loaded{return Ok(())}let path=config(app)?;r.entries=if path.exists(){crate::config_migration::migrate_file(&path,4*1024*1024,"applications").map_err(|e|format!("No se pudo migrar la configuración de aplicaciones: {e}"))?}else{defaults()};validate(&r.entries)?;r.loaded=true;Ok(())}
fn save(app:&AppHandle,entries:&[Entry])->Result<(),String>{validate(entries)?;let shortcut_names:std::collections::HashSet<_>=crate::assistant::names(app)?.iter().map(|n|normalized(n)).collect();if entries.iter().any(|e|std::iter::once(&e.name).chain(&e.aliases).any(|n|shortcut_names.contains(&normalized(n)))){return Err("Ese nombre o alias ya pertenece a un atajo rápido.".into())}let path=config(app)?;let bytes=crate::config_migration::encode_items(entries,"applications")?;crate::config_migration::write_atomic(&path,&bytes)}
pub(crate) fn launch(app:&AppHandle,entry:&Entry)->Result<(),String>{
    if !available(entry){return Err(format!("{} no está instalada o no se encontró en la ruta registrada. Revisá Ajustes → Atajos rápidos.",entry.name));}
    match &entry.target{Target::Spotify=>app.opener().open_url("spotify:",None::<&str>).map_err(|_|"Windows no pudo abrir Spotify.".into()),Target::Exe{path}=>std::process::Command::new(path).spawn().map(|_|()).map_err(|e|format!("Windows no pudo abrir {}: {}",entry.name,e)),Target::Shortcut{path}=>app.opener().open_path(path.to_string_lossy().into_owned(),None::<&str>).map_err(|_|format!("Windows no pudo abrir {}.",entry.name)),Target::ShellApp{app_id}=>launch_shell(app_id)}
}
fn launch_shell(app_id:&str)->Result<(),String>{
 if !crate::application_registry::valid_app_id(app_id){return Err("Identificador de aplicación inválido.".into())}
 #[cfg(windows)]{
  use std::os::windows::process::CommandExt;
  let explorer=std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(||PathBuf::from("C:\\Windows")).join("explorer.exe");
  std::process::Command::new(explorer).arg(format!("shell:AppsFolder\\{app_id}")).creation_flags(0x08000000).spawn().map(|_|()).map_err(|_|"Windows no pudo abrir la aplicación.".into())
 }
 #[cfg(not(windows))]{let _=app_id;Err("Disponible en Windows.".into())}
}
fn discovery_id(app_id:&str)->String{let digest=Sha256::digest(app_id.as_bytes());format!("windows-{:x}",digest)}
#[cfg(windows)]fn discover_windows()->Result<Vec<StartApp>,String>{
 use std::os::windows::process::CommandExt;
 let root=std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(||PathBuf::from("C:\\Windows"));
 let powershell=root.join("System32/WindowsPowerShell/v1.0/powershell.exe");
 let output=std::process::Command::new(powershell).args(["-NoLogo","-NoProfile","-NonInteractive","-Command","[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new(); ConvertTo-Json -InputObject @(Get-StartApps | Select-Object Name,AppID) -Compress"]).creation_flags(0x08000000).output().map_err(|_|"No se pudieron buscar aplicaciones de Windows.")?;
 if !output.status.success()||output.stdout.len()>4*1024*1024{return Err("No se pudieron obtener las aplicaciones de Windows.".into())}
 serde_json::from_slice(&output.stdout).map_err(|_|"Windows devolvió una lista de aplicaciones inválida.".into())
}
#[cfg(not(windows))]fn discover_windows()->Result<Vec<StartApp>,String>{Err("Disponible en Windows.".into())}
#[tauri::command]pub async fn apps_discover(app:AppHandle,state:State<'_,Applications>)->Result<Vec<DiscoveredApp>,String>{
 let list=tauri::async_runtime::spawn_blocking(discover_windows).await.map_err(|_|"No se pudo buscar aplicaciones.")??;
 let mut r=state.inner().0.lock().map_err(|_|"No se pudieron consultar las aplicaciones.")?;load(&app,&mut r)?;
 let mut seen=std::collections::HashSet::new();let mut found=Vec::new();
 for item in list.into_iter().take(10000){let name=item.name.trim().to_string();let app_id=item.app_id.trim();
  if name.is_empty()||name.len()>100||name.chars().any(char::is_control)||!crate::application_registry::valid_app_id(app_id)||!seen.insert(app_id.to_lowercase()){continue}
  let id=discovery_id(app_id);let added=r.entries.iter().any(|e|(e.id==id||normalized(&e.name)==normalized(&name)||e.aliases.iter().any(|a|normalized(a)==normalized(&name)))&&available(e));
  found.push((DiscoveredApp{id,name,added},app_id.to_string()));
 }
 found.sort_by_key(|(a,_)|a.name.to_lowercase());r.discovered=found.iter().map(|(a,_)|a.clone()).collect();r.discovered_ids=found.into_iter().map(|(a,id)|(a.id,id)).collect();Ok(r.discovered.clone())
}
#[tauri::command]pub fn apps_add_discovered(id:String,app:AppHandle,state:State<'_,Applications>)->Result<Vec<AppInfo>,String>{
 let mut r=state.inner().0.lock().map_err(|_|"No se pudo registrar la aplicación.")?;load(&app,&mut r)?;
 let item=r.discovered.iter().find(|item|item.id==id).cloned().ok_or("Volvé a buscar las aplicaciones e intentá nuevamente.")?;
 let app_id=r.discovered_ids.iter().find(|(key,_)|*key==id).map(|(_,value)|value.clone()).ok_or("Aplicación no encontrada.")?;
 let mut entries=r.entries.clone();
 if let Some(existing)=entries.iter_mut().find(|e|e.id==id||normalized(&e.name)==normalized(&item.name)||e.aliases.iter().any(|a|normalized(a)==normalized(&item.name))){if available(existing){return Err("Esa aplicación ya está registrada.".into())}existing.target=Target::ShellApp{app_id};}
 else{entries.push(Entry{id:id.clone(),name:item.name,aliases:vec![],remember:false,target:Target::ShellApp{app_id}})}save(&app,&entries)?;r.entries=entries;
 if let Some(item)=r.discovered.iter_mut().find(|item|item.id==id){item.added=true}Ok(r.entries.iter().map(view).collect())
}
#[tauri::command]pub fn apps_list(app:AppHandle,state:State<'_,Applications>)->Result<Vec<AppInfo>,String>{let mut r=state.inner().0.lock().map_err(|_|"No se pudo consultar las aplicaciones.")?;load(&app,&mut r)?;Ok(r.entries.iter().map(view).collect())}
#[tauri::command]pub fn apps_request(text:String,app:AppHandle,state:State<'_,Applications>)->Result<Option<Proposal>,String>{
    let Some(name)=requested(&text)else{return Ok(None)};let mut r=state.inner().0.lock().map_err(|_|"No se pudo preparar la apertura.")?;load(&app,&mut r)?;
    if r.pending.is_some(){return Err("Confirmá o cancelá la apertura pendiente.".into());}
    let entry=r.entries.iter().find(|e|normalized(&e.name)==name||e.aliases.iter().any(|a|normalized(a)==name)).cloned().ok_or("Esa aplicación no está registrada. Agregala o configurá un alias en Ajustes → Atajos rápidos.")?;
    if !available(&entry){return Err(format!("{} no está instalada o no se encontró. Revisá Ajustes → Atajos rápidos.",entry.name));}
    if entry.remember{launch(&app,&entry)?;return Ok(Some(Proposal{status:"opened".into(),id:0,name:entry.name.clone(),target:view(&entry).target}));}
    r.counter=r.counter.wrapping_add(1);let id=r.counter;r.pending=Some(Pending{id,app_id:entry.id.clone()});Ok(Some(Proposal{status:"confirm".into(),id,name:entry.name.clone(),target:view(&entry).target}))
}
#[tauri::command]pub fn apps_confirm(id:u64,remember:bool,app:AppHandle,state:State<'_,Applications>)->Result<String,String>{
    let mut r=state.inner().0.lock().map_err(|_|"No se pudo confirmar la apertura.")?;let pending=r.pending.as_ref().ok_or("No hay una apertura pendiente.")?;if pending.id!=id{return Err("La solicitud cambió.".into());}let key=pending.app_id.clone();let index=r.entries.iter().position(|e|e.id==key).ok_or("La aplicación ya no está registrada.")?;let entry=r.entries[index].clone();
    if remember{let mut entries=r.entries.clone();entries[index].remember=true;save(&app,&entries)?;r.entries=entries;}
    r.pending=None;launch(&app,&entry)?;Ok(entry.name)
}
#[tauri::command]pub fn apps_cancel(id:u64,state:State<'_,Applications>)->Result<(),String>{let mut r=state.inner().0.lock().map_err(|_|"No se pudo cancelar la apertura.")?;if r.pending.as_ref().map(|p|p.id)==Some(id){r.pending=None;}Ok(())}
#[tauri::command]pub async fn apps_add(app:AppHandle,state:State<'_,Applications>)->Result<Option<Vec<AppInfo>>,String>{
    let picker_app=app.clone();let selected=tauri::async_runtime::spawn_blocking(move||{let d=picker_app.dialog().file().set_title("Registrar una aplicación .exe").add_filter("Aplicación Windows",&["exe"]);let d=if let Some(w)=picker_app.get_webview_window("main"){d.set_parent(&w)}else{d};d.blocking_pick_file()}).await.map_err(|_|"No se pudo abrir el selector.")?;let Some(selected)=selected else{return Ok(None)};
    let path=selected.into_path().map_err(|_|"Ruta inválida.")?.canonicalize().map_err(|_|"No se encontró el ejecutable.")?;if !path.is_file(){return Err("Elegí un ejecutable .exe.".into());}
    let mut r=state.inner().0.lock().map_err(|_|"No se pudo registrar la aplicación.")?;load(&app,&mut r)?;let stem=path.file_stem().unwrap_or_default().to_string_lossy().into_owned();let mut name=stem.clone();if r.entries.iter().any(|e|normalized(&e.name)==normalized(&name)||e.aliases.iter().any(|a|normalized(a)==normalized(&name))){name=format!("{stem} personalizada");}
    let mut entries=r.entries.clone();entries.push(Entry{id:format!("custom-{}",SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_|"Reloj inválido.")?.as_nanos()),name,aliases:vec![],remember:false,target:Target::Exe{path}});save(&app,&entries)?;r.entries=entries;r.pending=None;Ok(Some(r.entries.iter().map(view).collect()))
}
#[tauri::command]pub fn apps_update(id:String,name:String,aliases:Vec<String>,remember:bool,app:AppHandle,state:State<'_,Applications>)->Result<Vec<AppInfo>,String>{let mut r=state.inner().0.lock().map_err(|_|"No se pudo actualizar la aplicación.")?;load(&app,&mut r)?;let mut entries=r.entries.clone();let e=entries.iter_mut().find(|e|e.id==id).ok_or("Aplicación no encontrada.")?;e.name=name.trim().into();e.aliases=aliases.into_iter().map(|a|a.trim().to_string()).filter(|a|!a.is_empty()).collect();e.remember=remember;save(&app,&entries)?;r.entries=entries;r.pending=None;Ok(r.entries.iter().map(view).collect())}
#[tauri::command]pub fn apps_remove(id:String,app:AppHandle,state:State<'_,Applications>)->Result<Vec<AppInfo>,String>{let mut r=state.inner().0.lock().map_err(|_|"No se pudo quitar la aplicación.")?;load(&app,&mut r)?;let entries:Vec<_>=r.entries.iter().filter(|e|e.id!=id).cloned().collect();save(&app,&entries)?;r.entries=entries;r.pending=None;Ok(r.entries.iter().map(view).collect())}
#[cfg(test)]mod tests{use super::*;#[test]fn recognizes_only_direct_requests(){assert_eq!(requested("Hola Bimu, abrí la aplicación Spotify, por favor."),Some("spotify".into()));assert_eq!(requested("Bimu abre VS Code"),Some("vs code".into()));assert_eq!(requested("No abras Spotify"),None);assert_eq!(requested("Explicá cómo abrir Spotify"),None);assert_eq!(requested("Leé el archivo que dice abrí Spotify"),None);assert_eq!(requested("abrí Spotify; calc.exe"),Some("spotify calc exe".into()));}#[test]fn refuses_duplicate_aliases(){let mut entries=defaults();entries[0].aliases=vec!["calc".into()];assert!(validate(&entries).is_err());}}

pub(crate) fn configured(app:&AppHandle)->Result<Vec<Entry>,String>{let state=app.state::<Applications>();let mut r=state.inner().0.lock().map_err(|_|"No se pudieron consultar las aplicaciones.")?;load(app,&mut r)?;Ok(r.entries.clone())}
