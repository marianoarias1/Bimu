//! Atomic migration/storage independent of UI and the platform encryption API.
use std::{path::Path,io::ErrorKind};
use crate::trusted_devices_core::{DeviceBook,Trust};
pub fn read(dir:&Path,crypt:impl Fn(&[u8],bool)->Result<Vec<u8>,String>)->Result<DeviceBook,String>{
 match crate::bounded_io::read_limited(&dir.join("linked-devices-v2.dat"),128*1024){
  Ok(bytes)=>{let b:DeviceBook=serde_json::from_slice(&crypt(&bytes,false)?).map_err(|_|"Registro de equipos inválido")?;b.validate()?;Ok(b)},
  Err(e)if e.kind()==ErrorKind::NotFound=>{let mut b=DeviceBook::default();match crate::bounded_io::read_limited(&dir.join("linked-device.dat"),4096){
   Ok(bytes)=>{let trust:Trust=serde_json::from_slice(&crypt(&bytes,false)?).map_err(|_|"Vinculación anterior inválida")?;b.remember(trust)?;write(dir,&b,crypt)?;},
   Err(e)if e.kind()==ErrorKind::NotFound=>{},Err(_)=>return Err("No se pudo leer la vinculación anterior".into())
  }Ok(b)},
  Err(_)=>Err("No se pudo leer el registro de equipos".into())
 }
}
pub fn write(dir:&Path,book:&DeviceBook,crypt:impl Fn(&[u8],bool)->Result<Vec<u8>,String>)->Result<(),String>{
 book.validate()?;let bytes=crypt(&serde_json::to_vec(book).map_err(|_|"Registro inválido")?,true)?;
 std::fs::create_dir_all(dir).map_err(|_|"No se pudo crear la carpeta de Bimu")?;crate::config_migration::write_atomic(&dir.join("linked-devices-v2.dat"),&bytes)?;
 match std::fs::remove_file(dir.join("linked-device.dat")){Ok(())=>Ok(()),Err(e)if e.kind()==ErrorKind::NotFound=>Ok(()),Err(_)=>Err("No se pudo completar la migración del vínculo anterior".into())}
}
#[cfg(test)]mod tests{
 use super::*;fn dir()->std::path::PathBuf{let p=std::env::temp_dir().join(format!("bimu-device-store-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir_all(&p).unwrap();p}
 fn crypt(bytes:&[u8],_:bool)->Result<Vec<u8>,String>{Ok(bytes.to_vec())}
 fn old(path:&Path)->Trust{let t=Trust{key:[7;32],host:true};std::fs::write(path.join("linked-device.dat"),serde_json::to_vec(&t).unwrap()).unwrap();t}
 #[test]fn legacy_migration_retains_the_exact_key_and_original_role(){let p=dir();let t=old(&p);let b=read(&p,crypt).unwrap();assert_eq!(b.selected().unwrap().trust.key,t.key);assert!(b.selected().unwrap().trust.host);assert!(!p.join("linked-device.dat").exists());let restored=read(&p,crypt).unwrap();assert_eq!(restored.devices.len(),1);assert_eq!(restored.selected().unwrap().trust.key,t.key);std::fs::remove_dir_all(p).unwrap();}
 #[test]fn encryption_failure_preserves_the_previous_credential(){let p=dir();old(&p);assert!(read(&p,|b,encrypt|if encrypt{Err("unavailable".into())}else{Ok(b.to_vec())}).is_err());assert!(p.join("linked-device.dat").exists());assert!(!p.join("linked-devices-v2.dat").exists());std::fs::remove_dir_all(p).unwrap();}
 #[test]fn damaged_new_store_never_falls_back_to_a_stale_legacy_link(){let p=dir();old(&p);std::fs::write(p.join("linked-devices-v2.dat"),b"broken").unwrap();assert!(read(&p,crypt).is_err());std::fs::remove_dir_all(p).unwrap();}
}
