fn main() {
    // Only override the executable's CompanyName while tauri-build creates its
    // Windows resource. Do not change bundle.publisher for the parent bundler:
    // NSIS must keep its legacy Software\\mariano\\Bimu installation-location key.
    // Store package Identity/Publisher and the application identifier stay intact.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut config: serde_json::Value = match std::env::var("TAURI_CONFIG") {
            Ok(value) => serde_json::from_str(&value).expect("Invalid Tauri build configuration"),
            Err(std::env::VarError::NotPresent) => serde_json::json!({}),
            Err(error) => panic!("Cannot read Tauri build configuration: {error}"),
        };
        let bundle = config
            .as_object_mut()
            .expect("Tauri build configuration must be an object")
            .entry("bundle")
            .or_insert_with(|| serde_json::json!({}))
            .as_object_mut()
            .expect("Tauri bundle configuration must be an object");
        bundle.insert("publisher".into(), serde_json::json!("Bimu"));
        // This environment change is local to the build-script process and its
        // children; the CLI/NSIS parent retains its original configuration.
        std::env::set_var("TAURI_CONFIG", config.to_string());
    }
    tauri_build::build()
}
