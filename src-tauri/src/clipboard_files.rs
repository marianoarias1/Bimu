//! Bounded clipboard file transfers. Paths are relative, never sender paths.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs::{self,File}, io::Read, path::{Path,PathBuf}};

pub const MAX_FILE_BYTES:usize=crate::transfer_limits::MAX_MB*crate::transfer_limits::MIB;
const MAX_ENTRIES:usize=500;
const MAX_ROOTS:usize=20;
const MAX_DEPTH:usize=16;
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileBundle{roots:Vec<String>,entries:Vec<FileEntry>}
impl FileBundle{pub fn encoded_bytes(&self)->usize{self.entries.iter().map(|e|e.data.len()).sum()}pub fn single_root(&self)->Option<&str>{if self.roots.len()==1{Some(&self.roots[0])}else{None}}}
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct FileEntry{path:String,folder:bool,data:String}
fn safe_name(name:&str)->bool{
 if name.is_empty()||name=="."||name==".."||name.len()>255||name.ends_with('.')||name.ends_with(' ')||name.chars().any(|c|c.is_control()||matches!(c,'/'|'\\'|':'|'*'|'?'|'"'|'<'|'>'|'|')){return false}
 let stem=name.split('.').next().unwrap_or("").trim_end().to_ascii_uppercase();
 if ["CON","PRN","AUX","NUL","CONIN$","CONOUT$"].contains(&stem.as_str()){return false}
 if stem.strip_prefix("COM").or_else(||stem.strip_prefix("LPT")).is_some_and(|n|matches!(n,"1"|"2"|"3"|"4"|"5"|"6"|"7"|"8"|"9"|"¹"|"²"|"³")){return false}
 true
}
fn parts(path:&str)->Result<Vec<&str>,String>{
 if path.len()>2048{return Err("Ruta demasiado larga".into())}
 let parts:Vec<_>=path.split('/').collect();
 if parts.len()>MAX_DEPTH||parts.iter().any(|p|!safe_name(p)){return Err("Nombre o ruta de archivo no permitida".into())}Ok(parts)
}
fn reparse(meta:&fs::Metadata)->bool{
 if meta.file_type().is_symlink(){return true}
 #[cfg(windows)]{use std::os::windows::fs::MetadataExt;return meta.file_attributes()&0x400!=0}
 #[cfg(not(windows))]{false}
}
fn pack_entry(path:&Path,relative:String,entries:&mut Vec<FileEntry>,total:&mut usize,limit:usize,cancel:&impl Fn()->bool)->Result<(),String>{
 if cancel(){return Err("Transferencia cancelada".into())}
 parts(&relative)?;
 if entries.len()>=MAX_ENTRIES{return Err("La selección contiene demasiados archivos".into())}
 let meta=fs::symlink_metadata(path).map_err(|_|"No se pudo leer un archivo")?;
 if reparse(&meta){return Err("No se transfieren accesos simbólicos ni puntos de unión".into())}
 if meta.is_dir(){
  entries.push(FileEntry{path:relative.clone(),folder:true,data:String::new()});
  let mut children=Vec::new();
  for child in fs::read_dir(path).map_err(|_|"No se pudo leer una carpeta")?{
   if children.len()+entries.len()>=MAX_ENTRIES{return Err("La selección contiene demasiados archivos".into())}
   children.push(child.map_err(|_|"No se pudo leer una carpeta")?);
  }
  children.sort_by_key(|c|c.file_name());
  for child in children{let name=child.file_name().into_string().map_err(|_|"Nombre de archivo no compatible")?;if !safe_name(&name){return Err("Nombre de archivo no compatible".into())}pack_entry(&child.path(),format!("{relative}/{name}"),entries,total,limit,cancel)?;}
 }else if meta.is_file(){
  let remaining=limit.checked_sub(*total).ok_or("La selección supera el límite configurado entre los equipos")?;
  if meta.len()>remaining as u64{return Err("La selección supera el límite configurado entre los equipos".into())}
  let file=File::open(path).map_err(|_|"No se pudo abrir un archivo")?;
  let actual=file.metadata().map_err(|_|"No se pudo leer un archivo")?;
  if reparse(&actual)||!actual.is_file(){return Err("No se transfieren accesos simbólicos".into())}
  let mut bytes=Vec::new();let mut reader=file.take(remaining as u64+1);let mut chunk=[0u8;32*1024];loop{if cancel(){return Err("Transferencia cancelada".into())}let n=reader.read(&mut chunk).map_err(|_|"No se pudo leer un archivo")?;if n==0{break}bytes.extend_from_slice(&chunk[..n]);}
  if bytes.len()>remaining{return Err("La selección supera el límite configurado entre los equipos".into())}
  *total+=bytes.len();entries.push(FileEntry{path:relative,folder:false,data:STANDARD.encode(bytes)});
 }else{return Err("Tipo de archivo no compatible".into())}Ok(())
}
pub fn pack(paths:&[PathBuf])->Result<FileBundle,String>{pack_with_limit(paths,crate::transfer_limits::local_bytes(),&||false)}
pub fn pack_with_limit(paths:&[PathBuf],limit:usize,cancel:&impl Fn()->bool)->Result<FileBundle,String>{
 if limit==0||limit>MAX_FILE_BYTES{return Err("Límite de transferencia inválido".into())}
 if paths.is_empty()||paths.len()>MAX_ROOTS{return Err("Seleccioná entre 1 y 20 archivos o carpetas".into())}
 let mut roots=Vec::new();let mut names=HashSet::new();let mut entries=Vec::new();let mut total=0usize;
 for path in paths{
  let name=path.file_name().and_then(|s|s.to_str()).ok_or("Nombre de archivo no compatible")?;
  if !safe_name(name)||!names.insert(name.to_lowercase()){return Err("Los archivos tienen nombres incompatibles o repetidos".into())}
  roots.push(name.to_string());pack_entry(path,name.to_string(),&mut entries,&mut total,limit,cancel)?;
 }Ok(FileBundle{roots,entries})
}
/// Validate the entire manifest before touching disk, then write only under a
/// newly-created private destination. Returns original selected root items.
pub fn unpack(bundle:&FileBundle,destination:&Path)->Result<Vec<PathBuf>,String>{unpack_with_limit(bundle,destination,crate::transfer_limits::local_bytes())}
pub fn unpack_with_limit(bundle:&FileBundle,destination:&Path,limit:usize)->Result<Vec<PathBuf>,String>{
 if limit==0||limit>MAX_FILE_BYTES{return Err("Límite de transferencia inválido".into())}
 if bundle.roots.is_empty()||bundle.roots.len()>MAX_ROOTS||bundle.entries.is_empty()||bundle.entries.len()>MAX_ENTRIES{return Err("Selección recibida inválida".into())}
 let mut root_names=HashSet::new();
 for root in &bundle.roots{if !safe_name(root)||!root_names.insert(root.to_lowercase()){return Err("Nombres recibidos inválidos".into())}}
 let mut paths=HashSet::new();let mut root_entries=HashSet::new();let mut total=0usize;
 for item in &bundle.entries{
  let components=parts(&item.path)?;let root=components[0].to_lowercase();
  if !root_names.contains(&root)||!paths.insert(item.path.to_lowercase()){return Err("Ruta repetida o ajena a la selección".into())}
  if components.len()==1{root_entries.insert(root);}
  if item.folder{if !item.data.is_empty(){return Err("Carpeta recibida inválida".into())}}
  else{
   let remaining=limit.checked_sub(total).ok_or("La selección supera el límite configurado entre los equipos")?;
   if item.data.len()>((remaining+2)/3)*4{return Err("La selección supera el límite configurado entre los equipos".into())}
   let decoded=STANDARD.decode(&item.data).map_err(|_|"Archivo recibido inválido")?;
   total=total.checked_add(decoded.len()).ok_or("La selección supera el límite configurado entre los equipos")?;
   if total>limit{return Err("La selección supera el límite configurado entre los equipos".into())}
  }
 }
 if root_entries!=root_names{return Err("Faltan elementos de la selección".into())}
 fs::create_dir(destination).map_err(|_|"No se pudo crear la carpeta de recepción")?;
 let result=(||->Result<Vec<PathBuf>,String>{
  for item in &bundle.entries{
   let mut target=destination.to_path_buf();for component in parts(&item.path)?{target.push(component)}
   if item.folder{fs::create_dir_all(&target).map_err(|_|"No se pudo crear una carpeta recibida")?}
   else{if let Some(parent)=target.parent(){fs::create_dir_all(parent).map_err(|_|"No se pudo crear una carpeta recibida")?}
    let bytes=STANDARD.decode(&item.data).map_err(|_|"Archivo recibido inválido")?;
    use std::io::Write;let mut file=fs::OpenOptions::new().write(true).create_new(true).open(&target).map_err(|_|"No se pudo guardar un archivo recibido")?;
    file.write_all(&bytes).map_err(|_|"No se pudo guardar un archivo recibido")?;
   }
  }
  Ok(bundle.roots.iter().map(|root|destination.join(root)).collect())
 })();
 if result.is_err(){let _=fs::remove_dir_all(destination);}result
}
#[cfg(test)]mod tests{
 use super::*;use std::time::{SystemTime,UNIX_EPOCH};
 fn temp()->PathBuf{let n=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();let p=std::env::temp_dir().join(format!("bimu-files-test-{}-{n}",std::process::id()));fs::create_dir(&p).unwrap();p}
 #[test]fn copies_files_and_nested_folders(){let dir=temp();fs::create_dir(dir.join("carpeta")).unwrap();fs::write(dir.join("carpeta").join("imagen.png"),b"image-data").unwrap();fs::write(dir.join("doc.pdf"),b"pdf-data").unwrap();let b=pack(&[dir.join("carpeta"),dir.join("doc.pdf")]).unwrap();let output=dir.join("received");let roots=unpack(&b,&output).unwrap();assert_eq!(roots.len(),2);assert_eq!(fs::read(output.join("carpeta").join("imagen.png")).unwrap(),b"image-data");assert_eq!(fs::read(output.join("doc.pdf")).unwrap(),b"pdf-data");fs::remove_dir_all(dir).unwrap();}
 #[test]fn rejects_files_over_limit(){let dir=temp();let file=File::create(dir.join("too-large.pdf")).unwrap();file.set_len(MAX_FILE_BYTES as u64+1).unwrap();drop(file);assert!(pack(&[dir.join("too-large.pdf")]).is_err());fs::remove_dir_all(dir).unwrap();}
 #[test]fn rejects_traversal_duplicates_and_windows_reserved_names(){let dir=temp();for bad in ["../escape","/absoluto","C:/escape","carpeta/../escape","NUL.txt","carpeta\\escape"]{let b=FileBundle{roots:vec!["carpeta".into()],entries:vec![FileEntry{path:bad.into(),folder:false,data:String::new()}]};let destination=dir.join("bad");assert!(unpack(&b,&destination).is_err(),"{bad}");assert!(!destination.exists());}let b=FileBundle{roots:vec!["a".into()],entries:vec![FileEntry{path:"a".into(),folder:false,data:String::new()},FileEntry{path:"A".into(),folder:false,data:String::new()}]};assert!(unpack(&b,&dir.join("duplicate")).is_err());fs::remove_dir_all(dir).unwrap();}
}

#[cfg(test)]mod transfer_policy_tests{
 use super::*;
 #[test]fn configured_total_limit_and_cancellation(){
  let dir=std::env::temp_dir().join(format!("bimu-limit-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir(&dir).unwrap();
  fs::write(dir.join("a.txt"),[1u8;600]).unwrap();fs::write(dir.join("b.txt"),[2u8;500]).unwrap();let paths=vec![dir.join("a.txt"),dir.join("b.txt")];
  assert!(pack_with_limit(&paths,1000,&||false).is_err());
  let bundle=pack_with_limit(&paths,1100,&||false).unwrap();
  assert!(unpack_with_limit(&bundle,&dir.join("rejected"),1000).is_err());assert!(!dir.join("rejected").exists());
  assert!(pack_with_limit(&paths,1100,&||true).is_err());
  assert!(unpack_with_limit(&bundle,&dir.join("ok"),1100).is_ok());fs::remove_dir_all(dir).unwrap();
 }
 #[test]fn thirty_megabytes_accepts_larger_than_old_limit(){
  let dir=std::env::temp_dir().join(format!("bimu-large-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir(&dir).unwrap();let path=dir.join("large.bin");let f=File::create(&path).unwrap();f.set_len(11*1024*1024).unwrap();drop(f);
  assert!(pack_with_limit(&[path.clone()],10*1024*1024,&||false).is_err());
  let bundle=pack_with_limit(&[path],30*1024*1024,&||false).unwrap();unpack_with_limit(&bundle,&dir.join("received"),30*1024*1024).unwrap();assert_eq!(fs::metadata(dir.join("received/large.bin")).unwrap().len(),11*1024*1024);fs::remove_dir_all(dir).unwrap();
 }
}

#[cfg(test)]mod device_alias_tests{
 use super::*;
 #[test]fn rejects_device_aliases_before_creating_a_destination(){
  let root=std::env::temp_dir().join(format!("bimu-device-alias-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir(&root).unwrap();
  for name in ["COM¹.txt","com²","LPT³.pdf","CONIN$","conout$","NUL.txt","LPT9"]{
   let b=FileBundle{roots:vec![name.into()],entries:vec![FileEntry{path:name.into(),folder:false,data:String::new()}]};
   let target=root.join("rejected");assert!(unpack(&b,&target).is_err(),"{name}");assert!(!target.exists());
  }
  assert!(safe_name("informe-ñ.pdf"));assert!(safe_name("COM10.txt"));fs::remove_dir_all(root).unwrap();
 }
}
