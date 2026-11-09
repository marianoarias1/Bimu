use serde::Serialize;

fn corrected_position(raw:f64,duration:f64,updated:f64,now:f64,playing:bool)->f64 {
    let delta=now-updated;
    let extra=if playing&&updated>0.0&&delta>=0.0&&delta<=duration.max(600.0){delta}else{0.0};
    let value=raw+extra;
    if duration>0.0 {value.clamp(0.0,duration)} else {value.max(0.0)}
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaState {
    title: String,
    artist: String,
    playing: bool,
    source: String,
    seek_range: Option<[f64; 2]>,
    position: f64,
    duration: f64,
    sampled_at_ms: u64,
    cover: Option<String>,
}

#[cfg(windows)]
async fn current() -> windows::core::Result<windows::Media::Control::GlobalSystemMediaTransportControlsSession> {
    use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager;
    GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.await?.GetCurrentSession()
}

#[tauri::command]
pub async fn media_state(include_cover: bool) -> Result<Option<MediaState>, String> {
    tauri::async_runtime::spawn_blocking(move || tauri::async_runtime::block_on(media_state_inner(include_cover)))
        .await.map_err(|e| e.to_string())?
}

async fn media_state_inner(include_cover: bool) -> Result<Option<MediaState>, String> {
    #[cfg(windows)] {
        use base64::{engine::general_purpose::STANDARD, Engine};
        use windows::{Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackStatus, Storage::Streams::DataReader};
        let session = match current().await { Ok(v) => v, Err(_) => return Ok(None) };
        let props = session.TryGetMediaPropertiesAsync().map_err(|e| e.to_string())?.await.map_err(|e| e.to_string())?;
        let title = props.Title().map(|v| v.to_string_lossy()).unwrap_or_default();
        if title.is_empty() { return Ok(None) }
        let artist = props.Artist().map(|v| v.to_string_lossy()).unwrap_or_default();
        let playback = session.GetPlaybackInfo().ok();
        let playing = playback.as_ref().and_then(|v| v.PlaybackStatus().ok()) == Some(GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing);
        let source = session.SourceAppUserModelId().map(|v| v.to_string_lossy()).unwrap_or_default();
        let seek_range = playback
            .and_then(|v| v.Controls().ok())
            .filter(|v| v.IsPlaybackPositionEnabled().unwrap_or(false))
            .filter(|_| !source.is_empty())
            .and_then(|_| seek_range(&session));
        let sampled_at_ms=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis().min(u64::MAX as u128) as u64;
        let (position,duration) = session.GetTimelineProperties().map(|v| {
            let raw=v.Position().map(|x| x.Duration as f64/10_000_000.0).unwrap_or(0.0);
            let duration=v.EndTime().map(|x| x.Duration as f64/10_000_000.0).unwrap_or(0.0);
            let updated=v.LastUpdatedTime().map(|x|x.UniversalTime as f64/10_000_000.0-11_644_473_600.0).unwrap_or(0.0);
            (corrected_position(raw,duration,updated,sampled_at_ms as f64/1000.0,playing),duration)
        }).unwrap_or((0.0,0.0));
        let cover = if include_cover { if let Ok(reference) = props.Thumbnail() {
            if let Ok(op) = reference.OpenReadAsync() {
                if let Ok(stream) = op.await {
                    let size = stream.Size().unwrap_or(0);
                    if size > 0 && size <= 2_000_000 {
                        if let Ok(reader) = DataReader::CreateDataReader(&stream) {
                            if let Ok(load) = reader.LoadAsync(size as u32) {
                                if load.await.is_ok() {
                                    let mut bytes = vec![0u8; size as usize];
                                    if reader.ReadBytes(&mut bytes).is_ok() { Some(format!("data:image/jpeg;base64,{}", STANDARD.encode(bytes))) } else { None }
                                } else { None }
                            } else { None }
                        } else { None }
                    } else { None }
                } else { None }
            } else { None }
        } else { None } } else { None };
        return Ok(Some(MediaState { title, artist, playing, source, seek_range, position, duration, sampled_at_ms, cover }))
    }
    #[cfg(not(windows))] { let _ = include_cover; Ok(None) }
}

#[cfg(test)]
mod tests {
    use super::corrected_position;
    #[test] fn advances_stale_playback_but_not_paused_or_invalid_timestamps(){
        assert!((corrected_position(10.0,100.0,1000.0,1000.4,true)-10.4).abs()<0.0001);
        assert_eq!(corrected_position(10.0,100.0,1000.0,1000.4,false),10.0);
        assert_eq!(corrected_position(10.0,100.0,0.0,1000.4,true),10.0);
        assert_eq!(corrected_position(99.0,100.0,1000.0,1005.0,true),100.0);
    }
}

#[tauri::command]
pub async fn media_control(action: String) -> Result<(), String> {
    #[cfg(windows)] {
        let session = current().await.map_err(|e| e.to_string())?;
        let request = match action.as_str() {
            "toggle" => session.TryTogglePlayPauseAsync(),
            "previous" => session.TrySkipPreviousAsync(),
            "next" => session.TrySkipNextAsync(),
            _ => return Err("Acción multimedia desconocida".into()),
        }.map_err(|e| e.to_string())?;
        return if request.await.map_err(|e| e.to_string())? { Ok(()) } else { Err("El reproductor no pudo realizar la acción".into()) }
    }
    #[cfg(not(windows))] { let _ = action; Err("Disponible en Windows".into()) }
}

#[cfg(windows)]
fn seek_range(session: &windows::Media::Control::GlobalSystemMediaTransportControlsSession) -> Option<[f64; 2]> {
    let timeline = session.GetTimelineProperties().ok()?;
    let end = timeline.EndTime().ok()?.Duration;
    let start = timeline.StartTime().ok()?.Duration.max(0);
    let reported_min = timeline.MinSeekTime().ok()?.Duration;
    let reported_max = timeline.MaxSeekTime().ok()?.Duration;
    let minimum = reported_min.max(start);
    // Some clients leave both seek limits at zero while reporting a duration.
    let maximum = if reported_min == 0 && reported_max == 0 { end } else { reported_max.min(end) };
    (maximum > minimum).then_some([minimum as f64 / 10_000_000., maximum as f64 / 10_000_000.])
}

#[tauri::command]
pub async fn media_seek(source: String, title: String, artist: String, seconds: f64) -> Result<f64, String> {
    if !seconds.is_finite() || seconds < 0. {
        return Err("Posición de reproducción inválida.".into());
    }
    #[cfg(windows)]
    {
        let session = current().await.map_err(|_| "No hay una reproducción activa.")?;
        let props = session.TryGetMediaPropertiesAsync().map_err(|_| "No se pudo consultar la reproducción.")?.await.map_err(|_| "No se pudo consultar la reproducción.")?;
        if session.SourceAppUserModelId().map(|v| v.to_string_lossy()).unwrap_or_default() != source
            || props.Title().map(|v| v.to_string_lossy()).unwrap_or_default() != title
            || props.Artist().map(|v| v.to_string_lossy()).unwrap_or_default() != artist
        {
            return Err("La reproducción cambió. Volvé a mover la barra.".into());
        }
        if !session.GetPlaybackInfo().and_then(|v| v.Controls()).and_then(|v| v.IsPlaybackPositionEnabled()).unwrap_or(false) {
            return Err("El reproductor no permite cambiar la posición.".into());
        }
        let [min, max] = seek_range(&session).ok_or("La reproducción no tiene una posición ajustable.")?;
        let target = seconds.clamp(min, max);
        let request = session.TryChangePlaybackPositionAsync((target * 10_000_000.).round() as i64)
            .map_err(|_| "No se pudo cambiar la posición.")?;
        if request.await.unwrap_or(false) {
            Ok(target)
        } else {
            Err("El reproductor no aceptó el cambio de posición.".into())
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (source, title, artist);
        Err("Disponible en Windows".into())
    }
}
