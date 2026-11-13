//! A transparent companion has no non-client area, even when WebView2 or
//! Windows requests a frame refresh during focus, movement or DPI changes.
use tauri::{AppHandle,Manager};
pub fn install(app:&AppHandle)->Result<(),String>{
    #[cfg(windows)]{
        let window=app.get_webview_window("main").ok_or("No se encontró la ventana principal")?;
        let hwnd=window.hwnd().map_err(|_|"No se pudo acceder a la ventana principal")?.0 as isize;
        win::install(hwnd)
    }
    #[cfg(not(windows))]{let _=app;Ok(())}
}
#[cfg(windows)]mod win{
    type Proc=unsafe extern "system" fn(isize,u32,usize,isize,usize,usize)->isize;
    const ID:usize=0x42494D55;
    #[link(name="comctl32")]extern "system"{
        fn SetWindowSubclass(hwnd:isize,callback:Option<Proc>,id:usize,data:usize)->i32;
        fn RemoveWindowSubclass(hwnd:isize,callback:Option<Proc>,id:usize)->i32;
        fn DefSubclassProc(hwnd:isize,message:u32,wparam:usize,lparam:isize)->isize;
    }
    #[link(name="user32")]extern "system"{
        fn GetWindowLongPtrW(hwnd:isize,index:i32)->isize;
        fn SetWindowLongPtrW(hwnd:isize,index:i32,value:isize)->isize;
        fn SetWindowPos(hwnd:isize,after:isize,x:i32,y:i32,width:i32,height:i32,flags:u32)->i32;
    }
    unsafe extern "system" fn frame(hwnd:isize,message:u32,wparam:usize,lparam:isize,_id:usize,_data:usize)->isize{
        match message{
            0x0083=>0, // WM_NCCALCSIZE: use the entire window as the client area.
            0x0085=>0, // WM_NCPAINT: no native caption or border to paint.
            0x0086=>1, // WM_NCACTIVATE: retain activation without native painting.
            0x0082=>{RemoveWindowSubclass(hwnd,Some(frame),ID);DefSubclassProc(hwnd,message,wparam,lparam)},
            _=>DefSubclassProc(hwnd,message,wparam,lparam)
        }
    }
    pub fn install(hwnd:isize)->Result<(),String>{unsafe{
        if SetWindowSubclass(hwnd,Some(frame),ID,0)==0{return Err("No se pudo configurar la ventana sin marco".into())}
        // Remove caption, sizing frame, system menu and min/max buttons. Do not
        // change extended transparency/click-through flags owned by Tao.
        let style=GetWindowLongPtrW(hwnd,-16);
        SetWindowLongPtrW(hwnd,-16,style&!(0x00C00000|0x00040000|0x00080000|0x00020000|0x00010000));
        // SWP_FRAMECHANGED | NOSIZE | NOMOVE | NOZORDER | NOACTIVATE
        if SetWindowPos(hwnd,0,0,0,0,0,0x0020|0x0001|0x0002|0x0004|0x0010)==0{return Err("No se pudo actualizar el área de Bimu".into())}
        Ok(())
    }}
}
