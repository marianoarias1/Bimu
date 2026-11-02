use serde::{Serialize,Deserialize};
use std::{path::PathBuf,collections::{HashSet,VecDeque},sync::atomic::{AtomicBool,Ordering},time::{Instant,Duration}};
use crate::application_intent::{normalized,requested};
#[derive(Clone,Serialize,Deserialize)]
#[serde(tag="kind",rename_all="camelCase")]
pub enum Action{App{#[serde(rename="appId")]app_id:String,#[serde(default,rename="profileDirectory",skip_serializing_if="Option::is_none")]profile_directory:Option<String>},Url{url:String,#[serde(default,rename="browserId")]browser_id:Option<String>,#[serde(default,rename="profileDirectory",skip_serializing_if="Option::is_none")]profile_directory:Option<String>},Folder{path:PathBuf}}
#[derive(Clone,Serialize,Deserialize)]pub struct Shortcut{pub id:String,pub name:String,pub aliases:Vec<String>,pub remember:bool,pub actions:Vec<Action>}
/// Reorder only an exact permutation, preserving every configured shortcut.
pub fn reorder_shortcuts(items:Vec<Shortcut>, ids:&[String])->Result<Vec<Shortcut>,String>{
 if items.len()!=ids.len(){return Err("La lista cambió; actualizá los atajos e intentá nuevamente.".into())}
 let mut remaining=items;let mut result=Vec::with_capacity(ids.len());
 for id in ids{let index=remaining.iter().position(|item|&item.id==id).ok_or("El orden de atajos no es válido.")?;result.push(remaining.remove(index));}
 if !remaining.is_empty(){return Err("El orden de atajos no es válido.".into())}Ok(result)
}
pub fn shortcut_matches(shortcut:&Shortcut,text:&str)->bool{let n=normalized(text);let n=n.strip_prefix("hola bimu ").or_else(||n.strip_prefix("bimu ")).unwrap_or(&n);let n=n.strip_prefix("por favor ").unwrap_or(n);let n=n.strip_suffix(" por favor").unwrap_or(n);std::iter::once(&shortcut.name).chain(&shortcut.aliases).any(|s|normalized(s)==n||requested(text).as_ref()==Some(&normalized(s)))}
pub fn validate_shortcuts(items:&[Shortcut],_app_names:&[String])->Result<(),String>{
 if items.len()>30{return Err("Podés guardar hasta 30 atajos.".into())}
 let mut ids=HashSet::new();let mut names=HashSet::new();
 for s in items{
  if s.id.is_empty()||s.id.len()>100||!ids.insert(&s.id)||s.name.trim().is_empty()||s.name.chars().count()>80||s.aliases.len()>10||s.actions.is_empty()||s.actions.len()>12{return Err("Revisá el nombre del atajo y agregá entre 1 y 12 acciones.".into())}
  let mut own=HashSet::new();for name in std::iter::once(&s.name).chain(&s.aliases){let name=normalized(name);if name.is_empty()||name.len()>160{return Err("Elegí un nombre más corto para el atajo.".into())}if own.insert(name.clone())&&!names.insert(name){return Err("Ese nombre ya pertenece a otro atajo.".into())}}
  for action in &s.actions{match action{
   Action::App{app_id,profile_directory}=>if app_id.is_empty()||app_id.len()>100||profile_directory.as_ref().is_some_and(|s|!valid_profile_id(s)){return Err("Elegí una aplicación para cada acción.".into())},
   Action::Url{url,browser_id,profile_directory}=>{if profile_directory.as_ref().is_some_and(|s|!valid_profile_id(s)||browser_id.is_none())||!valid_web_url(url)||browser_id.as_ref().is_some_and(|s|s.is_empty()||s.len()>100){return Err("Usá una dirección http:// o https:// válida y un navegador de la lista.".into())}},
   Action::Folder{path}=>if !path.is_absolute()||!local_path(path){return Err("Elegí una carpeta local existente.".into())}
  }}
 }Ok(())
}
pub fn valid_web_url(url:&str)->bool{
 if url.len()>2048||url.chars().any(|c|c.is_control()||c.is_whitespace())||!(url.starts_with("https://")||url.starts_with("http://")){return false}
 // Parsing and host verification also happen immediately before opening.
 let host=url.split_once("://").map(|(_,rest)|rest.split(['/', '?', '#']).next().unwrap_or("")).unwrap_or("");!host.is_empty()&&!host.contains('@')&&!host.contains('\\')
}
pub fn local_path(path:&std::path::Path)->bool{let s=path.to_string_lossy();if let Some(rest)=s.strip_prefix("\\\\?\\"){let b=rest.as_bytes();return b.len()>2&&b[0].is_ascii_alphabetic()&&b[1]==b':'}!s.starts_with("\\\\")&&!s.starts_with("//")}
// Shell consumers and the UI use ordinary drive paths after canonical validation.
pub fn shell_folder_path(path:PathBuf)->PathBuf{let text=path.to_string_lossy();if let Some(rest)=text.strip_prefix("\\\\?\\"){if local_path(&path){return PathBuf::from(rest)}}path}
#[derive(Clone,Debug,PartialEq)]pub struct FolderRequest{pub name:String,pub open:bool}
pub fn folder_request(text:&str)->Option<FolderRequest>{
 if text.len()>512{return None}let n=normalized(text);let mut n=n.as_str();for prefix in ["hola bimu ","bimu "]{if let Some(rest)=n.strip_prefix(prefix){n=rest;break}}if let Some(rest)=n.strip_prefix("por favor "){n=rest}
 let rest=["busca ","buscame ","buscar ","encontra ","encontrame "].iter().find_map(|p|n.strip_prefix(p))?;
 let rest=["una carpeta llamada ","la carpeta llamada ","una carpeta de nombre ","la carpeta de nombre ","una carpeta ","la carpeta ","carpeta "].iter().find_map(|p|rest.strip_prefix(p))?;
 let mut name=rest.to_string();let mut open=false;
 for suffix in [" por favor"]{if let Some(rest)=name.strip_suffix(suffix){name=rest.into()}}
 for suffix in [" y abrila"," y abrela"," y abrimela"," y abri la carpeta"," y abrirla"]{if let Some(rest)=name.strip_suffix(suffix){name=rest.into();open=true;break}}
 for suffix in [" en mi pc"," en mi computadora"," en mi equipo"," en la pc"]{if let Some(rest)=name.strip_suffix(suffix){name=rest.into();break}}
 if name.is_empty()||name.len()>160{return None}Some(FolderRequest{name,open})
}
#[derive(Clone,Serialize)]pub struct FolderMatch{pub id:String,pub name:String,pub path:PathBuf,pub exact:bool}
#[derive(Serialize)]#[serde(rename_all="camelCase")]pub struct SearchResult{pub query:String,pub matches:Vec<FolderMatch>,pub incomplete:bool,pub cancelled:bool,pub visited:usize,pub skipped:usize}
pub fn skip_directory(name:&str)->bool{matches!(name.to_ascii_lowercase().as_str(),"windows"|"program files"|"program files (x86)"|"programdata"|"appdata"|"$recycle.bin"|"system volume information"|"node_modules"|".git"|".cache"|".venv"|"venv"|"target")}
fn link_or_reparse(meta:&std::fs::Metadata)->bool{
 if meta.file_type().is_symlink(){return true}
 #[cfg(windows)]{use std::os::windows::fs::MetadataExt;if meta.file_attributes()&0x400!=0{return true}}
 false
}
pub fn search_folders(roots:Vec<PathBuf>,query:&str,cancel:&AtomicBool,budget:Duration,max_entries:usize)->SearchResult{
 let query=normalized(query);let started=Instant::now();let mut result=SearchResult{query:query.clone(),matches:vec![],incomplete:false,cancelled:false,visited:0,skipped:0};let mut queue:VecDeque<_>=roots.into_iter().collect();let mut seen=HashSet::new();
 'scan:while let Some(dir)=queue.pop_front(){
  if cancel.load(Ordering::Relaxed){result.cancelled=true;break}
  if started.elapsed()>=budget||result.visited>=max_entries||result.matches.len()>=50{result.incomplete=true;break}
  let meta=match std::fs::symlink_metadata(&dir){Ok(m)=>m,Err(_)=>{result.skipped+=1;continue}};if !meta.is_dir()||link_or_reparse(&meta)||!local_path(&dir){continue}
  let key=dir.to_string_lossy().to_lowercase();if !seen.insert(key){continue}
  let name=dir.file_name().unwrap_or_default().to_string_lossy().into_owned();let n=normalized(&name);
  if !query.is_empty()&&n.contains(&query){result.matches.push(FolderMatch{id:String::new(),name,path:dir.clone(),exact:n==query})}
  let entries=match std::fs::read_dir(&dir){Ok(e)=>e,Err(_)=>{result.skipped+=1;continue}};
  for entry in entries{
   if cancel.load(Ordering::Relaxed){result.cancelled=true;break 'scan}
   if started.elapsed()>=budget||result.visited>=max_entries{result.incomplete=true;break 'scan}
   result.visited+=1;let entry=match entry{Ok(e)=>e,Err(_)=>{result.skipped+=1;continue}};
   if skip_directory(&entry.file_name().to_string_lossy()){continue}
   if entry.file_type().is_ok_and(|t|t.is_dir()&&!t.is_symlink()){queue.push_back(entry.path())}
  }
 }
 result.matches.sort_by(|a,b|b.exact.cmp(&a.exact).then(a.path.cmp(&b.path)));result.matches.dedup_by(|a,b|a.path==b.path);result
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn parses_explicit_folder_requests(){assert_eq!(folder_request("Bimu, buscá una carpeta llamada fotos de viajes en mi PC y abrila"),Some(FolderRequest{name:"fotos de viajes".into(),open:true}));assert!(folder_request("No busques una carpeta fotos").is_none());assert!(folder_request("Explicame cómo buscar una carpeta llamada fotos").is_none());assert!(folder_request("busca Spotify").is_none());assert_eq!(folder_request("Buscame la carpeta Vacaciones"),Some(FolderRequest{name:"vacaciones".into(),open:false}));}
 #[test]fn refuses_non_web_destinations(){for url in ["file:///C:/Windows","javascript:alert(1)","spotify:","https://", "https://user:pass@example.com","https://a.com\ncalc.exe"]{assert!(!valid_web_url(url),"{url}")}assert!(valid_web_url("https://mail.google.com/mail/u/0/"));}
 #[test]fn shell_paths_keep_unicode_and_remove_only_local_device_prefix(){assert_eq!(shell_folder_path(PathBuf::from("\\\\?\\C:\\Fotos de España")),PathBuf::from("C:\\Fotos de España"));assert_eq!(shell_folder_path(PathBuf::from("\\\\?\\UNC\\server\\share")),PathBuf::from("\\\\?\\UNC\\server\\share"));}
 #[test]fn routine_names_match_only_direct_phrases(){let s=Shortcut{id:"work".into(),name:"Modo trabajo".into(),aliases:vec!["a trabajar".into()],remember:false,actions:vec![Action::App{app_id:"chrome".into(),profile_directory:None}]};assert!(shortcut_matches(&s,"Bimu, modo trabajo"));assert!(shortcut_matches(&s,"Ejecutá modo trabajo"));assert!(!shortcut_matches(&s,"No actives modo trabajo"));assert!(validate_shortcuts(&[s.clone(),s],&[]).is_err());}
 #[test]fn routine_validation_rejects_collisions_and_empty_steps(){let mut s=Shortcut{id:"work".into(),name:"Modo trabajo".into(),aliases:vec![],remember:false,actions:vec![Action::Url{url:"https://mail.google.com".into(),browser_id:None,profile_directory:None}]};assert!(validate_shortcuts(&[s.clone()],&[]).is_ok());assert!(validate_shortcuts(&[s.clone()],&["Modo trabajo".into()]).is_ok());s.actions.clear();assert!(validate_shortcuts(&[s],&[]).is_err());assert!(!local_path(std::path::Path::new("\\\\server\\share")));assert!(local_path(std::path::Path::new("\\\\?\\C:\\Fotos")));}
 #[test]fn folder_search_reports_ambiguity_limits_and_cancellation(){let root=std::env::temp_dir().join(format!("bimu-search-{}",std::process::id()));let _=std::fs::remove_dir_all(&root);std::fs::create_dir_all(root.join("A/Fotos de viajes")).unwrap();std::fs::create_dir_all(root.join("B/Fotos de viajes")).unwrap();std::fs::create_dir_all(root.join("node_modules/Fotos de viajes")).unwrap();let cancel=AtomicBool::new(false);let r=search_folders(vec![root.clone()],"fotos de viajes",&cancel,Duration::from_secs(2),100);assert_eq!(r.matches.len(),2);assert!(!r.incomplete);assert!(r.matches.iter().all(|m|m.exact));assert!(search_folders(vec![root.clone()],"fotos",&cancel,Duration::from_secs(2),1).incomplete);cancel.store(true,Ordering::Relaxed);assert!(search_folders(vec![root.clone()],"fotos",&cancel,Duration::from_secs(2),100).cancelled);std::fs::remove_dir_all(root).unwrap();}
}

#[derive(Clone,Serialize,Deserialize,Debug,PartialEq)]pub struct ChromeProfile{pub id:String,pub name:String}
pub fn valid_profile_id(id:&str)->bool{!id.is_empty()&&id.len()<=100&&id!="."&&id!=".."&&id.chars().all(|c|c.is_alphanumeric()||c==' '||c=='_'||c=='-')}
pub fn parse_chrome_profiles(bytes:&[u8])->Result<Vec<ChromeProfile>,String>{
 if bytes.len()>8*1024*1024{return Err("La configuración de Chrome es demasiado grande.".into())}
 #[derive(Deserialize)]struct CacheEntry{#[serde(default)]name:String}
 #[derive(Deserialize,Default)]struct Profiles{#[serde(default)]info_cache:std::collections::BTreeMap<String,CacheEntry>}
 #[derive(Deserialize)]struct LocalState{#[serde(default)]profile:Profiles}
 let state:LocalState=serde_json::from_slice(bytes).map_err(|_|"No se pudieron leer los perfiles de Chrome.")?;
 Ok(state.profile.info_cache.into_iter().filter(|(id,_)|valid_profile_id(id)).map(|(id,entry)|ChromeProfile{name:if entry.name.is_empty(){id.clone()}else{entry.name.chars().take(100).collect()},id}).take(100).collect())
}
#[cfg(test)]mod profile_tests{use super::*;#[test]fn old_actions_still_deserialize(){for json in [r#"{"kind":"app","appId":"chrome"}"#,r#"{"kind":"url","url":"https://gmail.com","browserId":"chrome"}"#]{let action:Action=serde_json::from_str(json).unwrap();match action{Action::App{profile_directory,..}|Action::Url{profile_directory,..}=>assert!(profile_directory.is_none()),_=>panic!()}}}#[test]fn reads_only_safe_profile_metadata(){let p=parse_chrome_profiles(br#"{"profile":{"info_cache":{"Default":{"name":"Personal"},"Profile 2":{"name":"Trabajo"},"../evil":{"name":"Mal"}}},"other":{"secret":"never returned"}}"#).unwrap();assert_eq!(p.len(),2);assert_eq!(p[1],ChromeProfile{id:"Profile 2".into(),name:"Trabajo".into()});for id in ["..","../Default",r"C:\Profile","--user-data-dir=x",""]{assert!(!valid_profile_id(id))}}}

#[cfg(test)] mod reorder_tests{
 use super::*;
 fn item(id:&str)->Shortcut{Shortcut{id:id.into(),name:id.into(),aliases:vec![],remember:false,actions:vec![Action::App{app_id:"test".into(),profile_directory:None}]}}
 #[test]fn preserves_entries_and_rejects_invalid_orders(){let original=vec![item("a"),item("b"),item("c")];let result=reorder_shortcuts(original.clone(),&["c".into(),"a".into(),"b".into()]).unwrap();assert_eq!(result.iter().map(|s|s.id.as_str()).collect::<Vec<_>>(),vec!["c","a","b"]);assert!(reorder_shortcuts(original.clone(),&["a".into(),"a".into(),"c".into()]).is_err());assert!(reorder_shortcuts(original.clone(),&["a".into(),"b".into()]).is_err());assert!(reorder_shortcuts(original,&["a".into(),"b".into(),"z".into()]).is_err());}
}

#[cfg(test)]mod single_application_tests{
 use super::*;
 #[test]fn app_names_are_valid_button_labels_but_shortcut_names_remain_unique(){
  let shortcut=Shortcut{id:"music".into(),name:"Spotify".into(),aliases:vec![],remember:false,actions:vec![Action::App{app_id:"spotify".into(),profile_directory:None}]};
  assert!(validate_shortcuts(&[shortcut.clone()],&["Spotify".into()]).is_ok());
  let mut duplicate=shortcut.clone();duplicate.id="other".into();duplicate.name="spotify".into();
  assert!(validate_shortcuts(&[shortcut,duplicate],&["Spotify".into()]).is_err());
 }
}
