//! Experimental loopback-tested transport, not wired to the application.
//! Not a complete pairing protocol: rate limits, expiry, consent and persistent
//! identity are mandatory before allowing access from another computer.
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use sha2::{Digest, Sha256};
use crate::pairing_core::{self, Role};

const PATTERN: &str = "Noise_NNpsk0_25519_ChaChaPoly_SHA256";
const MAX_HANDSHAKE: usize = 4096;
const MAX_FRAME: usize = 65535;
const MAX_PLAINTEXT: usize = MAX_FRAME - 16;
const PROLOGUE: &[u8] = b"bimu-experimental-local-pairing-v1";
fn read_error(stage: &str, error: std::io::Error) -> String {
    let detail = match error.kind() {
        std::io::ErrorKind::UnexpectedEof => "la otra PC cerró la conexión antes de completar el intercambio".to_owned(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => "se agotó el tiempo de espera".to_owned(),
        _ => format!("error de red: {error}"),
    };
    format!("{stage}: {detail}")
}
fn read_frame(stream: &mut TcpStream, limit: usize) -> Result<Vec<u8>, String> {
    let mut header = [0u8; 2];
    stream.read_exact(&mut header).map_err(|e| read_error("cabecera", e))?;
    let len = u16::from_be_bytes(header) as usize;
    if len == 0 || len > limit { return Err("tamaño de mensaje inválido".into()); }
    let mut frame = vec![0; len];
    stream.read_exact(&mut frame).map_err(|e| read_error("contenido", e))?;
    Ok(frame)
}
fn write_frame(stream: &mut TcpStream, frame: &[u8]) -> Result<(), String> {
    if frame.is_empty() || frame.len() > MAX_FRAME { return Err("invalid frame length".into()); }
    stream.write_all(&(frame.len() as u16).to_be_bytes()).map_err(|e| e.to_string())?;
    stream.write_all(frame).map_err(|e| e.to_string())
}
/// Blocks on I/O and is for an isolated test only, never the UI thread.
pub fn pair(mut stream: TcpStream, role: Role, code: &str) -> Result<SecureChannel, String> {
    // Windows accepted sockets can inherit nonblocking mode from the listener.
    // Handshake framing uses read_exact, so normalize before applying timeouts.
    stream.set_nodelay(true).map_err(|e| e.to_string())?;
    stream.set_nonblocking(false).map_err(|e| format!("preparar conexión: {e}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
    stream.set_write_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
    let (pending, message) = pairing_core::start(role, code).map_err(str::to_owned)?;
    let peer_message = (match role {
        Role::Client => { write_frame(&mut stream, &message)?; read_frame(&mut stream, MAX_HANDSHAKE) }
        Role::Host => { let peer = read_frame(&mut stream, MAX_HANDSHAKE)?; write_frame(&mut stream, &message)?; Ok(peer) }
    }).map_err(|e| format!("intercambio inicial: {e}"))?;
    let (waiting, confirmation) = pending.finish(&peer_message).map_err(str::to_owned)?;
    let confirmed = match role {
        Role::Client => {
            write_frame(&mut stream, &confirmation)?;
            let tag = read_frame(&mut stream, MAX_HANDSHAKE).map_err(|e|format!("confirmación del código: {e}"))?;
            waiting.verify(&tag).map_err(|_| "El código no coincide o la verificación fue rechazada".to_string())?
        }
        Role::Host => {
            let tag = read_frame(&mut stream, MAX_HANDSHAKE).map_err(|e|format!("confirmación del código: {e}"))?;
            let key = waiting.verify(&tag).map_err(|_| "El código no coincide o la verificación fue rechazada".to_string())?;
            write_frame(&mut stream, &confirmation)?;
            key
        }
    };
    // Bind the newly authenticated secret to Noise, rather than sending plaintext.
    let mut digest = Sha256::new();
    digest.update(b"bimu-noise-psk-v1\0");
    digest.update(confirmed.as_bytes());
    let psk: [u8; 32] = digest.finalize().into();
    reconnect(stream, role, &psk)
}
/// Authenticates a stored random secret before any application data is exchanged.
pub fn reconnect(mut stream: TcpStream, role: Role, psk: &[u8; 32]) -> Result<SecureChannel, String> {
    stream.set_nodelay(true).map_err(|e| e.to_string())?;
    stream.set_nonblocking(false).map_err(|e| e.to_string())?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
    stream.set_write_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
    let params = PATTERN.parse().map_err(|_| "invalid noise pattern".to_string())?;
    let builder = snow::Builder::new(params).psk(0, psk).map_err(|e| e.to_string())?
        .prologue(PROLOGUE).map_err(|e| e.to_string())?;
    let mut state = match role {
        Role::Client => builder.build_initiator(),
        Role::Host => builder.build_responder(),
    }.map_err(|e| e.to_string())?;
    let mut output = vec![0; MAX_HANDSHAKE];
    match role {
        Role::Client => {
            let n = state.write_message(&[], &mut output).map_err(|e| e.to_string())?;
            write_frame(&mut stream, &output[..n])?;
            let reply = read_frame(&mut stream, MAX_HANDSHAKE).map_err(|e|format!("canal cifrado: {e}"))?;
            state.read_message(&reply, &mut output).map_err(|e| e.to_string())?;
        }
        Role::Host => {
            let request = read_frame(&mut stream, MAX_HANDSHAKE).map_err(|e|format!("canal cifrado: {e}"))?;
            state.read_message(&request, &mut output).map_err(|e| e.to_string())?;
            let n = state.write_message(&[], &mut output).map_err(|e| e.to_string())?;
            write_frame(&mut stream, &output[..n])?;
        }
    }
    let transport = state.into_transport_mode().map_err(|e| e.to_string())?;
    Ok(SecureChannel { stream, transport })
}
pub struct SecureChannel { stream: TcpStream, transport: snow::TransportState }
impl SecureChannel {
    pub(crate) fn receive_timeout(&self,seconds:u64)->Result<(),String>{self.stream.set_read_timeout(Some(Duration::from_secs(seconds))).map_err(|e|e.to_string())}
    pub(crate) fn control_stream(&self)->std::io::Result<TcpStream>{self.stream.try_clone()}
    pub fn send(&mut self, data: &[u8]) -> Result<(), String> {
        if data.len() > MAX_PLAINTEXT { return Err("message too large".into()); }
        let mut ciphertext = vec![0; data.len() + 16];
        let n = self.transport.write_message(data, &mut ciphertext).map_err(|e| e.to_string())?;
        write_frame(&mut self.stream, &ciphertext[..n])
    }
    pub fn receive(&mut self) -> Result<Vec<u8>, String> {
        let ciphertext = read_frame(&mut self.stream, MAX_FRAME)?;
        let mut plaintext = vec![0; ciphertext.len()];
        let n = self.transport.read_message(&ciphertext, &mut plaintext).map_err(|e| e.to_string())?;
        plaintext.truncate(n); Ok(plaintext)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;
    #[test]
    fn correct_code_protects_messages_in_both_directions() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut channel = pair(stream, Role::Host, "012345").unwrap();
            assert_eq!(channel.receive().unwrap(), b"texto de prueba");
            channel.send(b"recibido").unwrap();
        });
        let mut client = pair(TcpStream::connect(addr).unwrap(), Role::Client, "012345").unwrap();
        client.send(b"texto de prueba").unwrap();
        assert_eq!(client.receive().unwrap(), b"recibido");
        server.join().unwrap();
    }
    #[test]
    fn nonblocking_listener_and_delayed_client_can_pair() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => thread::sleep(Duration::from_millis(5)),
                    Err(e) => panic!("accept: {e}"),
                }
            };
            // Force nonblocking as well, making this regression meaningful on Unix.
            stream.set_nonblocking(true).unwrap();
            let mut channel = pair(stream, Role::Host, "012345").expect("host pairing");
            assert_eq!(channel.receive().unwrap(), b"hola");
        });
        let stream = TcpStream::connect(addr).unwrap();
        thread::sleep(Duration::from_millis(150));
        let mut client = pair(stream, Role::Client, "012345").expect("client pairing");
        client.send(b"hola").unwrap();
        server.join().unwrap();
    }
    #[test]
    fn closed_peer_reports_the_failing_stage() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            read_frame(&mut stream, MAX_HANDSHAKE).unwrap();
            drop(stream);
        });
        let error = match pair(TcpStream::connect(addr).unwrap(), Role::Client, "012345") {
            Ok(_) => panic!("pairing should fail"), Err(error) => error,
        };
        assert!(error.contains("intercambio inicial"), "{error}");
        assert!(error.contains("cerró la conexión"), "{error}");
        server.join().unwrap();
    }
    #[test]
    fn reconnect_accepts_only_the_stored_key() {
        let listener=TcpListener::bind("127.0.0.1:0").unwrap();
        let addr=listener.local_addr().unwrap();
        let server=thread::spawn(move||{let (stream,_)=listener.accept().unwrap();let mut c=reconnect(stream,Role::Host,&[7;32]).unwrap();assert_eq!(c.receive().unwrap(),b"ok");});
        let mut client=reconnect(TcpStream::connect(addr).unwrap(),Role::Client,&[7;32]).unwrap();client.send(b"ok").unwrap();server.join().unwrap();
        let listener=TcpListener::bind("127.0.0.1:0").unwrap();let addr=listener.local_addr().unwrap();
        let server=thread::spawn(move||{let (stream,_)=listener.accept().unwrap();assert!(reconnect(stream,Role::Host,&[7;32]).is_err());});
        assert!(reconnect(TcpStream::connect(addr).unwrap(),Role::Client,&[8;32]).is_err());server.join().unwrap();
    }
    #[test]
    fn wrong_code_never_reaches_transport() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            assert!(pair(stream, Role::Host, "012345").is_err());
        });
        assert!(pair(TcpStream::connect(addr).unwrap(), Role::Client, "012346").is_err());
        server.join().unwrap();
    }
}
