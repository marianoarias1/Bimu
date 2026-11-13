//! Metadata exchanged for on-demand access to a paired device's shelf.
use serde::{Deserialize,Serialize};
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShelfMetadata{#[serde(default,skip_serializing_if="Option::is_none")]pub revision:Option<String>,pub id:String,pub name:String,pub kind:String,pub available:bool}
pub fn valid_id(id:&str)->bool{id.len()==64&&id.bytes().all(|b|b.is_ascii_hexdigit()&&!b.is_ascii_uppercase())}
pub fn validate(items:&[ShelfMetadata])->Result<(),String>{
 if items.len()>100{return Err("Demasiados archivos compartidos".into())}
 let mut ids=std::collections::HashSet::new();
 for item in items{
  if item.revision.as_ref().map(|s|s.len()>64||!s.bytes().all(|b|b.is_ascii_digit()||b==b':')).unwrap_or(false)||!valid_id(&item.id)||!ids.insert(&item.id)||item.name.is_empty()||item.name.len()>255||item.name.chars().any(char::is_control)||!matches!(item.kind.as_str(),"file"|"folder"){
   return Err("Información de archivos compartidos inválida".into())
  }
 }Ok(())
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn catalog_accepts_valid_metadata_and_rejects_forged_or_duplicate_ids(){let a=ShelfMetadata{revision:None,id:"a".repeat(64),name:"informe.pdf".into(),kind:"file".into(),available:true};assert!(validate(&[a.clone()]).is_ok());assert!(validate(&[a.clone(),a.clone()]).is_err());let mut b=a;b.id="../archivo".into();assert!(validate(&[b]).is_err());}
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

pub fn valid_preview(value:&str)->bool{
 use base64::{engine::general_purpose::STANDARD,Engine};
 let Some(encoded)=value.strip_prefix("data:image/png;base64,")else{return false};if value.len()>64*1024{return false}
 let Ok(bytes)=STANDARD.decode(encoded)else{return false};if bytes.len()<33||&bytes[..8]!=b"\x89PNG\r\n\x1a\n"||&bytes[12..16]!=b"IHDR"{return false}
 let width=u32::from_be_bytes(bytes[16..20].try_into().unwrap());let height=u32::from_be_bytes(bytes[20..24].try_into().unwrap());
 if width==0||height==0||width>160||height>120||bytes[24]!=8||!matches!(bytes[25],2|6)||bytes[26..29]!=[0,0,0]{return false}
 let mut at=8;let mut pixels=false;
 while at+12<=bytes.len(){let length=u32::from_be_bytes(bytes[at..at+4].try_into().unwrap())as usize;let Some(end)=at.checked_add(length).and_then(|n|n.checked_add(12))else{return false};if end>bytes.len(){return false}
  match &bytes[at+4..at+8]{b"IHDR" if at==8&&length==13=>{},b"IDAT"=>pixels=true,b"IEND" if length==0=>return pixels&&end==bytes.len(),b"sRGB" if length==1=>{},b"gAMA" if length==4=>{},b"pHYs" if length==9=>{},_=>return false}at=end;
 }false
}

/// Older peers reject unknown metadata fields, so optional revisions are v6-only.
pub fn for_peer(mut items:Vec<ShelfMetadata>,capable:bool)->Vec<ShelfMetadata>{if !capable{for item in &mut items{item.revision=None}}items}
#[cfg(test)]mod preview_compatibility_tests{
 use super::*;use base64::{engine::general_purpose::STANDARD,Engine};
 #[derive(Deserialize)]#[serde(deny_unknown_fields)]struct Legacy{ id:String,name:String,kind:String,available:bool }
 #[test]fn legacy_catalogs_keep_the_original_schema(){
  let item=ShelfMetadata{revision:Some("123:456".into()),id:"a".repeat(64),name:"foto.png".into(),kind:"file".into(),available:true};
  let bytes=serde_json::to_vec(&for_peer(vec![item.clone()],false)).unwrap();let legacy:Vec<Legacy>=serde_json::from_slice(&bytes).unwrap();assert_eq!(legacy[0].id,item.id);assert_eq!(legacy[0].name,item.name);assert_eq!(legacy[0].kind,item.kind);assert!(legacy[0].available);
  assert_eq!(for_peer(vec![item],true)[0].revision.as_deref(),Some("123:456"));
 }
 #[test]fn previews_reject_unbounded_dimensions_and_metadata(){
  let png="iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAAEUlEQVR4nGPo2RfwH4QZYAwAW8AKZWZiO2wAAAAASUVORK5CYII=";
  let value=format!("data:image/png;base64,{png}");assert!(valid_preview(&value));
  let mut bytes=STANDARD.decode(png).unwrap();bytes[16..20].copy_from_slice(&10000u32.to_be_bytes());assert!(!valid_preview(&format!("data:image/png;base64,{}",STANDARD.encode(&bytes))));
  assert!(!valid_preview(&format!("data:image/svg+xml;base64,{png}")));assert!(!valid_preview(&(value+&"A".repeat(65536))));
  let mut bytes=STANDARD.decode(png).unwrap();bytes[37..41].copy_from_slice(b"iCCP");assert!(!valid_preview(&format!("data:image/png;base64,{}",STANDARD.encode(bytes))));
 }
}
