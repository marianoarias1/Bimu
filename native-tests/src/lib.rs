#![allow(dead_code)]
// Exercise the production modules that support current Bimu features.
#[path="../../src-tauri/src/application_intent.rs"]pub mod application_intent;
#[path="../../src-tauri/src/application_registry.rs"]pub mod application_registry;
#[path="../../src-tauri/src/config_migration.rs"]pub mod config_migration;
#[path="../../src-tauri/src/release_core.rs"]pub mod release_core;
#[path="../../src-tauri/src/assistant_core.rs"]pub mod assistant_core;
#[path="../../src-tauri/src/startup_core.rs"]pub mod startup_core;

#[path="../../src-tauri/src/pairing_core.rs"]pub mod pairing_core;
#[path="../../src-tauri/src/clipboard_sync_core.rs"]pub mod clipboard_sync_core;
#[path="../../src-tauri/src/pairing_transport.rs"]pub mod pairing_transport;
#[path="../../src-tauri/src/clipboard_protocol.rs"]pub mod clipboard_protocol;
#[path="../../src-tauri/src/clipboard_files.rs"]pub mod clipboard_files;
#[path="../../src-tauri/src/shelf_shared.rs"]pub mod shelf_shared;

#[path="../../src-tauri/src/pairing_discovery.rs"]pub mod pairing_discovery;
