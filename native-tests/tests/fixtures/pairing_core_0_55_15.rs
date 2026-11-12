// Frozen production pairing primitive from Bimu 0.55.15 (commit 257aad02).
// Only crate import names are aliased; keep protocol behavior unchanged.
//! PAKE pairing primitive. This module does not open sockets or synchronize data.
//! A caller must enforce expiry, attempt limits, consent, and channel binding.
use hmac_legacy::{Hmac, Mac};
use sha2::Sha256;
use spake2::{Ed25519Group, Identity, Password, Spake2};

const CLIENT_ID: &[u8] = b"bimu-pairing-client-v1";
const HOST_ID: &[u8] = b"bimu-pairing-host-v1";
type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Copy)]
pub enum Role { Host, Client }
impl Role {
    fn confirmation(self) -> &'static [u8] {
        match self { Self::Host => b"bimu-confirm-host-v1", Self::Client => b"bimu-confirm-client-v1" }
    }
    fn peer(self) -> Self { match self { Self::Host => Self::Client, Self::Client => Self::Host } }
}

pub struct Pending { state: Spake2<Ed25519Group>, role: Role }
pub struct AwaitingConfirmation { key: Vec<u8>, role: Role }
pub struct ConfirmedKey(Vec<u8>);
impl ConfirmedKey { pub fn as_bytes(&self) -> &[u8] { &self.0 } }

/// One-time six ASCII digits; generation and rate limiting belong to the caller.
pub fn start(role: Role, code: &str) -> Result<(Pending, Vec<u8>), &'static str> {
    if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) { return Err("invalid code"); }
    let password = Password::new(code.as_bytes());
    let client = Identity::new(CLIENT_ID);
    let host = Identity::new(HOST_ID);
    let (state, message) = match role {
        Role::Client => Spake2::<Ed25519Group>::start_a(&password, &client, &host),
        Role::Host => Spake2::<Ed25519Group>::start_b(&password, &client, &host),
    };
    Ok((Pending { state, role }, message))
}
impl Pending {
    /// Produces a confirmation tag but does not yet authenticate the other party.
    pub fn finish(self, peer_message: &[u8]) -> Result<(AwaitingConfirmation, Vec<u8>), &'static str> {
        if peer_message.len() != 33 { return Err("invalid pairing message"); }
        let key = self.state.finish(peer_message).map_err(|_| "invalid pairing message")?;
        let mut mac = HmacSha256::new_from_slice(&key).map_err(|_| "invalid key")?;
        mac.update(self.role.confirmation());
        let tag = mac.finalize().into_bytes().to_vec();
        Ok((AwaitingConfirmation { key, role: self.role }, tag))
    }
}
impl AwaitingConfirmation {
    /// A key is available only after the other role has confirmed the same secret.
    pub fn verify(self, peer_tag: &[u8]) -> Result<ConfirmedKey, &'static str> {
        let mut mac = HmacSha256::new_from_slice(&self.key).map_err(|_| "invalid key")?;
        mac.update(self.role.peer().confirmation());
        mac.verify_slice(peer_tag).map_err(|_| "confirmation failed")?;
        Ok(ConfirmedKey(self.key))
    }
}
