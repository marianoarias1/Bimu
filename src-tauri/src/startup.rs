use std::{fs,path::PathBuf};
use serde::{Serialize,Deserialize};
use tauri::{AppHandle,Manager,State};
use crate::startup_core::{startup_command,initial_startup};
#[derive(Default)]pub struct Startup(pub tokio::sync::Mutex<()>);
#[derive(Serialize)]#[serde(rename_all="camelCase")]pub struct Status{enabled:bool,supported:bool,automatic_launch:bool}
#[derive(Serialize,Deserialize)]struct Settings{enabled:bool}
fn config(app:&AppHandle)->Result<PathBuf,String>{Ok(app.path().app_config_dir().map_err(|_|"No se encontró la configuración.")?.join("startup-v1.json"))}
fn preference(app:&AppHandle)->Result<Option<bool>,String>{let path=config(app)?;if !path.exists(){return Ok(None)}let bytes=crate::bounded_io::read_limited(&path,1024).map_err(|_|"No se pudo leer el inicio automático.")?;if bytes.len()>1024{return Err("La configuración de inicio está dañada.".into())}Ok(Some(serde_json::from_slice::<Settings>(&bytes).map_err(|_|"La configuración de inicio está dañada.")?.enabled))}
fn save(app:&AppHandle,enabled:bool)->Result<(),String>{crate::config_migration::write_atomic(&config(app)?,&serde_json::to_vec(&Settings{enabled}).map_err(|e|e.to_string())?)}
#[cfg(windows)]fn register(enabled:bool)->Result<(),String>{use winreg::{RegKey,enums::HKEY_CURRENT_USER};let (key,_)=RegKey::predef(HKEY_CURRENT_USER).create_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run").map_err(|_|"Windows no permitió configurar el inicio de Bimu.")?;if enabled{let command=startup_command(&std::env::current_exe().map_err(|e|e.to_string())?)?;key.set_value("Bimu",&command).map_err(|_|"No se pudo activar el inicio con Windows.".into())}else{match key.delete_value("Bimu"){Ok(())=>Ok(()),Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(()),Err(_)=>Err("No se pudo desactivar el inicio con Windows.".into())}}}
#[cfg(not(windows))]fn register(_enabled:bool)->Result<(),String>{Err("El inicio automático requiere Windows.".into())}
fn status(enabled:bool)->Status{Status{enabled,supported:cfg!(windows)&&!cfg!(debug_assertions),automatic_launch:std::env::args().any(|arg|arg=="--autostart")}}

#[cfg(windows)]
async fn packaged_task() -> Result<windows::ApplicationModel::StartupTask,String> {
    use windows::{ApplicationModel::StartupTask,core::HSTRING};
    StartupTask::GetAsync(&HSTRING::from("BimuStartup")).map_err(|e|format!("No se pudo consultar el inicio de Windows: {e}"))?
        .await.map_err(|e|format!("No se pudo consultar el inicio de Windows: {e}"))
}
#[cfg(windows)]
async fn packaged_status() -> Result<bool,String> {
    use windows::ApplicationModel::StartupTaskState;
    let task=packaged_task().await?;
    let state=task.State().map_err(|e|e.to_string())?;
    Ok(state==StartupTaskState::Enabled || state==StartupTaskState::EnabledByPolicy)
}
#[cfg(not(windows))]
async fn packaged_status() -> Result<bool,String>{Err("El inicio requiere Windows.".into())}
#[cfg(windows)]
async fn packaged_set(app:&AppHandle,enabled:bool)->Result<bool,String>{
    use windows::ApplicationModel::StartupTaskState;
    let task=packaged_task().await?;
    if !enabled {task.Disable().map_err(|e|format!("No se pudo desactivar el inicio: {e}"))?;return packaged_status().await}
    if task.State().map_err(|e|e.to_string())?==StartupTaskState::DisabledByUser {
        return Err("Windows deshabilitó el inicio. Volvé a activarlo desde Configuración > Aplicaciones > Inicio.".into())
    }
    let (tx,rx)=tokio::sync::oneshot::channel();
    app.run_on_main_thread(move||{let _=tx.send(task.RequestEnableAsync().map_err(|e|e.to_string()));})
        .map_err(|e|format!("No se pudo solicitar el inicio: {e}"))?;
    let operation=rx.await.map_err(|_|"No se pudo solicitar el inicio.".to_string())??;
    let state=operation.await.map_err(|e|format!("No se pudo activar el inicio: {e}"))?;
    if state==StartupTaskState::DisabledByUser {return Err("Windows deshabilitó el inicio. Volvé a activarlo desde Configuración > Aplicaciones > Inicio.".into())}
    if state==StartupTaskState::DisabledByPolicy {return Err("La configuración de Windows impide activar el inicio.".into())}
    Ok(state==StartupTaskState::Enabled || state==StartupTaskState::EnabledByPolicy)
}
#[cfg(not(windows))]
async fn packaged_set(_app:&AppHandle,_enabled:bool)->Result<bool,String>{Err("El inicio requiere Windows.".into())}
#[tauri::command]pub async fn startup_sync(listen_preferred:bool,app:AppHandle,state:State<'_,Startup>)->Result<Status,String>{
 let _guard=state.inner().0.lock().await;
 if !cfg!(windows)||cfg!(debug_assertions){return Ok(status(false))}
 if crate::package_identity::is_packaged(){return Ok(status(packaged_status().await?))}
 let old=preference(&app)?;let enabled=initial_startup(old,listen_preferred);
 if let Some(value)=enabled{register(value)?;if old.is_none(){if let Err(e)=save(&app,value){let _=register(false);return Err(e)}}}
 Ok(status(enabled.unwrap_or(false)))
}
#[tauri::command]pub async fn startup_set(enabled:bool,app:AppHandle,state:State<'_,Startup>)->Result<Status,String>{
 let _guard=state.inner().0.lock().await;
 if !cfg!(windows)||cfg!(debug_assertions){return Err("El inicio con Windows está disponible en la app instalada.".into())}
 if crate::package_identity::is_packaged(){let actual=packaged_set(&app,enabled).await?;if actual!=enabled{return Err("Windows no permitió cambiar el inicio de Bimu.".into())}return Ok(status(actual))}
 let old=preference(&app)?.unwrap_or(false);register(enabled)?;
 if let Err(e)=save(&app,enabled){let _=register(old);return Err(e)}
 Ok(status(enabled))
}
