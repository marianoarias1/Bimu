use crate::release_core::{Release,Asset,candidate};
use serde::Serialize;
use sha2::{Digest,Sha256};
use std::{path::PathBuf,io::{Write,Read},time::Duration};
use tauri::{Manager,State,Emitter};
#[derive(Default)]pub struct Updates(tokio::sync::Mutex<UpdateState>);
#[derive(Default)]struct UpdateState{candidate:Option<(String,Asset)>,download:Option<PathBuf>}
#[derive(Serialize)]#[serde(rename_all="camelCase")]
pub struct UpdateInfo{current:String,available:Option<String>,downloaded:bool}
fn client()->Result<reqwest::Client,String>{reqwest::Client::builder().user_agent("Bimu-updates").https_only(true).connect_timeout(Duration::from_secs(15)).timeout(Duration::from_secs(600)).build().map_err(|e|e.to_string())}
fn hash(path:&std::path::Path)->Result<String,String>{let mut file=std::fs::File::open(path).map_err(|e|e.to_string())?;let mut h=Sha256::new();let mut buf=[0u8;65536];loop{let n=file.read(&mut buf).map_err(|e|e.to_string())?;if n==0{break}h.update(&buf[..n]);}Ok(format!("{:x}",h.finalize()))}
#[tauri::command]pub async fn updates_check(app:tauri::AppHandle,state:State<'_,Updates>)->Result<UpdateInfo,String>{
 if crate::package_identity::is_packaged(){return Err("Esta instalación se actualiza desde Microsoft Store.".into())}
 let mut s=state.inner().0.lock().await;let current=app.package_info().version.to_string();
 let response=client()?.get("https://api.github.com/repos/marianoarias1/Bimu/releases/latest").header("Accept","application/vnd.github+json").send().await.map_err(|_|"No se pudo conectar con GitHub. Revisa tu conexion.")?;
 if response.status().as_u16()==404{s.candidate=None;s.download=None;return Ok(UpdateInfo{current,available:None,downloaded:false})}
 if !response.status().is_success(){return Err(format!("GitHub no pudo comprobar actualizaciones ({}).",response.status()))}
 let release:Release=crate::http_response::json_limited(response,1_000_000).await.map_err(|_|"GitHub devolvió una release inválida o demasiado grande.")?;
 let next=candidate(release,&current)?;
 let same=s.candidate.as_ref().zip(next.as_ref()).map(|(a,b)|a.0==b.0&&a.1.digest==b.1.digest).unwrap_or(false);
 if !same{s.download=None} s.candidate=next;
 Ok(UpdateInfo{current,available:s.candidate.as_ref().map(|v|v.0.clone()),downloaded:s.download.as_ref().map(|p|p.is_file()).unwrap_or(false)})
}
#[tauri::command]pub async fn updates_download(app:tauri::AppHandle,state:State<'_,Updates>)->Result<UpdateInfo,String>{
 if crate::package_identity::is_packaged(){return Err("Esta instalación se actualiza desde Microsoft Store.".into())}
 let mut s=state.inner().0.lock().await;let (version,asset)=s.candidate.clone().ok_or("Primero busca una actualizacion.")?;
 let dir=app.path().app_cache_dir().map_err(|e|e.to_string())?.join("updates");std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
 let path=dir.join(&asset.name);let partial=dir.join(format!("{}.partial",asset.name));
 let result=async{
  let mut response=client()?.get(&asset.browser_download_url).send().await.map_err(|_|"Se interrumpio la descarga.")?;
  if !response.status().is_success(){return Err("GitHub no pudo descargar el instalador.".into())}
  let mut file=std::fs::File::create(&partial).map_err(|e|e.to_string())?;let mut total=0u64;let mut digest=Sha256::new();let mut progress=0;
  while let Some(chunk)=response.chunk().await.map_err(|_|"Se interrumpio la descarga.")?{
   total+=chunk.len() as u64;if total>asset.size{return Err("El instalador supera el tamano anunciado.".into())}
   file.write_all(&chunk).map_err(|e|e.to_string())?;digest.update(&chunk);
   let percent=(total*100/asset.size)as u8;if percent!=progress{progress=percent;let _=app.emit("update-progress",percent);}
  }
  file.sync_all().map_err(|e|e.to_string())?;drop(file);
  if total!=asset.size || format!("sha256:{:x}",digest.finalize())!=asset.digest.as_deref().unwrap_or("").to_lowercase(){return Err("El SHA256 del instalador no coincide. No se ejecutara.".into())}
  if path.exists(){std::fs::remove_file(&path).map_err(|e|e.to_string())?;}std::fs::rename(&partial,&path).map_err(|e|e.to_string())?;Ok::<(),String>(())
 }.await;
 if result.is_err(){let _=std::fs::remove_file(&partial);}result?;s.download=Some(path);
 Ok(UpdateInfo{current:app.package_info().version.to_string(),available:Some(version),downloaded:true})
}
#[tauri::command]pub async fn updates_install(app:tauri::AppHandle,state:State<'_,Updates>)->Result<(),String>{
 if crate::package_identity::is_packaged(){return Err("Esta instalación se actualiza desde Microsoft Store.".into())}
 if cfg!(debug_assertions){return Err("Instala primero Bimu con el .exe de Releases. La instalacion desde la app se habilita en esa version.".into())}
 let s=state.inner().0.lock().await;let path=s.download.as_ref().ok_or("Primero descarga la actualizacion.")?;let (_,asset)=s.candidate.as_ref().ok_or("Primero busca una actualizacion.")?;
 if format!("sha256:{}",hash(path)?)!=asset.digest.as_deref().unwrap_or("").to_lowercase(){return Err("El instalador cambio despues de descargarlo. Descargalo nuevamente.".into())}
 std::process::Command::new(path).spawn().map_err(|e|format!("No se pudo abrir el instalador: {e}"))?;
 app.exit(0);Ok(())
}

#[tauri::command]pub fn updates_channel()->&'static str {if crate::package_identity::is_packaged(){"store"}else{"github"}}
