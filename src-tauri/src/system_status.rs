use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStatus {
    battery_percent: Option<u8>,
    charging: bool,
    wifi_signal: Option<u8>,
}

#[cfg(windows)]
#[repr(C)]
struct PowerStatus { ac: u8, flags: u8, percent: u8, reserved: u8, lifetime: u32, full_lifetime: u32 }

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" { fn GetSystemPowerStatus(status: *mut PowerStatus) -> i32; }

#[tauri::command]
pub async fn system_status()->Result<SystemStatus,String>{
    tauri::async_runtime::spawn_blocking(read_status).await.map_err(|_|"No se pudo consultar el estado del sistema.".into())
}
fn read_status() -> SystemStatus {
    let mut result = SystemStatus { battery_percent: None, charging: false, wifi_signal: None };
    #[cfg(windows)] {
        let mut status = PowerStatus { ac: 255, flags: 255, percent: 255, reserved: 0, lifetime: 0, full_lifetime: 0 };
        if unsafe { GetSystemPowerStatus(&mut status) } != 0 && status.flags != 255 && status.flags & 128 == 0 && status.percent <= 100 {
            result.battery_percent = Some(status.percent);
            result.charging = status.flags & 8 != 0;
        }
        if result.battery_percent.is_none() {
            if let Some(output) = wifi_output() {
                let output = String::from_utf8_lossy(&output.stdout);
                for line in output.lines() {
                    if line.contains('%') {
                        if let Some(value) = line.split(':').nth(1).and_then(|v| v.trim().trim_end_matches('%').trim().parse::<u8>().ok()).filter(|v| *v <= 100) {
                            result.wifi_signal = Some(value); break;
                        }
                    }
                }
            }
        }
    }
    result
}

#[cfg(windows)]
fn wifi_output()->Option<std::process::Output>{
    use std::{process::{Command,Stdio},os::windows::process::CommandExt,time::{Duration,Instant}};
    let mut child=Command::new("netsh.exe").args(["wlan","show","interfaces"]).creation_flags(0x08000000).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    let deadline=Instant::now()+Duration::from_secs(2);
    loop{
        match child.try_wait(){Ok(Some(_))=>return child.wait_with_output().ok(),Ok(None)=>{},Err(_)=>{let _=child.kill();let _=child.wait();return None}}
        if Instant::now()>=deadline{let _=child.kill();let _=child.wait();return None}
        std::thread::sleep(Duration::from_millis(25));
    }
}
