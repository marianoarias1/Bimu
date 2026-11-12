#[derive(Clone,Copy,serde::Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ScreenRect { left:i32, top:i32, right:i32, bottom:i32 }
impl ScreenRect {
    fn intersects(&self,other:&ScreenRect)->bool { self.left<other.right && self.right>other.left && self.top<other.bottom && self.bottom>other.top }
    fn valid(&self)->bool {self.left<self.right && self.top<self.bottom}
}
#[derive(Clone,Copy,Debug,PartialEq)] enum AppKind { Browser, Code, Unknown }
fn app_kind(name:&str)->AppKind {match name.to_ascii_lowercase().as_str(){"chrome.exe"|"msedge.exe"|"brave.exe"|"vivaldi.exe"=>AppKind::Browser,"code.exe"|"code - insiders.exe"=>AppKind::Code,_=>AppKind::Unknown}}
#[tauri::command]
pub async fn window_overlaps_notch(rect:ScreenRect)->bool {
    if !rect.valid(){return false}
    #[cfg(windows)] { return tauri::async_runtime::spawn_blocking(move||windows_impl::overlaps(rect)).await.unwrap_or(false) }
    #[cfg(not(windows))] { let _=rect; false }
}
#[cfg(windows)]
mod windows_impl {
    use super::{ScreenRect,AppKind,app_kind};
    use std::{ffi::c_void,sync::{mpsc::{self,SyncSender},OnceLock},time::Duration};
    use windows::Win32::{Foundation::HWND,System::Com::{CoCreateInstance,CoInitializeEx,CoUninitialize,COINIT_MULTITHREADED,CLSCTX_INPROC_SERVER},UI::Accessibility::{IUIAutomation,IUIAutomationElement,IUIAutomationTreeWalker,CUIAutomation,UIA_TabItemControlTypeId,UIA_ButtonControlTypeId,UIA_EditControlTypeId,UIA_ComboBoxControlTypeId}};
    type Hwnd=*mut c_void;
    #[repr(C)] struct Rect{left:i32,top:i32,right:i32,bottom:i32}
    #[link(name="user32")]
    extern "system" {fn EnumWindows(callback:Option<unsafe extern "system" fn(Hwnd,isize)->i32>,arg:isize)->i32;fn IsWindowVisible(hwnd:Hwnd)->i32;fn IsIconic(hwnd:Hwnd)->i32;fn GetWindowThreadProcessId(hwnd:Hwnd,pid:*mut u32)->u32;fn GetClassNameW(hwnd:Hwnd,buffer:*mut u16,max:i32)->i32;fn GetWindowLongPtrW(hwnd:Hwnd,index:i32)->isize;}
    #[link(name="kernel32")]
    extern "system" {fn OpenProcess(access:u32,inherit:i32,pid:u32)->*mut c_void;fn QueryFullProcessImageNameW(process:*mut c_void,flags:u32,path:*mut u16,size:*mut u32)->i32;fn CloseHandle(handle:*mut c_void)->i32;}
    #[link(name="dwmapi")]
    extern "system" { fn DwmGetWindowAttribute(hwnd:Hwnd,attribute:u32,value:*mut c_void,size:u32)->i32; }
    struct Context { target:ScreenRect,window:usize,pid:u32,own_pid:u32 }
    unsafe extern "system" fn visit(hwnd:Hwnd,arg:isize)->i32 {
        let ctx=&mut *(arg as *mut Context);
        if IsWindowVisible(hwnd)==0 || IsIconic(hwnd)!=0{return 1}
        let mut pid=0;GetWindowThreadProcessId(hwnd,&mut pid);if pid==ctx.own_pid{return 1}
        let style=GetWindowLongPtrW(hwnd,-20) as usize;if style&0x80!=0||style&0x20!=0{return 1}
        let mut class=[0u16;128];let len=GetClassNameW(hwnd,class.as_mut_ptr(),class.len() as i32);let class=String::from_utf16_lossy(&class[..len.max(0) as usize]);
        if matches!(class.as_str(),"Progman"|"WorkerW"|"Shell_TrayWnd"|"Shell_SecondaryTrayWnd"){return 1}
        let mut cloaked=0u32;if DwmGetWindowAttribute(hwnd,14,&mut cloaked as *mut _ as *mut c_void,4)==0&&cloaked!=0{return 1}
        let mut r=Rect{left:0,top:0,right:0,bottom:0};if DwmGetWindowAttribute(hwnd,9,&mut r as *mut _ as *mut c_void,std::mem::size_of::<Rect>() as u32)!=0{return 1}
        if ctx.target.intersects(&ScreenRect{left:r.left,top:r.top,right:r.right,bottom:r.bottom}){ctx.window=hwnd as usize;ctx.pid=pid;return 0}1
    }
    unsafe fn process_kind(pid:u32)->AppKind {let handle=OpenProcess(0x1000,0,pid);if handle.is_null(){return AppKind::Unknown}let mut path=[0u16;1024];let mut len=path.len() as u32;let ok=QueryFullProcessImageNameW(handle,0,path.as_mut_ptr(),&mut len);CloseHandle(handle);if ok==0{return AppKind::Unknown}let path=String::from_utf16_lossy(&path[..len as usize]);app_kind(path.rsplit('\\').next().unwrap_or(""))}
    fn bounds(r:windows::Win32::Foundation::RECT)->ScreenRect {ScreenRect{left:r.left,top:r.top,right:r.right,bottom:r.bottom}}
    unsafe fn chrome_controls(walker:&IUIAutomationTreeWalker,root:&IUIAutomationElement,target:&ScreenRect,depth:u8,budget:&mut usize)->bool {
        if depth>10||*budget==0{return false}let mut child=walker.GetFirstChildElement(root).ok();
        while let Some(element)=child {if *budget==0{return false}*budget-=1;let rect=element.CurrentBoundingRectangle().ok().map(bounds);let intersects=rect.map(|r|r.valid()&&r.intersects(target)).unwrap_or(false);
            if intersects&&element.CurrentIsOffscreen().map(|v|!v.as_bool()).unwrap_or(false){if let Ok(kind)=element.CurrentControlType(){if kind==UIA_TabItemControlTypeId||kind==UIA_ButtonControlTypeId{return true}}}
            if (intersects||rect.map(|r|!r.valid()).unwrap_or(true))&&chrome_controls(walker,&element,target,depth+1,budget){return true}
            child=walker.GetNextSiblingElement(&element).ok();
        }false
    }
    unsafe fn inspect(automation:&IUIAutomation,target:ScreenRect)->bool {
        let mut ctx=Context{target,window:0,pid:0,own_pid:std::process::id()};EnumWindows(Some(visit),&mut ctx as *mut _ as isize);if ctx.window==0{return false}
        let kind=process_kind(ctx.pid);if kind==AppKind::Unknown{return false}
        if let Ok(focus)=automation.GetFocusedElement(){if focus.CurrentProcessId().map(|pid|pid as u32==ctx.pid||process_kind(pid as u32)==kind).unwrap_or(false){if let (Ok(control),Ok(rect),Ok(offscreen))=(focus.CurrentControlType(),focus.CurrentBoundingRectangle(),focus.CurrentIsOffscreen()){if !offscreen.as_bool()&&(control==UIA_EditControlTypeId||control==UIA_ComboBoxControlTypeId)&&target.intersects(&bounds(rect)){return true}}}}
        if kind==AppKind::Browser {if let (Ok(root),Ok(walker))=(automation.ElementFromHandle(HWND(ctx.window as *mut c_void)),automation.ControlViewWalker()){return chrome_controls(&walker,&root,&target,0,&mut 600)}}false
    }
    type Request=(ScreenRect,SyncSender<bool>);
    static WORKER:OnceLock<SyncSender<Request>>=OnceLock::new();
    fn worker()->&'static SyncSender<Request>{WORKER.get_or_init(||{let (tx,rx)=mpsc::sync_channel::<Request>(1);std::thread::spawn(move||unsafe{let init=CoInitializeEx(None,COINIT_MULTITHREADED);if init.is_err(){return}let automation:windows::core::Result<IUIAutomation>=CoCreateInstance(&CUIAutomation,None,CLSCTX_INPROC_SERVER);if let Ok(automation)=automation{while let Ok((rect,reply))=rx.recv(){let _=reply.try_send(inspect(&automation,rect));}}CoUninitialize();});tx})}
    pub fn overlaps(target:ScreenRect)->bool {let (tx,rx)=mpsc::sync_channel(1);if worker().try_send((target,tx)).is_err(){return false}rx.recv_timeout(Duration::from_millis(350)).unwrap_or(false)}
}
#[cfg(test)]mod tests {use super::*;#[test]fn geometry_handles_edges_and_negative_monitors(){let n=ScreenRect{left:-900,top:-100,right:-750,bottom:-73};assert!(n.intersects(&ScreenRect{left:-1000,top:-100,right:-700,bottom:700}));assert!(!n.intersects(&ScreenRect{left:-750,top:-100,right:-600,bottom:700}));}#[test]fn unsupported_apps_stay_unknown(){assert_eq!(app_kind("chrome.exe"),AppKind::Browser);assert_eq!(app_kind("Code.exe"),AppKind::Code);assert_eq!(app_kind("other.exe"),AppKind::Unknown);}}
