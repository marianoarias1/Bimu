//! Discovery only: advertisements are untrusted and never authorize a peer.
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

const PORT: u16 = 45948;
const MAGIC: &str = "bimu-pairing-discovery-v1";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Advertisement { #[serde(default)]pub management:bool, magic: String, pub name: String, pub port: u16 }
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredPeer {pub management:bool, pub name: String, pub address: String }
fn valid_ip(ip: Ipv4Addr) -> bool { ip.is_private() || ip.is_link_local() }
fn decode(bytes: &[u8], source: SocketAddr) -> Option<DiscoveredPeer> {
    if bytes.len() > 512 { return None; }
    let SocketAddr::V4(source) = source else { return None; };
    if !valid_ip(*source.ip()) { return None; }
    let ad: Advertisement = serde_json::from_slice(bytes).ok()?;
    if ad.magic != MAGIC || ad.port == 0 || ad.name.trim().is_empty() || ad.name.len() > 128 || ad.name.chars().any(char::is_control) { return None; }
    Some(DiscoveredPeer {management:ad.management, name: ad.name, address: SocketAddrV4::new(*source.ip(), ad.port).to_string() })
}
pub fn advertise(name: &str, port: u16) -> Result<UdpSocket, String> {
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    socket.set_broadcast(true).map_err(|e| e.to_string())?;
    let ad = Advertisement {management:true, magic: MAGIC.into(), name: name.into(), port };
    let bytes = serde_json::to_vec(&ad).map_err(|e| e.to_string())?;
    socket.send_to(&bytes, (Ipv4Addr::BROADCAST, PORT)).map_err(|e| e.to_string())?;
    Ok(socket)
}
pub fn send(socket: &UdpSocket, name: &str, port: u16) {
    let ad = Advertisement {management:true, magic: MAGIC.into(), name: name.into(), port };
    if let Ok(bytes) = serde_json::to_vec(&ad) { let _ = socket.send_to(&bytes, (Ipv4Addr::BROADCAST, PORT)); }
}
pub fn scan() -> Result<Vec<DiscoveredPeer>, String> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, PORT)).map_err(|_| "No se pudo buscar equipos en la red. Revisá el firewall de Windows.")?;
    socket.set_read_timeout(Some(Duration::from_millis(200))).map_err(|e| e.to_string())?;
    let until = Instant::now() + Duration::from_millis(1500);
    let mut peers: Vec<DiscoveredPeer> = Vec::new();
    let mut buf = [0u8; 513];
    while Instant::now() < until {
        if let Ok((n, source)) = socket.recv_from(&mut buf) {
            if let Some(peer) = decode(&buf[..n], source) {
                if !peers.iter().any(|known| known.address == peer.address) { peers.push(peer); }
                if peers.len() >= 32 { break; }
            }
        }
    }
    Ok(peers)
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn advertisements_only_propose_addresses() {
        let ad = br#"{"magic":"bimu-pairing-discovery-v1","name":"PC","port":1234}"#;
        assert_eq!(decode(ad, "192.168.1.10:9000".parse().unwrap()).unwrap().address, "192.168.1.10:1234");
        assert!(decode(ad, "8.8.8.8:9000".parse().unwrap()).is_none());
        assert!(decode(br#"{"magic":"other","name":"PC","port":1234}"#, "192.168.1.10:9000".parse().unwrap()).is_none());
    }
}
