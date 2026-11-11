use serde::Serialize;

#[derive(Serialize)]
pub struct Volume {
    level: u8,
    muted: bool,
}

#[cfg(windows)]
fn endpoint(
    change: impl FnOnce(
        &windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume,
    ) -> windows::core::Result<()>,
) -> Result<Volume, String> {
    use windows::Win32::{
        Media::Audio::{
            eMultimedia, eRender, Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator,
            MMDeviceEnumerator,
        },
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, CLSCTX_INPROC_SERVER,
            COINIT_MULTITHREADED,
        },
    };
    struct Com;
    impl Drop for Com {
        fn drop(&mut self) {
            unsafe { CoUninitialize() }
        }
    }
    let result = (|| -> windows::core::Result<Volume> {
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok()? };
        let _com = Com;
        let enumerator: IMMDeviceEnumerator =
            unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER)? };
        let device = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)? };
        let volume: IAudioEndpointVolume = unsafe { device.Activate(CLSCTX_ALL, None)? };
        change(&volume)?;
        Ok(Volume {
            level: (unsafe { volume.GetMasterVolumeLevelScalar()? }.clamp(0., 1.) * 100.).round()
                as u8,
            muted: unsafe { volume.GetMute()? }.as_bool(),
        })
    })();
    result.map_err(|_| "No se pudo acceder al volumen de la salida de Windows.".into())
}

#[tauri::command]
pub async fn system_volume() -> Result<Volume, String> {
    #[cfg(windows)]
    return tauri::async_runtime::spawn_blocking(|| endpoint(|_| Ok(())))
        .await
        .map_err(|_| "No se pudo consultar el volumen.")?;
    #[cfg(not(windows))]
    Err("El volumen de Windows no está disponible.".into())
}

#[tauri::command]
pub async fn system_volume_set(level: Option<u8>, muted: Option<bool>) -> Result<Volume, String> {
    if !matches!((level, muted), (Some(0..=100), None) | (None, Some(_))) {
        return Err("Control de volumen inválido.".into());
    }
    #[cfg(windows)]
    return tauri::async_runtime::spawn_blocking(move || {
        endpoint(|volume| unsafe {
            if let Some(level) = level {
                volume.SetMasterVolumeLevelScalar(f32::from(level) / 100., std::ptr::null())?;
            }
            if let Some(muted) = muted {
                volume.SetMute(muted, std::ptr::null())?;
            }
            Ok(())
        })
    })
    .await
    .map_err(|_| "No se pudo ajustar el volumen.")?;
    #[cfg(not(windows))]
    Err("El volumen de Windows no está disponible.".into())
}
