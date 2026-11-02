//! Metadata exchanged for on-demand access to a paired device's shelf.
use serde::{Deserialize,Serialize};
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShelfMetadata{pub id:String,pub name:String,pub kind:String,pub available:bool}
pub fn valid_id(id:&str)->bool{id.len()==64&&id.bytes().all(|b|b.is_ascii_hexdigit()&&!b.is_ascii_uppercase())}
pub fn validate(items:&[ShelfMetadata])->Result<(),String>{
 if items.len()>100{return Err("Demasiados archivos compartidos".into())}
 let mut ids=std::collections::HashSet::new();
 for item in items{
  if !valid_id(&item.id)||!ids.insert(&item.id)||item.name.is_empty()||item.name.len()>255||item.name.chars().any(char::is_control)||!matches!(item.kind.as_str(),"file"|"folder"){
   return Err("Información de archivos compartidos inválida".into())
  }
 }Ok(())
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn catalog_accepts_valid_metadata_and_rejects_forged_or_duplicate_ids(){let a=ShelfMetadata{id:"a".repeat(64),name:"informe.pdf".into(),kind:"file".into(),available:true};assert!(validate(&[a.clone()]).is_ok());assert!(validate(&[a.clone(),a.clone()]).is_err());let mut b=a;b.id="../archivo".into();assert!(validate(&[b]).is_err());}
}

/// A small metadata heartbeat cannot be starved by clipboard traffic.
pub fn catalog_due(first:bool,dirty:bool,elapsed:std::time::Duration)->bool{first||dirty||elapsed>=std::time::Duration::from_secs(3)}
#[cfg(test)]mod publication_tests{
 use super::*;use std::time::Duration;
 #[test]fn additions_publish_immediately_and_steady_traffic_cannot_starve_catalogs(){
  assert!(catalog_due(true,false,Duration::ZERO));assert!(catalog_due(false,true,Duration::ZERO));
  assert!(!catalog_due(false,false,Duration::from_secs(2)));
  assert!(catalog_due(false,false,Duration::from_secs(3)));
 }
}
