//! Protected storage model. Revoked records authorize revocation receipts only.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[derive(Clone, Serialize, Deserialize)]
pub struct Trust {
    pub key: [u8; 32],
    pub host: bool,
}
impl Trust {
    pub fn role(&self) -> crate::pairing_core::Role {
        if self.host {
            crate::pairing_core::Role::Host
        } else {
            crate::pairing_core::Role::Client
        }
    }
    pub fn tag(&self) -> String {
        let mut h = Sha256::new();
        h.update(b"bimu-trusted-discovery-v1");
        h.update(self.key);
        h.finalize()[..16]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct KnownDevice {
    pub id: String,
    pub name: String,
    pub trust: Trust,
    #[serde(default)]
    pub last_connected: u64,
    #[serde(default)]
    pub management: bool,
    #[serde(default)]
    pub revoked: bool,
    #[serde(default)]
    pub notified: bool,
    #[serde(default)]
    pub paused: bool,
    #[serde(default)]
    pub pause_pending: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct DeviceBook {
    pub version: u8,
    pub devices: Vec<KnownDevice>,
    pub selected: Option<String>,
    pub favorite: Option<String>,
    pub automatic: bool,
}
impl Default for DeviceBook {
    fn default() -> Self {
        Self {
            version: 2,
            devices: vec![],
            selected: None,
            favorite: None,
            automatic: true,
        }
    }
}
impl DeviceBook {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 2 || self.devices.len() > 128 {
            return Err("Registro de equipos inválido".into());
        }
        let mut ids = std::collections::HashSet::new();
        for d in &self.devices {
            if d.id != d.trust.tag()
                || !ids.insert(&d.id)
                || d.name.is_empty()
                || d.name.len() > 256
                || d.name.chars().any(char::is_control)
            {
                return Err("Equipo guardado inválido".into());
            }
        }
        for id in [&self.selected, &self.favorite].into_iter().flatten() {
            if !self.devices.iter().any(|d| d.id == *id && !d.revoked) {
                return Err("Preferencia de equipo inválida".into());
            }
        }
        Ok(())
    }
    pub fn selected(&self) -> Option<&KnownDevice> {
        self.selected
            .as_ref()
            .and_then(|id| self.devices.iter().find(|d| &d.id == id && !d.revoked))
            .or_else(|| self.devices.iter().find(|d| !d.revoked))
    }
    pub fn remember(&mut self, trust: Trust) -> Result<(), String> {
        let id = trust.tag();
        if let Some(d) = self.devices.iter_mut().find(|d| d.id == id) {
            if d.revoked {
                return Err("Este vínculo fue revocado. Generá un código nuevo.".into());
            }
            d.trust = trust;
        } else {
            if self.devices.iter().filter(|d| !d.revoked).count() >= 32 || self.devices.len() >= 128
            {
                return Err("Se alcanzó el límite de equipos guardados.".into());
            }
            self.devices.push(KnownDevice {
                id: id.clone(),
                name: "Equipo vinculado".into(),
                trust,
                last_connected: 0,
                management: false,
                revoked: false,
                notified: false,
                paused: false,
                pause_pending: false,
            });
        }
        self.selected = Some(id.clone());
        if self.favorite.is_none() {
            self.favorite = Some(id)
        }
        self.automatic = true;
        Ok(())
    }
    pub fn revoke(&mut self, id: &str, notified: bool) -> Result<(), String> {
        let d = self
            .devices
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or("El equipo ya no está guardado")?;
        d.revoked = true;
        d.notified |= notified;
        d.pause_pending = false;
        if self.selected.as_deref() == Some(id) {
            self.selected = None
        }
        if self.favorite.as_deref() == Some(id) {
            self.favorite = None
        }
        Ok(())
    }
    pub fn ranked(&self) -> Vec<KnownDevice> {
        let mut rows = self.devices.clone();
        rows.sort_by_key(|d| {
            (
                !d.revoked,
                self.favorite.as_deref() != Some(&d.id),
                std::cmp::Reverse(d.last_connected),
            )
        });
        rows
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn trust(n: u8) -> Trust {
        Trust {
            key: [n; 32],
            host: n % 2 == 0,
        }
    }
    #[test]
    fn remembers_multiple_devices_without_replacing_keys() {
        let mut b = DeviceBook::default();
        b.remember(trust(1)).unwrap();
        let first = b.favorite.clone();
        b.remember(trust(2)).unwrap();
        assert_eq!(b.devices.len(), 2);
        assert_eq!(b.favorite, first);
        b.validate().unwrap();
    }
    #[test]
    fn revocation_blocks_data_and_selection_but_retains_a_receipt() {
        let mut b = DeviceBook::default();
        b.remember(trust(1)).unwrap();
        let id = b.selected.clone().unwrap();
        b.revoke(&id, false).unwrap();
        assert!(b.selected().is_none());
        assert!(b.devices[0].revoked && !b.devices[0].notified);
        assert!(b.remember(trust(1)).is_err());
        b.revoke(&id, true).unwrap();
        assert!(b.devices[0].notified);
        b.validate().unwrap();
    }
    #[test]
    fn rejects_duplicate_ids_and_inconsistent_preferences() {
        let mut b = DeviceBook::default();
        b.remember(trust(1)).unwrap();
        b.devices.push(b.devices[0].clone());
        assert!(b.validate().is_err());
        b.devices.pop();
        b.favorite = Some("foreign".into());
        assert!(b.validate().is_err());
    }
}
