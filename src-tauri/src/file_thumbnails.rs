//! Bounded, lazy previews. Callers resolve saved IDs before entering this module.
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};
static CACHE: OnceLock<Mutex<VecDeque<(PathBuf, u64, u128, Option<String>)>>> = OnceLock::new();
use crate::shelf_shared::valid_preview;
pub fn file(path: &Path) -> Option<String> {
    if !path.is_absolute() || !crate::assistant_core::local_path(path) {
        return None;
    }
    let meta = path.metadata().ok()?;
    if !meta.is_file() {
        return None;
    }
    let stamp = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos();
    let cache = CACHE.get_or_init(|| Mutex::new(VecDeque::new()));
    if let Ok(c) = cache.lock() {
        if let Some((_, _, _, value)) = c
            .iter()
            .find(|(p, n, t, _)| p == path && *n == meta.len() && *t == stamp)
        {
            return value.clone();
        }
    }
    let shell_path = crate::assistant_core::shell_folder_path(path.to_owned());
    let value = extract(&shell_path).filter(|s| valid_preview(s));
    if let (Some(_), Ok(mut c)) = (&value, cache.lock()) {
        c.retain(|(p, _, _, _)| p != path);
        c.push_back((path.to_owned(), meta.len(), stamp, value.clone()));
        while c.len() > 128 {
            c.pop_front();
        }
    }
    value
}
#[cfg(not(windows))]
fn extract(_: &Path) -> Option<String> {
    None
}
#[cfg(windows)]
fn extract(path: &Path) -> Option<String> {
    use std::ffi::c_void;
    use windows::{
        core::PCWSTR,
        Win32::{
            Foundation::SIZE,
            System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED},
            UI::Shell::{
                IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_THUMBNAILONLY,
            },
        },
    };
    #[repr(C)]
    struct Bitmap {
        kind: i32,
        width: i32,
        height: i32,
        stride: i32,
        planes: u16,
        bits: u16,
        data: *mut c_void,
    }
    #[repr(C)]
    struct Header {
        size: u32,
        width: i32,
        height: i32,
        planes: u16,
        bits: u16,
        compression: u32,
        image_size: u32,
        xppm: i32,
        yppm: i32,
        used: u32,
        important: u32,
    }
    #[repr(C)]
    struct Info {
        header: Header,
        colors: [u32; 1],
    }
    #[link(name = "gdi32")]
    extern "system" {
        fn GetObjectW(object: isize, size: i32, value: *mut c_void) -> i32;
        fn CreateCompatibleDC(dc: isize) -> isize;
        fn DeleteDC(dc: isize) -> i32;
        fn DeleteObject(object: isize) -> i32;
        fn GetDIBits(
            dc: isize,
            bitmap: isize,
            start: u32,
            lines: u32,
            data: *mut c_void,
            info: *mut Info,
            usage: u32,
        ) -> i32;
    }
    struct Com;
    impl Drop for Com {
        fn drop(&mut self) {
            unsafe { CoUninitialize() }
        }
    }
    struct Handle(isize);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                DeleteObject(self.0);
            }
        }
    }
    struct Dc(isize);
    impl Drop for Dc {
        fn drop(&mut self) {
            unsafe {
                DeleteDC(self.0);
            }
        }
    }
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok().ok()?;
        let _com = Com;
        let wide: Vec<u16> = path
            .as_os_str()
            .to_string_lossy()
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let factory: IShellItemImageFactory =
            SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None).ok()?;
        let bitmap = Handle(
            factory
                .GetImage(SIZE { cx: 128, cy: 88 }, SIIGBF_THUMBNAILONLY)
                .ok()?
                .0 as isize,
        );
        let mut meta: Bitmap = std::mem::zeroed();
        if GetObjectW(
            bitmap.0,
            std::mem::size_of::<Bitmap>() as i32,
            (&mut meta as *mut Bitmap).cast(),
        ) == 0
            || meta.width <= 0
            || meta.height <= 0
            || meta.width > 160
            || meta.height > 120
        {
            return None;
        }
        let dc = Dc(CreateCompatibleDC(0));
        if dc.0 == 0 {
            return None;
        }
        let mut info = Info {
            header: Header {
                size: 40,
                width: meta.width,
                height: -meta.height,
                planes: 1,
                bits: 32,
                compression: 0,
                image_size: 0,
                xppm: 0,
                yppm: 0,
                used: 0,
                important: 0,
            },
            colors: [0],
        };
        let mut bytes = vec![0u8; meta.width as usize * meta.height as usize * 4];
        if GetDIBits(
            dc.0,
            bitmap.0,
            0,
            meta.height as u32,
            bytes.as_mut_ptr().cast(),
            &mut info,
            0,
        ) != meta.height
        {
            return None;
        }
        let bytes =
            crate::thumbnail_pixels::rgba(meta.width as usize, meta.height as usize, &bytes)?;
        let image = image::RgbaImage::from_raw(meta.width as u32, meta.height as u32, bytes)?;
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut out, image::ImageFormat::Png)
            .ok()?;
        use base64::{engine::general_purpose::STANDARD, Engine};
        Some(format!(
            "data:image/png;base64,{}",
            STANDARD.encode(out.into_inner())
        ))
    }
}

static WORKERS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);
pub async fn prepare(path: PathBuf) -> Option<String> {
    let permit = WORKERS.acquire().await.ok()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        file(&path)
    })
    .await
    .ok()
    .flatten()
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn windows_generates_a_bounded_png_thumbnail() {
        let path = std::env::temp_dir().join(format!("bimu-preview-{}.png", std::process::id()));
        let image = image::RgbaImage::from_pixel(320, 200, image::Rgba([100, 170, 200, 255]));
        image.save(&path).unwrap();
        let canonical = path.canonicalize().unwrap();
        let preview = file(&canonical).expect("Windows PNG thumbnail provider");
        assert!(valid_preview(&preview));
        std::fs::remove_file(path).unwrap();
    }
}
