//! File-transfer policy. These numbers are bounds, never buffer reservations.
use std::sync::atomic::{AtomicUsize,Ordering};
pub const MIB:usize=1024*1024;
pub const DEFAULT_MB:usize=30;
pub const MAX_MB:usize=100;
static LOCAL_MB:AtomicUsize=AtomicUsize::new(DEFAULT_MB);
pub fn validate(mb:usize)->Result<usize,String>{if (1..=MAX_MB).contains(&mb){Ok(mb)}else{Err(format!("Elegí un límite entre 1 y {MAX_MB} MB"))}}
pub fn current_mb()->usize{LOCAL_MB.load(Ordering::Relaxed)}
pub fn set_mb(mb:usize)->Result<(),String>{LOCAL_MB.store(validate(mb)?,Ordering::Relaxed);Ok(())}
pub fn local_bytes()->usize{current_mb()*MIB}
pub fn effective_mb(local:usize,peer:usize)->Result<usize,String>{Ok(validate(local)?.min(validate(peer)?))}
pub fn wire_bytes(file_limit:usize)->usize{((file_limit+2)/3)*4+1024*1024}
pub fn hello(name:&str,mb:usize)->Result<String,String>{validate(mb)?;if name.is_empty()||name.len()>256||name.chars().any(char::is_control){return Err("Nombre de equipo inválido".into())}Ok(format!("bimu-sync-v4\n{mb}\n{name}"))}
pub fn parse_hello(value:&str)->Result<(String,usize),String>{
 let mut parts=value.splitn(3,'\n');if parts.next()!=Some("bimu-sync-v4"){return Err("Actualizá Bimu en ambas PCs para usar esta conexión".into())}
 let mb=parts.next().and_then(|s|s.parse().ok()).ok_or("Límite remoto inválido")?;validate(mb)?;
 let name=parts.next().ok_or("Falta el nombre del equipo")?;
 if name.is_empty()||name.len()>256||name.chars().any(char::is_control){return Err("Nombre de equipo inválido".into())}Ok((name.into(),mb))
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn policy_and_negotiation_are_bounded(){assert_eq!(effective_mb(30,10).unwrap(),10);assert_eq!(effective_mb(100,30).unwrap(),30);for n in [0,101,usize::MAX]{assert!(validate(n).is_err())}assert!(wire_bytes(30*MIB)>40*MIB);}
 #[test]fn hello_validates_version_and_remote_limits(){let h=hello("Desktop",30).unwrap();assert_eq!(parse_hello(&h).unwrap(),("Desktop".into(),30));for h in ["bimu-sync-v3\nDesktop","bimu-sync-v4\n999\nPC","bimu-sync-v4\n30\n","bimu-sync-v4\n30\nPC\nInjected"]{assert!(parse_hello(h).is_err());}}
}
