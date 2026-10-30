use bimu_native_core_tests::config_migration::*;
use serde_json::{json, Value};
use std::{fs, path::PathBuf};
fn root(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("bimu-review-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}
#[test]
fn migration_is_idempotent_and_preserves_items() {
    let p = root("roundtrip");
    let f = p.join("applications-v1.json");
    let original = json!([{"id":"x","name":"Editor","aliases":["code"],"remember":true,"target":{"kind":"Exe","path":"C:\\Editor.exe"}}]);
    let bytes = serde_json::to_vec(&original).unwrap();
    fs::write(&f, &bytes).unwrap();
    let first = migrate_file::<Value>(&f, 4194304, "applications").unwrap();
    let after = fs::read(&f).unwrap();
    let second = migrate_file::<Value>(&f, 4194304, "applications").unwrap();
    assert_eq!(first, second);
    assert_eq!(first, original.as_array().unwrap().clone());
    assert_eq!(after, fs::read(&f).unwrap());
    let bak: Vec<_> = fs::read_dir(&p)
        .unwrap()
        .flatten()
        .filter(|x| x.path().extension().is_some_and(|s| s == "bak"))
        .collect();
    assert_eq!(bak.len(), 1);
    assert_eq!(fs::read(bak[0].path()).unwrap(), bytes);
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn bad_and_future_files_remain_untouched() {
    for (i, bytes) in [
        b"not json".as_slice(),
        br#"{"schemaVersion":99,"shortcuts":[]}"#,
    ]
    .iter()
    .enumerate()
    {
        let p = root(&format!("bad-{i}"));
        let f = p.join("shortcuts-v1.json");
        fs::write(&f, bytes).unwrap();
        assert!(migrate_file::<Value>(&f, 1048576, "shortcuts").is_err());
        assert_eq!(fs::read(&f).unwrap(), *bytes);
        assert_eq!(fs::read_dir(&p).unwrap().count(), 1);
        fs::remove_dir_all(p).unwrap();
    }
}
#[test]
fn versioned_format_requires_migration_backup_for_downgrade() {
    let bytes = encode_items(&[json!({"id":"work"})], "shortcuts").unwrap();
    assert!(serde_json::from_slice::<Vec<Value>>(&bytes).is_err());
}
#[test]
fn large_catalog_roundtrips() {
    let p = root("large");
    let f = p.join("applications-v1.json");
    let items:Vec<_>=(0..1000).map(|i|json!({"id":format!("app-{i}"),"name":format!("App {i}"),"aliases":[],"remember":false,"target":{"kind":"ShellApp","app_id":format!("App{i}")}})).collect();
    fs::write(&f, serde_json::to_vec(&items).unwrap()).unwrap();
    assert_eq!(
        migrate_file::<Value>(&f, 4194304, "applications").unwrap(),
        items
    );
    assert_eq!(
        migrate_file::<Value>(&f, 4194304, "applications").unwrap(),
        items
    );
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn simultaneous_first_reads_are_safe() {
    use std::sync::{Arc, Barrier};
    let p = root("concurrent");
    let f = p.join("shortcuts-v1.json");
    let items:Vec<_>=(0..30).map(|i|json!({"id":format!("s{i}"),"name":format!("Mode {i}"),"aliases":[],"remember":false,"actions":[{"kind":"url","url":"https://example.com"}]})).collect();
    fs::write(&f, serde_json::to_vec(&items).unwrap()).unwrap();
    let barrier = Arc::new(Barrier::new(8));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let f = f.clone();
            let b = barrier.clone();
            std::thread::spawn(move || {
                b.wait();
                migrate_file::<Value>(&f, 1048576, "shortcuts").map(|_| ())
            })
        })
        .collect();
    let errors: Vec<_> = handles
        .into_iter()
        .map(|h| h.join().unwrap())
        .filter_map(Result::err)
        .collect();
    fs::remove_dir_all(p).unwrap();
    assert!(
        errors.is_empty(),
        "Concurrent migration failures: {errors:?}"
    );
}
