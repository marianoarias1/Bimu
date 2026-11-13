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
mod sessions {
    use std::sync::Mutex;
    use windows::Media::Control::{GlobalSystemMediaTransportControlsSession as Session,GlobalSystemMediaTransportControlsSessionManager as Manager,GlobalSystemMediaTransportControlsSessionMediaProperties as Properties,GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status};
    static PREVIOUS:Mutex<String>=Mutex::new(String::new());
    async fn manager()->windows::core::Result<Manager>{Manager::RequestAsync()?.await}
    pub async fn resolve()->windows::core::Result<Option<(Session,Properties)>>{
        let manager=manager().await?;
        let current=manager.GetCurrentSession().ok();
        let mut sessions:Vec<Session>=current.clone().into_iter().collect();
        match manager.GetSessions(){
            Ok(all)=>{for i in 0..all.Size()?.min(32){if let Ok(session)=all.GetAt(i){if !sessions.contains(&session){sessions.push(session)}}}},
            Err(error)=>{if sessions.is_empty(){return Err(error)}}
        }
        let previous=PREVIOUS.lock().map(|value|value.clone()).unwrap_or_default();
        let hints:Vec<_>=sessions.iter().map(|session|crate::media_selection::Candidate{
            playing:session.GetPlaybackInfo().and_then(|info|info.PlaybackStatus()).ok()==Some(Status::Playing),
            current:current.as_ref()==Some(session),
            previous:!previous.is_empty()&&session.SourceAppUserModelId().map(|id|id.to_string_lossy()==previous).unwrap_or(false)
        }).collect();
        let mut last_error=None;
        for index in crate::media_selection::ranked(&hints).into_iter().take(8){
            let session=&sessions[index];
            let result=match session.TryGetMediaPropertiesAsync(){Ok(operation)=>operation.await,Err(error)=>Err(error)};
            match result{
                Ok(props) if props.Title().map(|title|!title.to_string_lossy().trim().is_empty()).unwrap_or(false)=>{
                    if let Ok(mut value)=PREVIOUS.lock(){*value=session.SourceAppUserModelId().map(|id|id.to_string_lossy()).unwrap_or_default()}
                    return Ok(Some((session.clone(),props)))
                },
                Ok(_)=>{},Err(error)=>last_error=Some(error)
            }
        }
        if let Some(error)=last_error{return Err(error)}Ok(None)
    }
}
#[cfg(windows)]
async fn current()->windows::core::Result<windows::Media::Control::GlobalSystemMediaTransportControlsSession>{
    sessions::resolve().await?.map(|(session,_)|session).ok_or_else(||windows::core::Error::from_hresult(windows::core::HRESULT(0x80070490u32 as i32)))
}

#[tauri::command]
pub async fn media_state(include_cover: bool) -> Result<Option<MediaState>, String> {
    #[cfg(windows)]{crate::media_worker::run(move||media_state_inner(include_cover)).await}
    #[cfg(not(windows))]{let _=include_cover;Ok(None)}
}

async fn media_state_inner(include_cover: bool) -> Result<Option<MediaState>, String> {
    #[cfg(windows)] {
        use base64::{engine::general_purpose::STANDARD, Engine};
        use windows::{Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackStatus, Storage::Streams::DataReader};
        let (session,props)=match sessions::resolve().await.map_err(|e|e.to_string())?{Some(value)=>value,None=>return Ok(None)};
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
pub async fn media_control(action:String)->Result<(),String>{
    #[cfg(windows)]{crate::media_worker::run(move||media_control_inner(action)).await}
    #[cfg(not(windows))]{let _=action;Err("Disponible en Windows".into())}
}
async fn media_control_inner(action: String) -> Result<(), String> {
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
pub async fn media_seek(source:String,title:String,artist:String,seconds:f64)->Result<f64,String>{
    #[cfg(windows)]{crate::media_worker::run(move||media_seek_inner(source,title,artist,seconds)).await}
    #[cfg(not(windows))]{let _=(source,title,artist,seconds);Err("Disponible en Windows".into())}
}
async fn media_seek_inner(source: String, title: String, artist: String, seconds: f64) -> Result<f64, String> {
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

#[cfg(all(test,windows))]mod native_tests{
    #[test]fn fresh_thread_can_query_the_real_windows_media_service(){
        let result=std::thread::spawn(||tauri::async_runtime::block_on(super::media_state(false))).join().expect("native caller thread");
        assert!(result.is_ok(),"The real SMTC query must work without relying on another feature initializing COM: {:?}",result.err());
    }
    #[test]fn media_worker_keeps_one_initialized_apartment_for_repeated_requests(){
        #[link(name="ole32")]extern "system"{fn CoGetApartmentType(kind:*mut i32,qualifier:*mut i32)->i32;}
        let ids=tauri::async_runtime::block_on(async{
            let mut ids=Vec::new();for _ in 0..3{ids.push(crate::media_worker::run(||async{
                let(mut kind,mut qualifier)=(0,0);let result=unsafe{CoGetApartmentType(&mut kind,&mut qualifier)};
                assert!(result>=0);assert_eq!(kind,1,"MTA apartment");Ok(std::thread::current().id())
            }).await.unwrap())}ids
        });assert!(ids.iter().all(|id|id==&ids[0]));
    }
}
