//! Bounded application messages inside an already authenticated Noise channel.
use crate::pairing_transport::SecureChannel;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::clipboard_files::FileBundle;
use crate::shelf_shared::ShelfMetadata;
const MAX_TEXT: usize = 1024*1024;
const MAX_IMAGE: usize = 12*1024*1024;
const MAX_WIRE: usize = crate::transfer_limits::wire_bytes(crate::transfer_limits::MAX_MB*crate::transfer_limits::MIB);
const CHUNK: usize = 32*1024;

#[derive(Clone, Debug, PartialEq)]
pub enum ClipboardPayload { Limits{mb:usize,configured:bool}, Text(String), Image { width:usize, height:usize, rgba:Vec<u8> }, Files(Vec<PathBuf>), FileBundle(FileBundle), ShelfCatalog(Vec<ShelfMetadata>), ShelfRequest{id:String,token:u64}, ShelfReply{id:String,token:u64,bundle:Option<FileBundle>,error:Option<String>} }
#[derive(Serialize,Deserialize)]
#[serde(tag="kind", rename_all="snake_case", deny_unknown_fields)]
enum WirePayload { Limits { mb:usize, configured:bool }, Text { text:String }, Image { width:usize, height:usize, rgba:String }, Files { bundle:FileBundle }, ShelfCatalog { items:Vec<ShelfMetadata> }, ShelfRequest { id:String, token:u64 }, ShelfReply { id:String, token:u64, bundle:Option<FileBundle>, error:Option<String> } }
fn image_size(width:usize,height:usize)->Option<usize>{
 if width==0||height==0||width>16384||height>16384{return None}
 width.checked_mul(height)?.checked_mul(4).filter(|n|*n<=MAX_IMAGE)
}
fn encode(payload:ClipboardPayload)->Result<Vec<u8>,String>{
 let wire=match payload{
  ClipboardPayload::Limits{mb,configured}=>{crate::transfer_limits::validate(mb)?;WirePayload::Limits{mb,configured}},
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
  WirePayload::Limits{mb,configured}=>{crate::transfer_limits::validate(mb)?;Ok(ClipboardPayload::Limits{mb,configured})},
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
   // A bounded preparation notice keeps the ordinary idle timeout short while
   // allowing JSON encoding/decoding of a large selection on slower computers.
   let large=match &payload{ClipboardPayload::FileBundle(b)=>b.encoded_bytes()>8*1024*1024,ClipboardPayload::ShelfReply{bundle:Some(b),..}=>b.encoded_bytes()>8*1024*1024,_=>false};
   if large{channel.send(&[2])?;channel.receive_timeout(120)?;}
   let data=encode(payload)?;
   let mut header=[0u8;5];header[0]=1;header[1..].copy_from_slice(&(data.len() as u32).to_be_bytes());
   channel.send(&header)?;for chunk in data.chunks(CHUNK){channel.send(chunk)?}Ok(())
  }
 }
}
pub fn receive_turn(channel:&mut SecureChannel)->Result<Option<ClipboardPayload>,String>{receive_turn_with_limit(channel,crate::transfer_limits::local_bytes())}
pub fn receive_turn_with_limit(channel:&mut SecureChannel,file_limit:usize)->Result<Option<ClipboardPayload>,String>{
 let result=receive_payload(channel,file_limit);let restored=channel.receive_timeout(5);
 match result{Ok(value)=>{restored?;Ok(value)},Err(error)=>Err(error)}
}
fn receive_payload(channel:&mut SecureChannel,file_limit:usize)->Result<Option<ClipboardPayload>,String>{
 let mut header=channel.receive()?;
 // Only one notice per turn: authenticated peers cannot extend it indefinitely.
 if header==[2]{channel.receive_timeout(120)?;header=channel.receive()?;}
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
  let receiver=thread::spawn(move||{let(s,_)=listener.accept().unwrap();let mut c=pair(s,Role::Host,"123456").unwrap();assert_eq!(receive_turn(&mut c).unwrap(),Some(ClipboardPayload::Limits{mb:250,configured:true}));});
  let mut c=pair(TcpStream::connect(addr).unwrap(),Role::Client,"123456").unwrap();send_turn(&mut c,Some(ClipboardPayload::Limits{mb:250,configured:true})).unwrap();receiver.join().unwrap();
  assert!(encode(ClipboardPayload::Limits{mb:251,configured:true}).is_err());assert!(decode(br#"{"kind":"limits","mb":0,"configured":true}"#).is_err());
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
 } #[test]fn maximum_250_megabyte_selection_crosses_the_authenticated_channel(){
  use std::{fs,time::{SystemTime,UNIX_EPOCH}};
  let dir=std::env::temp_dir().join(format!("bimu-250mb-{}",SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir(&dir).unwrap();let path=dir.join("large.bin");let f=fs::File::create(&path).unwrap();f.set_len(250*1024*1024).unwrap();drop(f);
  let bundle=crate::clipboard_files::pack_with_limit(&[path],250*1024*1024,&||false).unwrap();
  let listener=TcpListener::bind("127.0.0.1:0").unwrap();let addr=listener.local_addr().unwrap();let dest=dir.join("received");
  let receiver=thread::spawn(move||{let(s,_)=listener.accept().unwrap();let mut c=pair(s,Role::Host,"123456").unwrap();let Some(ClipboardPayload::FileBundle(b))=receive_turn_with_limit(&mut c,250*1024*1024).unwrap()else{panic!("expected files")};crate::clipboard_files::unpack_with_limit(&b,&dest,250*1024*1024).unwrap();assert_eq!(fs::metadata(dest.join("large.bin")).unwrap().len(),250*1024*1024);});
  let mut c=pair(TcpStream::connect(addr).unwrap(),Role::Client,"123456").unwrap();send_turn(&mut c,Some(ClipboardPayload::FileBundle(bundle))).unwrap();receiver.join().unwrap();fs::remove_dir_all(dir).unwrap();
 }
}

#[cfg(test)]mod folder_regression_tests{
 use super::*;use crate::{pairing_core::Role,pairing_transport::pair,transfer_limits::{self,MIB}};
 use std::{fs,io::{Seek,SeekFrom,Write},net::{TcpListener,TcpStream},thread,time::{SystemTime,UNIX_EPOCH}};
 #[test]fn shelf_folder_70mb_is_listed_then_downloaded_with_main_250_secondary_30(){
  let dir=std::env::temp_dir().join(format!("bimu-shelf70-{}",SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
  let folder=dir.join("Proyecto70");fs::create_dir_all(folder.join("subcarpeta")).unwrap();
  for name in ["primero.bin","subcarpeta/segundo.bin"]{let mut f=fs::File::create(folder.join(name)).unwrap();f.set_len((35*MIB)as u64).unwrap();f.write_all(b"inicio").unwrap();f.seek(SeekFrom::End(-3)).unwrap();f.write_all(b"fin").unwrap();}
  let bundle=crate::clipboard_files::pack_with_limit(&[folder],250*MIB,&||false).unwrap();
  let listener=TcpListener::bind("127.0.0.1:0").unwrap();let addr=listener.local_addr().unwrap();let id="b".repeat(64);let host_id=id.clone();
  let host=thread::spawn(move||{
   let(s,_)=listener.accept().unwrap();let mut c=pair(s,Role::Host,"123456").unwrap();
   c.send(transfer_limits::hello("Principal",250,true).unwrap().as_bytes()).unwrap();
   send_turn(&mut c,Some(ClipboardPayload::ShelfCatalog(vec![ShelfMetadata{id:host_id.clone(),name:"Proyecto70".into(),kind:"folder".into(),available:true}]))).unwrap();
   assert_eq!(receive_turn_with_limit(&mut c,250*MIB).unwrap(),Some(ClipboardPayload::ShelfRequest{id:host_id.clone(),token:7}));
   send_turn(&mut c,Some(ClipboardPayload::ShelfReply{id:host_id,token:7,bundle:Some(bundle),error:None})).unwrap();
   assert_eq!(receive_turn_with_limit(&mut c,250*MIB).unwrap(),None);
  });
  let mut c=pair(TcpStream::connect(addr).unwrap(),Role::Client,"123456").unwrap();
  let(_,peer,configured)=transfer_limits::parse_hello(std::str::from_utf8(&c.receive().unwrap()).unwrap()).unwrap();
  let(send,receive)=transfer_limits::session_mb(30,peer,false,true,configured);assert_eq!((send,receive),(250,250));
  let Some(ClipboardPayload::ShelfCatalog(items))=receive_turn_with_limit(&mut c,receive*MIB).unwrap()else{panic!("missing catalog")};
  assert_eq!(items.len(),1);assert_eq!(items[0].kind,"folder");assert!(items[0].available);
  send_turn(&mut c,Some(ClipboardPayload::ShelfRequest{id:id.clone(),token:7})).unwrap();
  let Some(ClipboardPayload::ShelfReply{bundle:Some(b),error:None,..})=receive_turn_with_limit(&mut c,receive*MIB).unwrap()else{panic!("missing folder")};
  assert!(crate::clipboard_files::unpack_with_limit(&b,&dir.join("too-small"),30*MIB).is_err());assert!(!dir.join("too-small").exists());
  crate::clipboard_files::unpack_with_limit(&b,&dir.join("received"),receive*MIB).unwrap();
  for name in ["primero.bin","subcarpeta/segundo.bin"]{let bytes=fs::read(dir.join("received/Proyecto70").join(name)).unwrap();assert_eq!(bytes.len(),35*MIB);assert_eq!(&bytes[..6],b"inicio");assert_eq!(&bytes[bytes.len()-3..],b"fin");}
  send_turn(&mut c,None).unwrap();host.join().unwrap();fs::remove_dir_all(dir).unwrap();
 }
}
