//! On-demand Windows shell icons. Only native-resolved registered/discovered
//! targets enter here. No network, child process, startup scan or launch.
use std::{path::PathBuf,sync::{Mutex,OnceLock},collections::VecDeque};
#[derive(Clone,Debug)]pub(crate) enum IconSource{File(PathBuf),Shell(String)}
static CACHE:OnceLock<Mutex<VecDeque<(String,Option<String>)>>>=OnceLock::new();
pub(crate) fn icon(source:IconSource)->Option<String>{
 let key=format!("{source:?}");
 let cache=CACHE.get_or_init(||Mutex::new(VecDeque::new()));
 if let Ok(guard)=cache.lock(){if let Some((_,value))=guard.iter().find(|(k,_)|*k==key){return value.clone()}}
 let result=extract(&source);
 if let Ok(mut guard)=cache.lock(){guard.retain(|(k,_)|*k!=key);guard.push_back((key,result.clone()));while guard.len()>128{guard.pop_front();}}
 result
}
pub(crate) fn spotify_source()->Option<IconSource>{
 #[cfg(windows)]{
  use winreg::{RegKey,enums::HKEY_CLASSES_ROOT};
  let raw:Option<String>=RegKey::predef(HKEY_CLASSES_ROOT).open_subkey("spotify\\DefaultIcon").ok().and_then(|key|key.get_value("").ok());
  if let Some(raw)=raw{
   let raw=raw.trim();let path=if let Some(rest)=raw.strip_prefix('"'){rest.split('"').next().unwrap_or("")}else{raw.rsplit_once(',').filter(|(_,index)|index.trim().parse::<i32>().is_ok()).map(|(p,_)|p).unwrap_or(raw)};
   let path=PathBuf::from(path.trim());if path.is_absolute()&&path.is_file()&&crate::assistant_core::local_path(&path){return Some(IconSource::File(path))}
  }
  let path=std::env::var_os("APPDATA").map(PathBuf::from)?.join("Spotify/Spotify.exe");
  if path.is_file(){return Some(IconSource::File(path))}
 }None
}
#[cfg(not(windows))]fn extract(_: &IconSource)->Option<String>{None}
#[cfg(windows)]fn extract(source:&IconSource)->Option<String>{win::extract(source)}
#[cfg(windows)]mod win{
 use super::*;use std::{ffi::c_void,ptr};use crate::application_icon_pixels::{SIDE,rgba_from_mattes};
 #[repr(C)]struct ShellInfo{icon:isize,index:i32,attributes:u32,name:[u16;260],kind:[u16;80]}
 #[repr(C)]struct Header{size:u32,width:i32,height:i32,planes:u16,bits:u16,compression:u32,image_size:u32,xppm:i32,yppm:i32,used:u32,important:u32}
 #[repr(C)]struct BitmapInfo{header:Header,colors:[u32;1]}
 #[link(name="shell32")]extern "system"{fn SHGetFileInfoW(path:*const u16,attributes:u32,info:*mut ShellInfo,size:u32,flags:u32)->usize;fn SHParseDisplayName(name:*const u16,context:*mut c_void,pidl:*mut *mut c_void,attributes:u32,out:*mut u32)->i32;}
 #[link(name="ole32")]extern "system"{fn CoInitializeEx(reserved:*mut c_void,flags:u32)->i32;fn CoUninitialize();fn CoTaskMemFree(memory:*mut c_void);}
 #[link(name="gdi32")]extern "system"{fn CreateCompatibleDC(dc:isize)->isize;fn DeleteDC(dc:isize)->i32;fn CreateDIBSection(dc:isize,info:*const BitmapInfo,usage:u32,bits:*mut *mut c_void,section:isize,offset:u32)->isize;fn SelectObject(dc:isize,object:isize)->isize;fn DeleteObject(object:isize)->i32;fn GdiFlush()->i32;}
 #[link(name="user32")]extern "system"{fn DrawIconEx(dc:isize,x:i32,y:i32,icon:isize,width:i32,height:i32,step:u32,brush:isize,flags:u32)->i32;fn DestroyIcon(icon:isize)->i32;}
 struct Com(bool);impl Drop for Com{fn drop(&mut self){if self.0{unsafe{CoUninitialize()}}}}
 struct Icon(isize);impl Drop for Icon{fn drop(&mut self){unsafe{DestroyIcon(self.0);}}}
 struct Pidl(*mut c_void);impl Drop for Pidl{fn drop(&mut self){unsafe{CoTaskMemFree(self.0);}}}
 struct Surface{dc:isize,bitmap:isize,old:isize,bits:*mut u8}
 impl Drop for Surface{fn drop(&mut self){unsafe{SelectObject(self.dc,self.old);DeleteObject(self.bitmap);DeleteDC(self.dc);}}}
 impl Surface{
  fn new()->Option<Self>{unsafe{
   let dc=CreateCompatibleDC(0);if dc==0{return None}
   let info=BitmapInfo{header:Header{size:std::mem::size_of::<Header>()as u32,width:SIDE as i32,height:-(SIDE as i32),planes:1,bits:32,compression:0,image_size:(SIDE*SIDE*4)as u32,xppm:0,yppm:0,used:0,important:0},colors:[0]};
   let mut bits=ptr::null_mut();let bitmap=CreateDIBSection(dc,&info,0,&mut bits,0,0);
   if bitmap==0||bits.is_null(){if bitmap!=0{DeleteObject(bitmap);}DeleteDC(dc);return None}
   let old=SelectObject(dc,bitmap);if old==0||old== -1{DeleteObject(bitmap);DeleteDC(dc);return None}
   Some(Self{dc,bitmap,old,bits:bits.cast()})
  }}
  fn matte(&mut self,icon:isize,value:u8)->Option<Vec<u8>>{unsafe{
   GdiFlush();let pixels=std::slice::from_raw_parts_mut(self.bits,SIDE*SIDE*4);pixels.fill(value);for p in pixels.chunks_exact_mut(4){p[3]=255}
   if DrawIconEx(self.dc,0,0,icon,SIDE as i32,SIDE as i32,0,0,3)==0{return None}GdiFlush();
   Some(std::slice::from_raw_parts(self.bits,SIDE*SIDE*4).to_vec())
  }}
 }
 pub(super)fn extract(source:&IconSource)->Option<String>{
  unsafe{
   let initialized=CoInitializeEx(ptr::null_mut(),0);if initialized<0&&initialized!= -2147417850{return None}let _com=Com(initialized>=0);
   let mut info:ShellInfo=std::mem::zeroed();
   let ok=match source{
    IconSource::File(path)=>{if !path.is_absolute()||!crate::assistant_core::local_path(path){return None}let wide:Vec<u16>=path.to_str()?.encode_utf16().chain(Some(0)).collect();SHGetFileInfoW(wide.as_ptr(),0,&mut info,std::mem::size_of::<ShellInfo>()as u32,0x100)},
    IconSource::Shell(id)=>{if !crate::application_registry::valid_app_id(id){return None}let wide:Vec<u16>=format!("shell:AppsFolder\\{id}").encode_utf16().chain(Some(0)).collect();let mut raw=ptr::null_mut();if SHParseDisplayName(wide.as_ptr(),ptr::null_mut(),&mut raw,0,ptr::null_mut())<0{return None}let pidl=Pidl(raw);if pidl.0.is_null(){return None}SHGetFileInfoW(pidl.0.cast(),0,&mut info,std::mem::size_of::<ShellInfo>()as u32,0x100|8)}
   };
   if info.icon==0{return None}let handle=Icon(info.icon);if ok==0{return None}
   let mut surface=Surface::new()?;let black=surface.matte(handle.0,0)?;let white=surface.matte(handle.0,255)?;
   let rgba=rgba_from_mattes(&black,&white)?;let image=image::RgbaImage::from_raw(SIDE as u32,SIDE as u32,rgba)?;
   let mut png=std::io::Cursor::new(Vec::new());image::DynamicImage::ImageRgba8(image).write_to(&mut png,image::ImageFormat::Png).ok()?;
   let png=png.into_inner();if png.len()>16*1024{return None}use base64::{engine::general_purpose::STANDARD,Engine};Some(format!("data:image/png;base64,{}",STANDARD.encode(png)))
  }
 }
}

#[cfg(all(test,windows))]mod tests{
 use super::*;
 #[test]fn extracts_a_real_windows_icon_as_a_small_png(){
  let path=std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(||PathBuf::from("C:\\Windows")).join("explorer.exe");
  let uri=icon(IconSource::File(path)).expect("Windows Explorer icon");
  use base64::{engine::general_purpose::STANDARD,Engine};let bytes=STANDARD.decode(uri.strip_prefix("data:image/png;base64,").unwrap()).unwrap();
  let image=image::load_from_memory(&bytes).unwrap();assert_eq!((image.width(),image.height()),(32,32));assert!(bytes.len()<=16*1024);
 }
}
