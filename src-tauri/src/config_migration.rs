use serde::{de::DeserializeOwned, Serialize};
use serde_json::{Map, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

pub const CURRENT_SCHEMA_VERSION: u64 = 2;

// First reads can migrate too. Serialize them with normal writes so callers
// never race on the temporary file or replace a newer saved configuration.
static CONFIG_IO: Mutex<()> = Mutex::new(());

pub struct Decoded<T> {
    pub items: Vec<T>,
    pub needs_migration: bool,
}

/// Reads both the original bare arrays and the current versioned container.
pub fn decode_items<T: DeserializeOwned>(bytes: &[u8], key: &str) -> Result<Decoded<T>, String> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| "El archivo JSON está dañado.".to_string())?;
    match value {
        Value::Array(_) => Ok(Decoded {
            items: serde_json::from_value(value)
                .map_err(|_| "Los datos guardados usan un formato incompatible.".to_string())?,
            needs_migration: true,
        }),
        Value::Object(mut object) => {
            let version = object
                .remove("schemaVersion")
                .and_then(|v| v.as_u64())
                .ok_or("Falta la versión del formato guardado.")?;
            if version > CURRENT_SCHEMA_VERSION {
                return Err(
                    "La configuración fue creada por una versión más nueva de Bimu.".into(),
                );
            }
            let items = object
                .remove(key)
                .ok_or_else(|| format!("Falta la lista «{key}» en la configuración."))?;
            Ok(Decoded {
                items: serde_json::from_value(items)
                    .map_err(|_| "Los datos guardados usan un formato incompatible.".to_string())?,
                needs_migration: version < CURRENT_SCHEMA_VERSION,
            })
        }
        _ => Err("La configuración no contiene una lista válida.".into()),
    }
}

pub fn encode_items<T: Serialize>(items: &[T], key: &str) -> Result<Vec<u8>, String> {
    let mut object = Map::new();
    object.insert("schemaVersion".into(), Value::from(CURRENT_SCHEMA_VERSION));
    object.insert(
        key.into(),
        serde_json::to_value(items)
            .map_err(|_| "No se pudo preparar la configuración.".to_string())?,
    );
    serde_json::to_vec_pretty(&Value::Object(object))
        .map_err(|_| "No se pudo preparar la configuración.".into())
}

fn backup_path(path: &Path) -> Result<PathBuf, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "El reloj del sistema no es válido.")?
        .as_secs();
    let name = path
        .file_name()
        .ok_or("La ruta de configuración es inválida.")?
        .to_string_lossy();
    Ok(path.with_file_name(format!("{name}.pre-schema-{stamp}.bak")))
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let _guard = CONFIG_IO
        .lock()
        .map_err(|_| "La configuración está ocupada.")?;
    write_atomic_locked(path, bytes)
}

fn write_atomic_locked(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or("La ruta de configuración es inválida.")?;
    fs::create_dir_all(parent).map_err(|_| "No se pudo crear la carpeta de configuración.")?;
    let temp = path.with_extension("tmp");
    fs::write(&temp, bytes).map_err(|_| "No se pudo escribir la configuración temporal.")?;
    fs::rename(&temp, path).map_err(|_| "No se pudo actualizar la configuración.".to_string())
}

/// Creates a recoverable copy and upgrades a legacy file only after it decoded successfully.
pub fn migrate_file<T: DeserializeOwned + Serialize>(
    path: &Path,
    max_bytes: u64,
    key: &str,
) -> Result<Vec<T>, String> {
    let _guard = CONFIG_IO
        .lock()
        .map_err(|_| "La configuración está ocupada.")?;
    if !path.exists() {
        return Ok(vec![]);
    }
    let metadata = fs::metadata(path).map_err(|_| "No se pudo consultar la configuración.")?;
    if metadata.len() > max_bytes {
        return Err("La configuración guardada es demasiado grande.".into());
    }
    let bytes = fs::read(path).map_err(|_| "No se pudo leer la configuración.")?;
    let decoded = decode_items(&bytes, key)?;
    if decoded.needs_migration {
        fs::write(backup_path(path)?, &bytes)
            .map_err(|_| "No se pudo crear la copia previa a la migración.")?;
        write_atomic_locked(path, &encode_items(&decoded.items, key)?)?;
    }
    Ok(decoded.items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_legacy_arrays_and_current_containers() {
        let legacy = decode_items::<String>(br#"["uno","dos"]"#, "items").unwrap();
        assert!(legacy.needs_migration);
        assert_eq!(legacy.items, vec!["uno", "dos"]);
        let encoded = encode_items(&legacy.items, "items").unwrap();
        let current = decode_items::<String>(&encoded, "items").unwrap();
        assert!(!current.needs_migration);
        assert_eq!(current.items, legacy.items);
    }

    #[test]
    fn refuses_future_and_malformed_schemas() {
        assert!(decode_items::<String>(br#"{"schemaVersion":99,"items":[]}"#, "items").is_err());
        assert!(decode_items::<String>(br#"{"items":[]}"#, "items").is_err());
        assert!(decode_items::<String>(br#"{"schemaVersion":2,"other":[]}"#, "items").is_err());
    }

    #[test]
    fn migrates_a_legacy_file_and_keeps_a_backup() {
        let root =
            std::env::temp_dir().join(format!("bimu-config-migration-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let path = root.join("items.json");
        fs::write(&path, br#"["uno","dos"]"#).unwrap();
        let items = migrate_file::<String>(&path, 1024, "items").unwrap();
        assert_eq!(items, vec!["uno", "dos"]);
        let current = fs::read(&path).unwrap();
        assert!(
            !decode_items::<String>(&current, "items")
                .unwrap()
                .needs_migration
        );
        assert_eq!(
            fs::read_dir(&root)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry.file_name().to_string_lossy().ends_with(".bak"))
                .count(),
            1
        );
        fs::remove_dir_all(root).unwrap();
    }
}
