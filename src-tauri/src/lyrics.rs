use serde::{Deserialize, Serialize};
use std::{sync::OnceLock,time::Duration};

#[derive(Deserialize,Debug)]
#[serde(rename_all="camelCase")]
struct Candidate { track_name: String, artist_name: String, duration: Option<f64>, #[serde(default)] instrumental: bool, synced_lyrics: Option<String> }
#[derive(Serialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct LyricsResult { status: &'static str, synced_lyrics: Option<String> }
#[derive(Serialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct LyricsError { kind:&'static str, retry_after_seconds:u64 }
impl LyricsError { fn new(kind:&'static str)->Self{Self{kind,retry_after_seconds:15}} }
fn network_error(error:reqwest::Error)->LyricsError{LyricsError::new(if error.is_timeout(){"timeout"}else if error.is_connect(){"offline"}else{"temporary"})}
fn normalized(s: &str) -> String { s.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect() }
fn select(candidates:Vec<Candidate>,title:&str,artist:&str,duration:f64)->LyricsResult{
 let title=normalized(title);let artist=normalized(artist);
 let selected=candidates.into_iter().filter(|v|normalized(&v.track_name)==title&&normalized(&v.artist_name)==artist&&(duration<=0.0||v.duration.filter(|d|d.is_finite()).map(|d|(d-duration).abs()<=5.0).unwrap_or(false))).min_by(|a,b|{
  // Prefer synchronized records over empty duplicates, then nearest duration.
  let rank=|v:&Candidate|if v.instrumental||v.synced_lyrics.as_ref().is_some_and(|s|!s.trim().is_empty()){0}else{1};
  rank(a).cmp(&rank(b)).then_with(||a.duration.map(|d|(d-duration).abs()).unwrap_or(f64::MAX).total_cmp(&b.duration.map(|d|(d-duration).abs()).unwrap_or(f64::MAX)))
 });
 match selected {Some(v) if v.instrumental=>LyricsResult{status:"instrumental",synced_lyrics:None},Some(v) if v.synced_lyrics.as_ref().is_some_and(|s|!s.trim().is_empty())=>LyricsResult{status:"synced",synced_lyrics:v.synced_lyrics},_=>LyricsResult{status:"missing",synced_lyrics:None}}
}
fn transient(status:u16)->bool{matches!(status,408|429|500|502|503|504)}
async fn request(client:&reqwest::Client,url:&str,title:&str,artist:&str,delay:Duration)->Result<Vec<Candidate>,LyricsError>{
 for attempt in 0..3u32{
  let response=client.get(url).query(&[("track_name",title),("artist_name",artist)]).send().await;
  let mut response=match response{
   Ok(response)=>response,
   Err(error)=>{if attempt<2{tokio::time::sleep(delay*(1<<attempt)).await;continue}return Err(network_error(error))}
  };
  let status=response.status();
  if !status.is_success(){
   let wait=response.headers().get(reqwest::header::RETRY_AFTER).and_then(|v|v.to_str().ok()).and_then(|v|v.parse::<u64>().ok());
   if transient(status.as_u16())&&attempt<2&&wait.unwrap_or(0)<=5{
    drop(response);tokio::time::sleep(wait.map(Duration::from_secs).unwrap_or(delay*(1<<attempt))).await;continue
   }
   return Err(LyricsError{kind:"temporary",retry_after_seconds:wait.unwrap_or(15).clamp(5,60)})
  }
  const MAX_BYTES:usize=1_000_000;
  if response.content_length().unwrap_or(0)>MAX_BYTES as u64{return Err(LyricsError::new("temporary"))}
  let mut bytes=Vec::new();let mut failed=None;
  loop{match response.chunk().await{
   Ok(Some(chunk))=>{if bytes.len().saturating_add(chunk.len())>MAX_BYTES{return Err(LyricsError::new("temporary"))}bytes.extend_from_slice(&chunk)},
   Ok(None)=>break,Err(error)=>{failed=Some(network_error(error));break}
  }}
  if let Some(error)=failed{if attempt<2{tokio::time::sleep(delay*(1<<attempt)).await;continue}return Err(error)}
  return serde_json::from_slice(&bytes).map_err(|_|LyricsError::new("temporary"))
 }Err(LyricsError::new("temporary"))
}

#[tauri::command]
pub async fn lyrics_lookup(title: String, artist: String, duration: f64) -> Result<LyricsResult, LyricsError> {
 if title.trim().is_empty()||artist.trim().is_empty()||title.len()>500||artist.len()>500||!duration.is_finite(){return Err(LyricsError::new("invalid"))}
 static CLIENT:OnceLock<reqwest::Client>=OnceLock::new();
 let client=if let Some(client)=CLIENT.get(){client}else{
  let client=reqwest::Client::builder().timeout(Duration::from_secs(6)).connect_timeout(Duration::from_secs(4)).user_agent(concat!("Bimu/",env!("CARGO_PKG_VERSION")," (https://github.com/marianoarias1/Bimu)")).build().map_err(|_|LyricsError::new("temporary"))?;
  let _=CLIENT.set(client);CLIENT.get().ok_or_else(||LyricsError::new("temporary"))?
 };
 let candidates=request(client,"https://lrclib.net/api/search",&title,&artist,Duration::from_millis(800)).await?;
 Ok(select(candidates,&title,&artist,duration))
}
#[cfg(test)]mod tests{
 use super::*;
 fn candidate(duration:f64,lyrics:Option<&str>)->Candidate{Candidate{track_name:"Song".into(),artist_name:"Artist".into(),duration:Some(duration),instrumental:false,synced_lyrics:lyrics.map(str::to_owned)}}
 #[test]fn selects_synced_duplicates_without_accepting_another_recording(){
  assert_eq!(select(vec![candidate(100.0,None),candidate(101.0,Some("[00:01]Hello"))],"Song","Artist",100.0).status,"synced");
  assert_eq!(select(vec![candidate(150.0,Some("[00:01]Live"))],"Song","Artist",100.0).status,"missing");
  assert_eq!(select(vec![candidate(100.0,Some("[00:01]Hello"))],"Other","Artist",100.0).status,"missing");
 }
 fn server(statuses:Vec<(&'static str,&'static str)>)->(String,std::thread::JoinHandle<usize>){
  use std::io::{Read,Write};
  let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();let url=format!("http://{}/search",listener.local_addr().unwrap());
  listener.set_nonblocking(true).unwrap();
  let handle=std::thread::spawn(move||{let mut count=0;let deadline=std::time::Instant::now()+Duration::from_secs(10);for(status,body)in statuses{
   let mut socket=loop{match listener.accept(){Ok((socket,_))=>break socket,Err(error)if error.kind()==std::io::ErrorKind::WouldBlock=>{assert!(std::time::Instant::now()<deadline,"request did not arrive");std::thread::sleep(Duration::from_millis(5));},Err(error)=>panic!("{error}")}};
   socket.set_read_timeout(Some(Duration::from_secs(2))).unwrap();let mut request=[0;4096];let _=socket.read(&mut request).unwrap();
   write!(socket,"HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();count+=1;
  }count});(url,handle)
 }
 #[tokio::test]async fn retries_503_internally_and_returns_the_lyrics(){
  let(url,server)=server(vec![("503 Service Unavailable",""),("200 OK","[{\"trackName\":\"Song\",\"artistName\":\"Artist\",\"duration\":100,\"syncedLyrics\":\"[00:01]Hello\"}]")]);
  let list=request(&reqwest::Client::new(),&url,"Song","Artist",Duration::from_millis(5)).await.unwrap();assert_eq!(select(list,"Song","Artist",100.0).status,"synced");assert_eq!(server.join().unwrap(),2);
 }
 #[tokio::test]async fn outage_stops_after_three_attempts_and_never_exposes_http(){
  let(url,server)=server(vec![("503 Service Unavailable","");3]);let error=request(&reqwest::Client::new(),&url,"Song","Artist",Duration::from_millis(5)).await.unwrap_err();
  assert_eq!(error.kind,"temporary");assert_eq!(server.join().unwrap(),3);
 }
 #[tokio::test]async fn non_transient_errors_do_not_retry(){
  let(url,server)=server(vec![("400 Bad Request","")]);assert!(request(&reqwest::Client::new(),&url,"Song","Artist",Duration::from_millis(5)).await.is_err());assert_eq!(server.join().unwrap(),1);
 }
}
