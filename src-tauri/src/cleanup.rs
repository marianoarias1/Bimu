use serde::Serialize;

#[derive(Clone, Copy, Default, Debug, Serialize, PartialEq, Eq)]
struct Category {
    bytes: u64,
    files: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Analysis {
    plan_id: u32,
    temporary: Option<Category>,
    recycle: Option<Category>,
    skipped: u64,
    limited: bool,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    removed_bytes: u64,
    removed_files: u64,
    skipped: u64,
    recycle_status: Option<&'static str>,
}

#[derive(Default)]
pub struct Cleanup {
    #[cfg(windows)]
    inner: std::sync::Mutex<desktop::State>,
}

#[tauri::command]
pub async fn cleanup_analyze(app: tauri::AppHandle) -> Result<Analysis, String> {
    #[cfg(windows)]
    return tauri::async_runtime::spawn_blocking(move || desktop::analyze(&app))
        .await
        .map_err(|_| "No se pudo analizar el almacenamiento.")?;
    #[cfg(not(windows))]
    {
        let _ = app;
        Err("Limpieza está disponible en Windows.".into())
    }
}

#[tauri::command]
pub async fn cleanup_run(
    app: tauri::AppHandle,
    plan_id: u32,
    temporary: bool,
    recycle: bool,
) -> Result<Summary, String> {
    #[cfg(windows)]
    return tauri::async_runtime::spawn_blocking(move || {
        desktop::run(&app, plan_id, temporary, recycle)
    })
    .await
    .map_err(|_| "No se pudo completar la limpieza.")?;
    #[cfg(not(windows))]
    {
        let _ = (app, plan_id, temporary, recycle);
        Err("Limpieza está disponible en Windows.".into())
    }
}

#[cfg(windows)]
mod desktop {
    use super::*;
    use std::{
        ffi::OsString,
        fs::{self, File, Metadata, OpenOptions},
        io,
        os::windows::{
            ffi::OsStringExt,
            fs::{MetadataExt, OpenOptionsExt},
            io::AsRawHandle,
        },
        path::{Component, Path, PathBuf, Prefix},
        ptr::{null, null_mut},
        time::{Duration, Instant, SystemTime},
    };
    use tauri::Manager;
    use windows::Win32::System::Com::{
        CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED,
    };
    use windows_sys::Win32::{
        Foundation::HWND,
        Storage::FileSystem::{
            FileDispositionInfo, GetFileInformationByHandle, SetFileInformationByHandle,
            BY_HANDLE_FILE_INFORMATION, DELETE, FILE_ATTRIBUTE_READONLY,
            FILE_ATTRIBUTE_REPARSE_POINT, FILE_DISPOSITION_INFO, FILE_FLAG_BACKUP_SEMANTICS,
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES, FILE_SHARE_READ,
        },
        UI::Shell::{
            FOLDERID_LocalAppData, SHEmptyRecycleBinW, SHGetKnownFolderPath, SHQueryRecycleBinW,
            SHERB_NOPROGRESSUI, SHERB_NOSOUND, SHQUERYRBINFO,
        },
    };

    const MIN_AGE: Duration = Duration::from_secs(48 * 60 * 60);
    const PLAN_AGE: Duration = Duration::from_secs(5 * 60);
    const MAX_ENTRIES: usize = 10_000;
    const MAX_DEPTH: usize = 12;
    const SCAN_TIME: Duration = Duration::from_secs(8);
    const BUSY: &str = "Ya hay una operación de limpieza en curso.";
    const EXPIRED: &str = "Volvé a analizar el almacenamiento antes de limpiar.";

    #[derive(Default)]
    pub(super) struct State {
        next_id: u32,
        plan: Option<Plan>,
    }

    struct Plan {
        analysis: Analysis,
        created: Instant,
        files: Vec<Candidate>,
    }

    struct Candidate {
        path: PathBuf,
        stamp: Stamp,
    }

    #[derive(PartialEq, Eq)]
    struct Stamp {
        volume: u32,
        index: u64,
        created: SystemTime,
        modified: SystemTime,
        size: u64,
    }

    struct Com;
    impl Com {
        fn new() -> Result<Self, String> {
            unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok() }
                .map_err(|_| "No se pudo iniciar el servicio de almacenamiento de Windows.")?;
            Ok(Self)
        }
    }
    impl Drop for Com {
        fn drop(&mut self) {
            unsafe { CoUninitialize() }
        }
    }

    fn temp_root() -> Option<PathBuf> {
        let mut path = null_mut();
        // Use the OS-known profile directory, not a path or environment value
        // supplied by the webview. Custom TEMP directories are outside scope.
        let status =
            unsafe { SHGetKnownFolderPath(&FOLDERID_LocalAppData, 0, null_mut(), &mut path) };
        let result = if status >= 0 && !path.is_null() {
            let len = (0..32768).find(|i| unsafe { *path.add(*i) } == 0);
            len.map(|len| {
                let value = unsafe { std::slice::from_raw_parts(path, len) };
                PathBuf::from(OsString::from_wide(value)).join("Temp")
            })
        } else {
            None
        };
        if !path.is_null() {
            unsafe { CoTaskMemFree(Some(path.cast())) }
        }
        result
    }

    fn reparse(metadata: &Metadata) -> bool {
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }

    // Holding these handles without share-delete or share-write prevents path
    // components from being renamed or turned into junctions during traversal.
    fn open_directory(path: &Path) -> io::Result<File> {
        let file = OpenOptions::new()
            .access_mode(FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
            .open(path)?;
        let info = file.metadata()?;
        if !info.is_dir() || reparse(&info) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        Ok(file)
    }

    fn root_guards(root: &Path) -> io::Result<Vec<File>> {
        let mut components = root.components();
        if !matches!(components.next(), Some(Component::Prefix(p)) if matches!(p.kind(), Prefix::Disk(_)))
            || components.next() != Some(Component::RootDir)
            || !components.all(|part| matches!(part, Component::Normal(_)))
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let mut paths: Vec<_> = root.ancestors().filter(|path| path.has_root()).collect();
        paths.reverse();
        paths.into_iter().map(open_directory).collect()
    }

    fn open_candidate(path: &Path) -> io::Result<File> {
        // No sharing: Windows rejects a file another process has open for
        // read/write/delete, even if that process would allow unlinking it.
        OpenOptions::new()
            .access_mode(FILE_READ_ATTRIBUTES | DELETE)
            .share_mode(0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)
    }

    fn stamp(file: &File, now: SystemTime) -> Option<Stamp> {
        let meta = file.metadata().ok()?;
        if !meta.is_file()
            || reparse(&meta)
            || meta.file_attributes() & FILE_ATTRIBUTE_READONLY != 0
        {
            return None;
        }
        let created = meta.created().ok()?;
        let modified = meta.modified().ok()?;
        if [created, modified]
            .iter()
            .any(|time| !matches!(now.duration_since(*time), Ok(age) if age >= MIN_AGE))
        {
            return None;
        }
        let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0
            || info.nNumberOfLinks != 1
        {
            return None;
        }
        let index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
        if index == 0 {
            return None;
        }
        Some(Stamp {
            volume: info.dwVolumeSerialNumber,
            index,
            created,
            modified,
            size: meta.len(),
        })
    }

    struct Scan {
        files: Vec<Candidate>,
        counts: Category,
        skipped: u64,
        limited: bool,
        visited: usize,
        until: Instant,
        now: SystemTime,
    }

    impl Scan {
        fn new() -> Self {
            Self {
                files: Vec::new(),
                counts: Category::default(),
                skipped: 0,
                limited: false,
                visited: 0,
                until: Instant::now() + SCAN_TIME,
                now: SystemTime::now(),
            }
        }

        fn walk(&mut self, path: &Path, depth: usize) -> io::Result<()> {
            for entry in fs::read_dir(path)? {
                if self.visited >= MAX_ENTRIES || Instant::now() >= self.until {
                    self.limited = true;
                    break;
                }
                self.visited += 1;
                let Ok(entry) = entry else {
                    self.skipped += 1;
                    continue;
                };
                let Ok(meta) = fs::symlink_metadata(entry.path()) else {
                    self.skipped += 1;
                    continue;
                };
                if reparse(&meta) {
                    self.skipped += 1;
                } else if meta.is_dir() {
                    if depth >= MAX_DEPTH {
                        self.limited = true;
                        continue;
                    }
                    match open_directory(&entry.path()) {
                        Ok(_guard) => {
                            if self.walk(&entry.path(), depth + 1).is_err() {
                                self.skipped += 1;
                            }
                        }
                        Err(_) => self.skipped += 1,
                    }
                } else if meta.is_file() {
                    let path = entry.path();
                    match open_candidate(&path)
                        .ok()
                        .and_then(|file| stamp(&file, self.now))
                    {
                        Some(stamp) => {
                            self.counts.bytes = self.counts.bytes.saturating_add(stamp.size);
                            self.counts.files += 1;
                            self.files.push(Candidate { path, stamp });
                        }
                        None => self.skipped += 1,
                    }
                } else {
                    self.skipped += 1;
                }
            }
            Ok(())
        }
    }

    fn remove_candidate(root: &Path, candidate: &Candidate) -> io::Result<()> {
        let relative = candidate
            .path
            .strip_prefix(root)
            .map_err(|_| io::ErrorKind::InvalidInput)?;
        if relative.as_os_str().is_empty()
            || !relative
                .components()
                .all(|c| matches!(c, Component::Normal(_)))
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let mut paths: Vec<_> = candidate
            .path
            .parent()
            .ok_or(io::ErrorKind::InvalidInput)?
            .ancestors()
            .take_while(|path| *path != root)
            .collect();
        paths.reverse();
        let _guards: Vec<_> = paths
            .into_iter()
            .map(open_directory)
            .collect::<io::Result<_>>()?;
        let file = open_candidate(&candidate.path)?;
        if stamp(&file, SystemTime::now()).as_ref() != Some(&candidate.stamp) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let disposition = FILE_DISPOSITION_INFO { DeleteFile: 1 };
        let ok = unsafe {
            SetFileInformationByHandle(
                file.as_raw_handle(),
                FileDispositionInfo,
                (&disposition as *const FILE_DISPOSITION_INFO).cast(),
                std::mem::size_of_val(&disposition) as u32,
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        drop(file);
        Ok(())
    }

    fn query_bin() -> Option<Category> {
        let mut info = SHQUERYRBINFO {
            cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32,
            i64Size: 0,
            i64NumItems: 0,
        };
        let status = unsafe { SHQueryRecycleBinW(null(), &mut info) };
        (status >= 0).then_some(Category {
            bytes: info.i64Size.max(0) as u64,
            files: info.i64NumItems.max(0) as u64,
        })
    }

    pub(super) fn analyze(app: &tauri::AppHandle) -> Result<Analysis, String> {
        let cleanup = app.state::<Cleanup>();
        let mut state = cleanup.inner.try_lock().map_err(|_| BUSY)?;
        let _com = Com::new()?;
        let mut scan = Scan::new();
        let temporary = temp_root().and_then(|root| {
            let _guards = root_guards(&root).ok()?;
            scan.walk(&root, 0).ok()?;
            Some(scan.counts)
        });
        state.next_id = state.next_id.wrapping_add(1);
        let analysis = Analysis {
            plan_id: state.next_id,
            temporary,
            recycle: query_bin(),
            skipped: scan.skipped,
            limited: scan.limited,
        };
        state.plan = Some(Plan {
            analysis: analysis.clone(),
            created: Instant::now(),
            files: scan.files,
        });
        Ok(analysis)
    }

    pub(super) fn run(
        app: &tauri::AppHandle,
        plan_id: u32,
        temporary: bool,
        recycle: bool,
    ) -> Result<Summary, String> {
        let cleanup = app.state::<Cleanup>();
        let mut state = cleanup.inner.try_lock().map_err(|_| BUSY)?;
        let plan = state
            .plan
            .as_ref()
            .filter(|plan| plan.analysis.plan_id == plan_id && plan.created.elapsed() < PLAN_AGE)
            .ok_or(EXPIRED)?;
        if !(temporary || recycle)
            || temporary && !plan.analysis.temporary.is_some_and(|value| value.files > 0)
            || recycle && !plan.analysis.recycle.is_some_and(|value| value.files > 0)
        {
            return Err("Elegí al menos una categoría con archivos para limpiar.".into());
        }
        let _com = Com::new()?;
        // Consume the preview before any destructive action so it cannot be replayed.
        let plan = state.plan.take().ok_or(EXPIRED)?;
        let mut summary = Summary::default();
        if temporary {
            summary.skipped = plan.analysis.skipped;
            let removed = temp_root().and_then(|root| {
                let _guards = root_guards(&root).ok()?;
                for candidate in &plan.files {
                    if remove_candidate(&root, candidate).is_ok() {
                        summary.removed_files += 1;
                        summary.removed_bytes =
                            summary.removed_bytes.saturating_add(candidate.stamp.size);
                    } else {
                        summary.skipped += 1;
                    }
                }
                Some(())
            });
            if removed.is_none() {
                summary.skipped += plan.files.len() as u64;
            }
        }
        if recycle {
            let before = query_bin();
            summary.recycle_status = Some(if before != plan.analysis.recycle {
                "changed"
            } else {
                let owner: HWND = app
                    .get_webview_window("main")
                    .and_then(|window| window.hwnd().ok())
                    .map_or(null_mut(), |hwnd| hwnd.0 as HWND);
                // Keep Windows' native confirmation; do not suppress it.
                let status = unsafe {
                    SHEmptyRecycleBinW(owner, null(), SHERB_NOPROGRESSUI | SHERB_NOSOUND)
                };
                if status >= 0 && query_bin().is_some_and(|value| value.files == 0) {
                    "emptied"
                } else {
                    "notEmptied"
                }
            });
        }
        Ok(summary)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::{fs::FileTimes, os::windows::fs::FileTimesExt};

        #[test]
        fn skips_live_and_recent_files_and_checks_the_analyzed_identity() {
            let root = std::env::temp_dir().join(format!(
                "bimu-cleanup-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir(&root).unwrap();
            let old = SystemTime::now() - MIN_AGE - Duration::from_secs(60);
            let stale_file = |name: &str| {
                let path = root.join(name);
                let file = File::create(&path).unwrap();
                file.set_len(64).unwrap();
                file.set_times(FileTimes::new().set_created(old).set_modified(old))
                    .unwrap();
                path
            };
            let removable = stale_file("old.tmp");
            let replaced = stale_file("replace.tmp");
            let busy = stale_file("busy.tmp");
            let recent = root.join("recent.tmp");
            fs::write(&recent, b"keep").unwrap();
            let held = File::open(&busy).unwrap();
            let mut scan = Scan::new();
            scan.walk(&root, 0).unwrap();
            assert_eq!(
                scan.counts,
                Category {
                    bytes: 128,
                    files: 2
                }
            );
            assert_eq!(scan.skipped, 2);
            fs::rename(&replaced, root.join("old-identity.tmp")).unwrap();
            fs::write(&replaced, b"replacement").unwrap();
            for item in &scan.files {
                assert_eq!(
                    remove_candidate(&root, item).is_ok(),
                    item.path == removable
                );
            }
            assert!(!removable.exists());
            assert_eq!(fs::read(replaced).unwrap(), b"replacement");
            assert!(recent.exists() && busy.exists());
            drop(held);
            fs::remove_dir_all(root).unwrap();
        }
    }
}
