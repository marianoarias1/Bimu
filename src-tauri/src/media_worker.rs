//! All SMTC objects stay on one explicitly initialized WinRT apartment.
//! No COM call, lock or provider wait runs on the window's message thread.
#[cfg(windows)]
mod win {
    use std::{future::Future, sync::{mpsc::{self,SyncSender},OnceLock}, time::Duration};
    type Job=Box<dyn FnOnce(Result<(),String>)+Send>;
    static SENDER:OnceLock<Result<SyncSender<Job>,String>>=OnceLock::new();
    #[link(name="runtimeobject")]
    extern "system" {fn RoInitialize(kind:u32)->i32;fn RoUninitialize();}
    struct Apartment;
    impl Apartment {
        fn new()->Result<Self,String>{
            let result=unsafe{RoInitialize(1)}; // RO_INIT_MULTITHREADED
            if result<0{return Err(format!("No se pudo iniciar WinRT multimedia (0x{:08X}).",result as u32))}Ok(Self)
        }
    }
    impl Drop for Apartment{fn drop(&mut self){unsafe{RoUninitialize()}}}
    fn sender()->Result<&'static SyncSender<Job>,String>{
        SENDER.get_or_init(||{
            let (send,receive)=mpsc::sync_channel::<Job>(4);
            std::thread::Builder::new().name("bimu-media-winrt".into()).spawn(move||{
                let apartment=Apartment::new();
                while let Ok(job)=receive.recv(){job(apartment.as_ref().map(|_|()).map_err(Clone::clone))}
            }).map_err(|_|"No se pudo iniciar el hilo multimedia.".to_owned())?;
            Ok(send)
        }).as_ref().map_err(Clone::clone)
    }
    pub async fn run<T,F,Task>(task:Task)->Result<T,String>
    where T:Send+'static,F:Future<Output=Result<T,String>>+'static,Task:FnOnce()->F+Send+'static{
        let (send,receive)=tokio::sync::oneshot::channel();
        sender()?.try_send(Box::new(move|initialized|{
            let result=initialized.and_then(|_|tauri::async_runtime::block_on(async{
                tokio::time::timeout(Duration::from_secs(4),task()).await.map_err(|_|"El servicio multimedia tardó demasiado.".to_owned())?
            }));
            let _=send.send(result);
        })).map_err(|_|"El servicio multimedia está ocupado.".to_owned())?;
        tokio::time::timeout(Duration::from_secs(6),receive).await.map_err(|_|"La consulta multimedia venció.".to_owned())?.map_err(|_|"El hilo multimedia se cerró.".to_owned())?
    }
}
#[cfg(windows)]pub use win::run;
