//! Capability-negotiated control phase, before any clipboard or catalog data.
use serde::{Deserialize,Serialize};
use crate::{pairing_core::Role,pairing_transport::SecureChannel};
#[derive(Clone,Copy,Serialize,Deserialize)]#[serde(deny_unknown_fields)]pub struct Intent{pub version:u8,pub revoked:bool,pub paused:bool,pub resume:bool}
#[derive(Debug,PartialEq)]pub enum Decision{Sync,Pause,Forget}
pub fn decision(a:Intent,b:Intent)->Decision{if a.revoked||b.revoked{Decision::Forget}else if (a.paused||b.paused)&&!a.resume&&!b.resume{Decision::Pause}else{Decision::Sync}}
pub fn exchange(channel:&mut SecureChannel,role:Role,own:Intent)->Result<Decision,String>{
 let bytes=serde_json::to_vec(&own).map_err(|_|"Control inválido")?;
 let raw=match role{Role::Client=>{channel.send(&bytes)?;channel.receive()?},Role::Host=>{let raw=channel.receive()?;channel.send(&bytes)?;raw}};
 if raw.len()>256{return Err("Control de conexión demasiado grande".into())}
 let peer:Intent=serde_json::from_slice(&raw).map_err(|_|"Control de conexión inválido")?;if peer.version!=1{return Err("Actualizá Bimu en ambas PCs".into())}Ok(decision(own,peer))
}
#[cfg(test)]mod tests{use super::*;#[test]fn revocation_always_wins_and_only_explicit_resume_unpauses(){let normal=Intent{version:1,revoked:false,paused:false,resume:false};let paused=Intent{paused:true,..normal};assert_eq!(decision(normal,paused),Decision::Pause);assert_eq!(decision(Intent{resume:true,..normal},paused),Decision::Sync);assert_eq!(decision(Intent{resume:true,..normal},Intent{revoked:true,..paused}),Decision::Forget);}}

#[cfg(test)]mod transport_tests{
 use super::*;use crate::pairing_transport::pair;use std::{net::{TcpListener,TcpStream},thread};
 #[test]fn authenticated_revocation_and_pause_are_exchanged_before_data(){
  for (revoked,paused,resume,expected) in [(true,false,true,Decision::Forget),(false,true,false,Decision::Pause),(false,true,true,Decision::Sync)]{
   let listener=TcpListener::bind("127.0.0.1:0").unwrap();let addr=listener.local_addr().unwrap();
   let host=thread::spawn(move||{let(socket,_)=listener.accept().unwrap();let mut channel=pair(socket,Role::Host,"123456").unwrap();exchange(&mut channel,Role::Host,Intent{version:1,revoked,paused,resume:false}).unwrap()});
   let mut channel=pair(TcpStream::connect(addr).unwrap(),Role::Client,"123456").unwrap();let decision=exchange(&mut channel,Role::Client,Intent{version:1,revoked:false,paused:false,resume}).unwrap();assert_eq!(decision,expected);assert_eq!(host.join().unwrap(),expected);
  }
 }
}
