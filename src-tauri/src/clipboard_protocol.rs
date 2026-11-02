//! Bounded application messages inside an already authenticated Noise channel.
use crate::pairing_transport::SecureChannel;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::clipboard_files::FileBundle;
use crate::shelf_shared::ShelfMetadata;
const MAX_TEXT: usize = 1024*1024;
const MAX_IMAGE: usize = 12*1024*1024;
const MAX_WIRE: usize = 135*1024*1024;
const CHUNK: usize = 32*1024;

#[derive(Clone, Debug, PartialEq)]
pub enum ClipboardPayload { Limits(usize), Text(String), Image { width:usize, height:usize, rgba:Vec<u8> }, Files(Vec<PathBuf>), FileBundle(FileBundle), ShelfCatalog(Vec<ShelfMetadata>), ShelfRequest{id:String,token:u64}, ShelfReply{id:String,token:u64,bundle:Option<FileBundle>,error:Option<String>} }
#[derive(Serialize,Deserialize)]
#[serde(tag="kind", rename_all="snake_case", deny_unknown_fields)]
enum WirePayload { Limits { mb:usize }, Text { text:String }, Image { width:usize, height:usize, rgba:String }, Files { bundle:FileBundle }, ShelfCatalog { items:Vec<ShelfMetadata> }, ShelfRequest { id:String, token:u64 }, ShelfReply { id:String, token:u64, bundle:Option<FileBundle>, error:Option<String> } }
fn image_size(width:usize,height:usize)->Option<usize>{
 if width==0||height==0||width>16384||height>16384{return None}
 width.checked_mul(height)?.checked_mul(4).filter(|n|*n<=MAX_IMAGE)
}
fn encode(payload:ClipboardPayload)->Result<Vec<u8>,String>{
 let wire=match payload{
  ClipboardPayload::Limits(mb)=>{crate::transfer_limits::validate(mb)?;WirePayload::Limits{mb}},
  ClipboardPayload::Text(text) if !text.is_empty()&&text.len()<=MAX_TEXT=>WirePayload::Text{text},
  ClipboardPayload::Image{width,height,rgba} if image_size(width,height)==Some(rgba.len())=>WirePayload::Image{width,height,rgba:STANDARD.encode(rgba)},
  ClipboardPayload::FileBundle(bundle)=>WirePayload::Files{bundle},
  ClipboardPayload::ShelfCatalog(items)=>{crate::shelf_shared::validate(&items)?;WirePayload::ShelfCatalog{items}},
  ClipboardPayload::ShelfRequest{id,token} if crate::shelf_shared::valid_id(&id)=>WirePayload::ShelfRequest{id,token},
  ClipboardPayload::ShelfReply{id,token,bundle,error} if crate::shelf_shared::valid_id(&id)&&error.as_ref().map(|s|s.len()<=200).unwrap_or(true)&&bundle.is_some()!=error.is_some()=>WirePayload::ShelfReply{id,token,bundle,error},
  _=>return Err("Contenido del portapapeles fuera de los límites".into()),
 };
 let encoded=serde_json::to_vec(&wire).map_err(|_|"No se pudo preparar el contenido")?;
 if encoded.len()>MAX_WIRE{return Err("Contenido demasiado grande".into())}Ok(encoded)
}
fn decode(data:&[u8])->Result<ClipboardPayload,String>{
 let wire:WirePayload=serde_json::from_slice(data).map_err(|_|"Formato del portapapeles inválido")?;
 match wire{
  WirePayload::Limits{mb}=>{crate::transfer_limits::validate(mb)?;Ok(ClipboardPayload::Limits(mb))},
  WirePayload::Text{text} if !text.is_empty()&&text.len()<=MAX_TEXT=>Ok(ClipboardPayload::Text(text)),
  WirePayload::Files{bundle}=>Ok(ClipboardPayload::FileBundle(bundle)),
  WirePayload::ShelfCatalog{items}=>{crate::shelf_shared::validate(&items)?;Ok(ClipboardPayload::ShelfCatalog(items))},
  WirePayload::ShelfRequest{id,token} if crate::shelf_shared::valid_id(&id)=>Ok(ClipboardPayload::ShelfRequest{id,token}),
  WirePayload::ShelfReply{id,token,bundle,error} if crate::shelf_shared::valid_id(&id)&&error.as_ref().map(|s|s.len()<=200).unwrap_or(true)&&bundle.is_some()!=error.is_some()=>Ok(ClipboardPayload::ShelfReply{id,token,bundle,error}),
  WirePayload::Image{width,height,rgba}=>{
   let expected=image_size(width,height).ok_or("Dimensiones de imagen inválidas")?;
   if rgba.len()>((expected+2)/3)*4{return Err("Imagen demasiado grande".into())}
   let bytes=STANDARD.decode(rgba).map_err(|_|"Imagen inválida")?;
   if bytes.len()!=expected{return Err("Tamaño de imagen inválido".into())}
   Ok(ClipboardPayload::Image{width,height,rgba:bytes})
  },
  _=>Err("Contenido del portapapeles fuera de los límites".into()),
 }
}
pub fn send_turn(channel:&mut SecureChannel,payload:Option<ClipboardPayload>)->Result<(),String>{
 match payload{
  None=>channel.send(&[0]),
  Some(payload)=>{
   let data=encode(payload)?;
   let mut header=[0u8;5];header[0]=1;header[1..].copy_from_slice(&(data.len() as u32).to_be_bytes());
   channel.send(&header)?;for chunk in data.chunks(CHUNK){channel.send(chunk)?}Ok(())
  }
 }
}
pub fn receive_turn(channel:&mut SecureChannel)->Result<Option<ClipboardPayload>,String>{receive_turn_with_limit(channel,crate::transfer_limits::local_bytes())}
pub fn receive_turn_with_limit(channel:&mut SecureChannel,file_limit:usize)->Result<Option<ClipboardPayload>,String>{
 let header=channel.receive()?;
 if header==[0]{return Ok(None)}
 if header.len()!=5||header[0]!=1{return Err("Cabecera de sincronización inválida".into())}
 let size=u32::from_be_bytes(header[1..5].try_into().map_err(|_|"Cabecera inválida")?) as usize;
 if size==0||size>MAX_WIRE.min(crate::transfer_limits::wire_bytes(file_limit).max(17*1024*1024)){return Err("Tamaño de sincronización inválido".into())}
 let mut data=Vec::with_capacity(size);
 while data.len()<size{let chunk=channel.receive()?;if chunk.is_empty()||chunk.len()>CHUNK||chunk.len()>size-data.len(){return Err("Fragmento de sincronización inválido".into())}data.extend_from_slice(&chunk)}
 decode(&data).map(Some)
}
#[cfg(test)] mod tests{
 use super::*;use crate::pairing_core::Role;use crate::pairing_transport::pair;use std::{net::{TcpListener,TcpStream},thread};
 #[test] fn text_and_images_round_trip_over_paired_channel(){
  let listener=TcpListener::bind("127.0.0.1:0").unwrap();let addr=listener.local_addr().unwrap();
  let text="¡hola! ".repeat(10000);let sent=text.clone();
  let host=thread::spawn(move||{let (s,_)=listener.accept().unwrap();let mut c=pair(s,Role::Host,"012345").unwrap();assert_eq!(receive_turn(&mut c).unwrap(),Some(ClipboardPayload::Text(sent)));send_turn(&mut c,Some(ClipboardPayload::Image{width:2,height:1,rgba:vec![1,2,3,4,5,6,7,8]})).unwrap();assert_eq!(receive_turn(&mut c).unwrap(),None)});
  let mut c=pair(TcpStream::connect(addr).unwrap(),Role::Client,"012345").unwrap();send_turn(&mut c,Some(ClipboardPayload::Text(text))).unwrap();assert_eq!(receive_turn(&mut c).unwrap(),Some(ClipboardPayload::Image{width:2,height:1,rgba:vec![1,2,3,4,5,6,7,8]}));send_turn(&mut c,None).unwrap();host.join().unwrap();
 }
 #[test] fn selected_files_cross_the_channel_and_are_materialized(){
  use std::{fs,time::{SystemTime,UNIX_EPOCH}};
  let unique=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
  let dir=std::env::temp_dir().join(format!("bimu-wire-files-{}-{unique}",std::process::id()));
  fs::create_dir(&dir).unwrap();fs::write(dir.join("foto.png"),b"png bytes").unwrap();fs::write(dir.join("doc.pdf"),b"pdf bytes").unwrap();
  let bundle=crate::clipboard_files::pack(&[dir.join("foto.png"),dir.join("doc.pdf")]).unwrap();
  let listener=TcpListener::bind("127.0.0.1:0").unwrap();let addr=listener.local_addr().unwrap();
  let receiver=thread::spawn(move||{let(s,_)=listener.accept().unwrap();let mut channel=pair(s,Role::Host,"012345").unwrap();receive_turn(&mut channel).unwrap().unwrap()});
  let mut channel=pair(TcpStream::connect(addr).unwrap(),Role::Client,"012345").unwrap();send_turn(&mut channel,Some(ClipboardPayload::FileBundle(bundle))).unwrap();
  let received=receiver.join().unwrap();let ClipboardPayload::FileBundle(bundle)=received else{panic!("expected file bundle")};
  let destination=dir.join("received");let paths=crate::clipboard_files::unpack(&bundle,&destination).unwrap();assert_eq!(paths.len(),2);assert_eq!(fs::read(destination.join("foto.png")).unwrap(),b"png bytes");assert_eq!(fs::read(destination.join("doc.pdf")).unwrap(),b"pdf bytes");fs::remove_dir_all(dir).unwrap();
 }
 #[test] fn shelf_catalog_request_and_response_cross_the_paired_channel(){
  use std::{fs,time::{SystemTime,UNIX_EPOCH}};
  let unique=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();let dir=std::env::temp_dir().join(format!("bimu-shelf-wire-{}-{unique}",std::process::id()));fs::create_dir(&dir).unwrap();fs::write(dir.join("trabajo.pdf"),b"documento").unwrap();
  let bundle=crate::clipboard_files::pack(&[dir.join("trabajo.pdf")]).unwrap();let id="a".repeat(64);let sent_id=id.clone();
  let listener=TcpListener::bind("127.0.0.1:0").unwrap();let addr=listener.local_addr().unwrap();
  let host=thread::spawn(move||{let(s,_)=listener.accept().unwrap();let mut channel=pair(s,Role::Host,"012345").unwrap();let item=ShelfMetadata{id:sent_id.clone(),name:"trabajo.pdf".into(),kind:"file".into(),available:true};send_turn(&mut channel,Some(ClipboardPayload::ShelfCatalog(vec![item]))).unwrap();assert_eq!(receive_turn(&mut channel).unwrap(),Some(ClipboardPayload::ShelfRequest{id:sent_id.clone(),token:42}));send_turn(&mut channel,Some(ClipboardPayload::ShelfReply{id:sent_id,token:42,bundle:Some(bundle),error:None})).unwrap();});
  let mut channel=pair(TcpStream::connect(addr).unwrap(),Role::Client,"012345").unwrap();assert!(matches!(receive_turn(&mut channel).unwrap(),Some(ClipboardPayload::ShelfCatalog(_))));send_turn(&mut channel,Some(ClipboardPayload::ShelfRequest{id:id.clone(),token:42})).unwrap();let reply=receive_turn(&mut channel).unwrap().unwrap();let ClipboardPayload::ShelfReply{id:response_id,token,bundle:Some(bundle),error:None}=reply else{panic!("expected shelf reply")};assert_eq!(response_id,id);assert_eq!(token,42);assert_eq!(bundle.single_root(),Some("trabajo.pdf"));let destination=dir.join("received");let paths=crate::clipboard_files::unpack(&bundle,&destination).unwrap();assert_eq!(fs::read(&paths[0]).unwrap(),b"documento");host.join().unwrap();fs::remove_dir_all(dir).unwrap();
 }
 #[test] fn shelf_messages_validate_metadata_and_request_ids(){
  let item=ShelfMetadata{id:"a".repeat(64),name:"guía.pdf".into(),kind:"file".into(),available:true};
  let message=ClipboardPayload::ShelfCatalog(vec![item]);
  assert_eq!(decode(&encode(message.clone()).unwrap()).unwrap(),message);
  let request=ClipboardPayload::ShelfRequest{id:"a".repeat(64),token:17};
  assert_eq!(decode(&encode(request.clone()).unwrap()).unwrap(),request);
  assert!(encode(ClipboardPayload::ShelfRequest{id:"../escape".into(),token:1}).is_err());
  assert!(encode(ClipboardPayload::ShelfReply{id:"a".repeat(64),token:1,bundle:None,error:None}).is_err());
 }
 #[test] fn rejects_invalid_images_and_large_text(){
  assert!(encode(ClipboardPayload::Text("x".repeat(MAX_TEXT+1))).is_err());
  assert!(encode(ClipboardPayload::Image{width:2,height:2,rgba:vec![1,2]}).is_err());
  assert!(decode(br#"{"kind":"image","width":999999,"height":3,"rgba":""}"#).is_err());
 }
}

#[cfg(test)]mod limit_tests{
 use super::*;
 use crate::{pairing_core::Role,pairing_transport::pair};
 use std::{net::{TcpListener,TcpStream},thread};
 #[test]fn limit_changes_cross_authenticated_channel(){
  let listener=TcpListener::bind("127.0.0.1:0").unwrap();let addr=listener.local_addr().unwrap();
  let receiver=thread::spawn(move||{let(s,_)=listener.accept().unwrap();let mut c=pair(s,Role::Host,"123456").unwrap();assert_eq!(receive_turn(&mut c).unwrap(),Some(ClipboardPayload::Limits(30)));});
  let mut c=pair(TcpStream::connect(addr).unwrap(),Role::Client,"123456").unwrap();send_turn(&mut c,Some(ClipboardPayload::Limits(30))).unwrap();receiver.join().unwrap();
  assert!(encode(ClipboardPayload::Limits(101)).is_err());assert!(decode(br#"{"kind":"limits","mb":0}"#).is_err());
 }
 #[test]fn oversized_announcement_is_rejected_before_payload_allocation(){
  let listener=TcpListener::bind("127.0.0.1:0").unwrap();let addr=listener.local_addr().unwrap();
  let receiver=thread::spawn(move||{let(s,_)=listener.accept().unwrap();let mut c=pair(s,Role::Host,"123456").unwrap();assert!(receive_turn_with_limit(&mut c,crate::transfer_limits::MIB).is_err());});
  let mut c=pair(TcpStream::connect(addr).unwrap(),Role::Client,"123456").unwrap();let mut header=[1u8;5];header[1..].copy_from_slice(&(18u32*1024*1024).to_be_bytes());c.send(&header).unwrap();receiver.join().unwrap();
 }
 #[test]fn thirty_megabyte_selection_crosses_the_authenticated_channel(){
  use std::{fs,time::{SystemTime,UNIX_EPOCH}};
  let dir=std::env::temp_dir().join(format!("bimu-30mb-{}",SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir(&dir).unwrap();let path=dir.join("large.bin");let f=fs::File::create(&path).unwrap();f.set_len(30*1024*1024).unwrap();drop(f);
  let bundle=crate::clipboard_files::pack_with_limit(&[path],30*1024*1024,&||false).unwrap();
  let listener=TcpListener::bind("127.0.0.1:0").unwrap();let addr=listener.local_addr().unwrap();let dest=dir.join("received");
  let receiver=thread::spawn(move||{let(s,_)=listener.accept().unwrap();let mut c=pair(s,Role::Host,"123456").unwrap();let Some(ClipboardPayload::FileBundle(b))=receive_turn_with_limit(&mut c,30*1024*1024).unwrap()else{panic!("expected files")};crate::clipboard_files::unpack_with_limit(&b,&dest,30*1024*1024).unwrap();assert_eq!(fs::metadata(dest.join("large.bin")).unwrap().len(),30*1024*1024);});
  let mut c=pair(TcpStream::connect(addr).unwrap(),Role::Client,"123456").unwrap();send_turn(&mut c,Some(ClipboardPayload::FileBundle(bundle))).unwrap();receiver.join().unwrap();fs::remove_dir_all(dir).unwrap();
 }
}
