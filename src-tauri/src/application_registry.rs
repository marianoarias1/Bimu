use std::path::PathBuf;
use serde::{Serialize,Deserialize};
use crate::application_intent::normalized;
#[derive(Clone,Serialize,Deserialize)]#[serde(tag="kind")]pub(crate) enum Target{Exe{path:PathBuf},Shortcut{path:PathBuf},Spotify,ShellApp{app_id:String}}
#[derive(Clone,Serialize,Deserialize)]pub(crate) struct Entry{pub id:String,pub name:String,pub aliases:Vec<String>,pub remember:bool,pub target:Target}
#[derive(Serialize)]pub struct AppInfo{pub id:String,pub name:String,pub aliases:Vec<String>,pub remember:bool,pub available:bool,pub target:String}
pub(crate) fn available(entry:&Entry)->bool{match &entry.target{Target::Exe{path}|Target::Shortcut{path}=>path.is_file(),Target::Spotify=>spotify_installed(),Target::ShellApp{..}=>true}}
#[cfg(windows)]fn spotify_installed()->bool{winreg::RegKey::predef(winreg::enums::HKEY_CLASSES_ROOT).open_subkey("spotify").is_ok()}
#[cfg(not(windows))]fn spotify_installed()->bool{false}
pub(crate) fn view(e:&Entry)->AppInfo{AppInfo{id:e.id.clone(),name:e.name.clone(),aliases:e.aliases.clone(),remember:e.remember,available:available(e),target:match &e.target{Target::Exe{path}|Target::Shortcut{path}=>path.display().to_string(),Target::Spotify=>"Protocolo registrado de Spotify (spotify:)".into(),Target::ShellApp{..}=>"Aplicación de Windows".into()}}}
pub(crate) fn validate(entries:&[Entry])->Result<(),String>{
    if entries.len()>1000{return Err("Podés registrar hasta 1000 aplicaciones.".into());}let mut names=std::collections::HashSet::new();
    for e in entries{if e.name.trim().is_empty()||e.name.len()>100||e.aliases.len()>10{return Err("Nombre o alias inválidos.".into());}let mut own=std::collections::HashSet::new();for n in std::iter::once(&e.name).chain(e.aliases.iter()){let n=normalized(n);if n.is_empty()||n.len()>100{return Err("Alias vacío o demasiado largo.".into());}if own.insert(n.clone())&&!names.insert(n){return Err("Ese nombre o alias ya pertenece a otra aplicación.".into());}}match &e.target{Target::Exe{path}=>{if !path.is_absolute()||path.extension().and_then(|s|s.to_str()).unwrap_or("").to_ascii_lowercase()!="exe"{return Err("La aplicación debe ser un ejecutable .exe local.".into());}},Target::Shortcut{path}=>{if !path.is_absolute()||path.extension().and_then(|s|s.to_str()).unwrap_or("").to_ascii_lowercase()!="lnk"{return Err("El acceso directo debe ser un archivo .lnk local.".into());}},Target::ShellApp{app_id}=>if !valid_app_id(app_id){return Err("Identificador de aplicación inválido.".into())},Target::Spotify=>{}}}Ok(())
}
pub(crate) fn valid_app_id(id:&str)->bool{!id.trim().is_empty()&&id.len()<=512&&!id.contains("..")&&!id.chars().any(|c|c.is_control()||matches!(c,'\\'|'/'|':'|'"'))}
pub(crate) fn defaults()->Vec<Entry>{
    let system=std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(||PathBuf::from("C:\\Windows"));
    let local=std::env::var_os("LOCALAPPDATA").map(PathBuf::from);let pf=std::env::var_os("ProgramFiles").map(PathBuf::from);let pf86=std::env::var_os("ProgramFiles(x86)").map(PathBuf::from);
    let first=|paths:Vec<PathBuf>|paths.iter().find(|p|p.is_file()).cloned().or_else(||paths.first().cloned()).unwrap_or_else(||system.join("missing.exe"));
    let mut entries=vec![Entry{id:"spotify".into(),name:"Spotify".into(),aliases:vec![],remember:false,target:Target::Spotify},Entry{id:"notepad".into(),name:"Bloc de notas".into(),aliases:vec!["notepad".into()],remember:false,target:Target::Exe{path:system.join("System32/notepad.exe")}},Entry{id:"calculator".into(),name:"Calculadora".into(),aliases:vec!["calc".into()],remember:false,target:Target::Exe{path:system.join("System32/calc.exe")}}];
    for(id,name,aliases,paths)in [
        ("chrome","Chrome",vec!["Google Chrome"],pf.iter().chain(pf86.iter()).chain(local.iter()).map(|p|p.join("Google/Chrome/Application/chrome.exe")).collect()),
        ("edge","Edge",vec!["Microsoft Edge"],pf86.iter().chain(pf.iter()).map(|p|p.join("Microsoft/Edge/Application/msedge.exe")).collect()),
        ("vscode","Visual Studio Code",vec!["VS Code","vscode","code"],local.iter().map(|p|p.join("Programs/Microsoft VS Code/Code.exe")).chain(pf.iter().map(|p|p.join("Microsoft VS Code/Code.exe"))).collect())
    ]{entries.push(Entry{id:id.into(),name:name.into(),aliases:aliases.into_iter().map(str::to_string).collect(),remember:false,target:Target::Exe{path:first(paths)}})}entries
}

#[cfg(test)]mod tests{use super::*;#[test]fn rejects_alias_ambiguity(){let first=Entry{id:"a".into(),name:"Editor".into(),aliases:vec!["code".into()],remember:false,target:Target::Exe{path:std::env::temp_dir().join("editor.exe")}};let mut second=first.clone();second.id="b".into();second.name="Otro".into();assert!(validate(&[first.clone()]).is_ok());assert!(validate(&[first,second]).is_err());}#[test]fn never_accepts_scripts_as_applications(){let entry=Entry{id:"a".into(),name:"Script".into(),aliases:vec![],remember:false,target:Target::Exe{path:std::env::temp_dir().join("run.cmd")}};assert!(validate(&[entry]).is_err());}}

#[cfg(test)]mod discovered_tests{use super::*;#[test]fn validates_discovered_ids_and_preserves_existing_entries(){assert!(valid_app_id("Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"));for id in ["", "../bad", "C:\\bad.exe", "bad\nname"]{assert!(!valid_app_id(id))}let entry=Entry{id:"new".into(),name:"Calculadora moderna".into(),aliases:vec![],remember:false,target:Target::ShellApp{app_id:"Microsoft.WindowsCalculator_8wekyb3d8bbwe!App".into()}};let json=serde_json::to_string(&entry).unwrap();let decoded:Entry=serde_json::from_str(&json).unwrap();assert!(validate(&[decoded]).is_ok());}}

#[cfg(test)]
mod migration_tests {
    use super::*;
    #[test]
    fn accepts_shortcuts_and_catalogs_larger_than_the_old_limit() {
        let shortcut = Entry {
            id: "discord".into(), name: "Discord".into(), aliases: vec![], remember: false,
            target: Target::Shortcut { path: std::env::temp_dir().join("Discord.lnk") },
        };
        let encoded = serde_json::to_vec(&shortcut).unwrap();
        let decoded: Entry = serde_json::from_slice(&encoded).unwrap();
        let entries = (0..1001).map(|index| Entry {
            id: format!("app-{index}"), name: format!("App {index}"),
            aliases: vec![], remember: false, target: decoded.target.clone(),
        }).collect::<Vec<_>>();
        assert!(validate(&entries[..31]).is_ok());
        assert!(validate(&entries[..1000]).is_ok());
        assert!(validate(&entries).is_err());
    }
}
