//! PAKE pairing primitive. This module does not open sockets or synchronize data.
//! A caller must enforce expiry, attempt limits, consent, and channel binding.
use hmac::{Hmac, KeyInit, Mac};
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn both_sides_confirm_the_same_key() {
        let (a, am) = start(Role::Host, "001234").unwrap();
        let (b, bm) = start(Role::Client, "001234").unwrap();
        let (a, at) = a.finish(&bm).unwrap();
        let (b, bt) = b.finish(&am).unwrap();
        assert_eq!(a.verify(&bt).unwrap().as_bytes(), b.verify(&at).unwrap().as_bytes());
    }
    #[test]
    fn incorrect_code_and_reflected_confirmation_are_rejected() {
        let (a, am) = start(Role::Host, "001234").unwrap();
        let (b, bm) = start(Role::Client, "001235").unwrap();
        let (a, at) = a.finish(&bm).unwrap();
        let (b, bt) = b.finish(&am).unwrap();
        assert!(a.verify(&bt).is_err());
        assert!(b.verify(&at).is_err());
        let (a, am) = start(Role::Host, "001234").unwrap();
        let (b, bm) = start(Role::Client, "001234").unwrap();
        let (a, at) = a.finish(&bm).unwrap();
        let _ = b.finish(&am).unwrap();
        assert!(a.verify(&at).is_err());
    }
    #[test]
    fn rejects_invalid_codes_and_messages() {
        for code in ["12345", "1234567", "1234a6", "１２３４５６"] { assert!(start(Role::Host, code).is_err()); }
        let (a, _) = start(Role::Host, "123456").unwrap();
        assert!(a.finish(&[0; 32]).is_err());
    }
}
