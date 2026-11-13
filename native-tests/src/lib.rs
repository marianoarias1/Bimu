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

#[path="../../src-tauri/src/transfer_limits.rs"]pub mod transfer_limits;

#[path="../../src-tauri/src/application_icon_pixels.rs"]pub mod application_icon_pixels;

#[path="../../src-tauri/src/bounded_io.rs"]pub mod bounded_io;

#[path="../../src-tauri/src/random_core.rs"]pub mod random_core;

#[path="../../src-tauri/src/trusted_devices_core.rs"]pub mod trusted_devices_core;

#[path="../../src-tauri/src/connection_control.rs"]pub mod connection_control;

#[path="../../src-tauri/src/trusted_devices_store.rs"]pub mod trusted_devices_store;

#[path="../../src-tauri/src/thumbnail_pixels.rs"]pub mod thumbnail_pixels;

#[path="../../src-tauri/src/media_selection.rs"]pub mod media_selection;
