//! File-transfer policy. Bounds never reserve buffers; the saved local choice
//! remains unchanged when the original code-generating PC overrides a session.
use std::sync::atomic::{AtomicBool,AtomicUsize,Ordering};
pub const MIB:usize=1024*1024;
pub const DEFAULT_MB:usize=30;
pub const MAX_MB:usize=250;
static LOCAL_MB:AtomicUsize=AtomicUsize::new(DEFAULT_MB);
static CONFIGURED:AtomicBool=AtomicBool::new(false);
pub fn validate(mb:usize)->Result<usize,String>{if (1..=MAX_MB).contains(&mb){Ok(mb)}else{Err(format!("Elegí un límite entre 1 y {MAX_MB} MB"))}}
pub fn current_mb()->usize{LOCAL_MB.load(Ordering::Relaxed)}
pub fn configured()->bool{CONFIGURED.load(Ordering::Relaxed)}
pub fn set_mb(mb:usize)->Result<(),String>{LOCAL_MB.store(validate(mb)?,Ordering::Relaxed);CONFIGURED.store(true,Ordering::Relaxed);Ok(())}
pub fn clear(){LOCAL_MB.store(DEFAULT_MB,Ordering::Relaxed);CONFIGURED.store(false,Ordering::Relaxed);}
pub fn local_bytes()->usize{current_mb()*MIB}
/// Send and receive bounds agree across both endpoints. Without an explicit main
/// choice, each origin uses its own saved limit for the files it sends.
pub fn session_mb(local:usize,peer:usize,main:bool,local_configured:bool,peer_configured:bool)->(usize,usize){
 let override_mb=if main&&local_configured{Some(local)}else if !main&&peer_configured{Some(peer)}else{None};
 override_mb.map(|mb|(mb,mb)).unwrap_or((local,peer))
}
pub const fn wire_bytes(file_limit:usize)->usize{((file_limit+2)/3)*4+1024*1024}
pub fn hello(name:&str,mb:usize,configured:bool)->Result<String,String>{validate(mb)?;if name.is_empty()||name.len()>256||name.chars().any(char::is_control){return Err("Nombre de equipo inválido".into())}Ok(format!("bimu-sync-v5\n{mb}\n{}\n{name}",if configured{1}else{0}))}
pub fn parse_hello(value:&str)->Result<(String,usize,bool),String>{
 let mut parts=value.splitn(4,'\n');if parts.next()!=Some("bimu-sync-v5"){return Err("Actualizá Bimu en ambas PCs para usar esta conexión".into())}
 let mb=parts.next().and_then(|s|s.parse().ok()).ok_or("Límite remoto inválido")?;validate(mb)?;
 let configured=match parts.next(){Some("1")=>true,Some("0")=>false,_=>return Err("Configuración remota inválida".into())};
 let name=parts.next().ok_or("Falta el nombre del equipo")?;
 if name.is_empty()||name.len()>256||name.chars().any(char::is_control){return Err("Nombre de equipo inválido".into())}Ok((name.into(),mb,configured))
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn policy_and_negotiation_are_bounded(){assert_eq!(validate(250).unwrap(),250);for n in [0,251,usize::MAX]{assert!(validate(n).is_err())}assert!(wire_bytes(250*MIB)>333*MIB);}
 #[test]fn main_override_and_fallback_match_in_both_directions(){
  for host in [1,30,250]{for client in [1,80,250]{for hset in [false,true]{for cset in [false,true]{
   let a=session_mb(host,client,true,hset,cset);let b=session_mb(client,host,false,cset,hset);
   assert_eq!(a.0,b.1);assert_eq!(a.1,b.0);
   assert_eq!(a,if hset{(host,host)}else{(host,client)});
  }}}}
 }
 #[test]fn disconnect_and_reconnect_never_replace_saved_choices(){
  let saved=(200,10);assert_eq!(session_mb(saved.0,saved.1,true,true,true),(200,200));
  // Clearing the main override immediately restores each origin's setting.
  assert_eq!(session_mb(30,saved.1,true,false,true),(30,10));
  assert_eq!(saved,(200,10));assert_eq!(session_mb(saved.1,saved.0,false,true,true),(200,200));
 }
 #[test]fn hello_validates_version_and_remote_limits(){let h=hello("Desktop",250,true).unwrap();assert_eq!(parse_hello(&h).unwrap(),("Desktop".into(),250,true));assert_eq!(parse_hello(&hello("Laptop",30,false).unwrap()).unwrap(),("Laptop".into(),30,false));for h in ["bimu-sync-v4\n30\nDesktop","bimu-sync-v5\n251\n1\nPC","bimu-sync-v5\n30\n2\nPC","bimu-sync-v5\n30\n0\n","bimu-sync-v5\n30\n1\nPC\nInjected"]{assert!(parse_hello(h).is_err());}}
}
