//! Persistent credential protected for the current Windows user with DPAPI.
use tauri::{AppHandle,Manager};
use crate::pairing_core::Role;
use crate::pairing_transport::SecureChannel;

pub use crate::trusted_devices_core::{Trust,DeviceBook};
#[cfg(windows)]
fn protect(data:&[u8], encrypt:bool)->Result<Vec<u8>,String>{
 use std::ffi::c_void;
 #[repr(C)] struct Blob { size:u32, data:*mut u8 }
 #[link(name="crypt32")] extern "system" { fn CryptProtectData(input:*const Blob,desc:*const u16,entropy:*const Blob,reserved:*mut c_void,prompt:*mut c_void,flags:u32,output:*mut Blob)->i32; fn CryptUnprotectData(input:*const Blob,desc:*mut *mut u16,entropy:*const Blob,reserved:*mut c_void,prompt:*mut c_void,flags:u32,output:*mut Blob)->i32; }
 #[link(name="kernel32")] extern "system" {fn LocalFree(mem:*mut c_void)->*mut c_void;}
 let input=Blob{size:data.len().try_into().map_err(|_|"Datos inválidos")?,data:data.as_ptr() as *mut u8};let mut output=Blob{size:0,data:std::ptr::null_mut()};
 let ok=unsafe{if encrypt{CryptProtectData(&input,std::ptr::null(),std::ptr::null(),std::ptr::null_mut(),std::ptr::null_mut(),1,&mut output)}else{CryptUnprotectData(&input,std::ptr::null_mut(),std::ptr::null(),std::ptr::null_mut(),std::ptr::null_mut(),1,&mut output)}};
 if ok==0||output.data.is_null(){return Err("Windows no pudo proteger los datos de vinculación".into())}
 let result=unsafe{std::slice::from_raw_parts(output.data,output.size as usize).to_vec()};unsafe{LocalFree(output.data.cast());}Ok(result)
}
#[cfg(not(windows))] fn protect(_: &[u8],_:bool)->Result<Vec<u8>,String>{Err("La vinculación persistente requiere Windows".into())}
static BOOK_LOCK:std::sync::Mutex<()>=std::sync::Mutex::new(());
fn read_book(app:&AppHandle)->Result<DeviceBook,String>{crate::trusted_devices_store::read(&app.path().app_data_dir().map_err(|_|"No se pudo ubicar la carpeta de Bimu")?,protect)}
fn write_book(app:&AppHandle,book:&DeviceBook)->Result<(),String>{crate::trusted_devices_store::write(&app.path().app_data_dir().map_err(|_|"No se pudo ubicar la carpeta de Bimu")?,book,protect)}
pub fn book(app:&AppHandle)->Result<DeviceBook,String>{let _guard=BOOK_LOCK.lock().map_err(|_|"Registro de equipos ocupado")?;read_book(app)}
pub fn edit(app:&AppHandle,f:impl FnOnce(&mut DeviceBook)->Result<(),String>)->Result<(),String>{let _guard=BOOK_LOCK.lock().map_err(|_|"Registro de equipos ocupado")?;let mut b=read_book(app)?;f(&mut b)?;write_book(app,&b)}
pub fn load(app:&AppHandle)->Result<Option<Trust>,String>{Ok(book(app)?.selected().map(|d|d.trust.clone()))}
pub fn save(app:&AppHandle,trust:&Trust)->Result<(),String>{edit(app,|b|b.remember(trust.clone()))}
pub fn forget(app:&AppHandle)->Result<(),String>{edit(app,|b|{if let Some(d)=b.selected(){let id=d.id.clone();b.revoke(&id,false)?;}Ok(())})}
pub fn remove_failed(app:&AppHandle,id:&str)->Result<(),String>{edit(app,|b|{if b.devices.iter().any(|d|d.id==id&&(d.last_connected>0||d.revoked)){return Ok(())}b.devices.retain(|d|d.id!=id);if b.selected.as_deref()==Some(id){b.selected=None}if b.favorite.as_deref()==Some(id){b.favorite=None}Ok(())})}
pub fn establish(channel:&mut SecureChannel,role:Role,store:impl Fn(&Trust)->Result<(),String>,rollback:impl Fn(&Trust))->Result<(),String>{
 match role {
  Role::Host=>{
   let key=crate::random_core::key()?;channel.send(&key)?;
   if channel.receive()?.as_slice()!=b"stored"{return Err("No se pudo guardar la vinculación en la otra PC".into())}
   let trust=Trust{key,host:true};store(&trust)?;
   if let Err(error)=channel.send(b"stored"){rollback(&trust);return Err(error)}
   Ok(())
  },
  Role::Client=>{
   let data=channel.receive()?;let key:[u8;32]=data.try_into().map_err(|_|"Credencial inválida")?;
   let trust=Trust{key,host:false};store(&trust)?;
   let result=(||->Result<(),String>{channel.send(b"stored")?;if channel.receive()?.as_slice()!=b"stored"{return Err("No se pudo guardar la vinculación en la otra PC".into())}Ok(())})();
   if result.is_err(){rollback(&trust)}result
  }
 }
}
#[cfg(test)] mod tests {
 use super::*;
 use std::{net::{TcpListener,TcpStream},sync::{Arc,Mutex},thread};
 #[test] fn failed_exchange_rolls_back_the_client_credential(){
  let listener=TcpListener::bind("127.0.0.1:0").unwrap();let addr=listener.local_addr().unwrap();
  let host=thread::spawn(move||{let (socket,_)=listener.accept().unwrap();let mut c=crate::pairing_transport::pair(socket,Role::Host,"123456").unwrap();c.send(&[9u8;32]).unwrap();assert_eq!(c.receive().unwrap(),b"stored");});
  let mut c=crate::pairing_transport::pair(TcpStream::connect(addr).unwrap(),Role::Client,"123456").unwrap();
  let saved=Arc::new(Mutex::new(None));let for_store=saved.clone();let for_rollback=saved.clone();
  assert!(establish(&mut c,Role::Client,move|t|{*for_store.lock().unwrap()=Some(t.key);Ok(())},move|t|{let mut value=for_rollback.lock().unwrap();if *value==Some(t.key){*value=None}}).is_err());
  assert!(saved.lock().unwrap().is_none());host.join().unwrap();
 }
}
