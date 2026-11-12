//! Cross-version regression: new HMAC/SHA-256 must preserve the deployed protocol.
use bimu_native_core_tests::pairing_core as current;
use hmac::{Hmac, KeyInit, Mac};
use sha2_hmac::{Digest, Sha256};

#[path = "fixtures/pairing_core_0_55_15.rs"]
mod legacy;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn hmac_sha256_preserves_published_vectors() {
    // RFC 4231 cases 1, 2 and 6 (including a key larger than SHA-256's block).
    for (key, message, expected) in [
        (vec![0x0b; 20], b"Hi There".as_slice(), "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"),
        (b"Jefe".to_vec(), b"what do ya want for nothing?".as_slice(), "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"),
        (vec![0xaa; 131], b"Test Using Larger Than Block-Size Key - Hash Key First".as_slice(), "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"),
    ] {
        let mut mac = Hmac::<Sha256>::new_from_slice(&key).unwrap();
        mac.update(message);
        assert_eq!(hex(&mac.finalize().into_bytes()), expected);
    }
}

#[test]
fn sha256_preserves_streamed_hashes_and_noise_key_derivation() {
    use sha2::Digest as LegacyDigest;
    for data in [vec![], b"abc".to_vec(), vec![0x5a; 65_537]] {
        let mut streamed = Sha256::new();
        for chunk in data.chunks(257) { streamed.update(chunk); }
        assert_eq!(streamed.finalize().as_slice(), sha2::Sha256::digest(&data).as_slice());
    }
    assert_eq!(hex(&Sha256::digest(b"abc")), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    let mut new_psk = Sha256::new();
    let mut old_psk = sha2::Sha256::new();
    for bytes in [b"bimu-noise-psk-v1\0".as_slice(), &[0x5a; 32]] {
        new_psk.update(bytes);
        old_psk.update(bytes);
    }
    assert_eq!(new_psk.finalize().as_slice(), old_psk.finalize().as_slice());
}

#[test]
fn new_host_pairs_with_previous_client() {
    let (host, host_message) = current::start(current::Role::Host, "001234").unwrap();
    let (client, client_message) = legacy::start(legacy::Role::Client, "001234").unwrap();
    let (host, host_tag) = host.finish(&client_message).unwrap();
    let (client, client_tag) = client.finish(&host_message).unwrap();
    assert_eq!(host_tag.len(), 32);
    assert_eq!(host.verify(&client_tag).unwrap().as_bytes(), client.verify(&host_tag).unwrap().as_bytes());
}

#[test]
fn previous_host_pairs_with_new_client() {
    let (host, host_message) = legacy::start(legacy::Role::Host, "001234").unwrap();
    let (client, client_message) = current::start(current::Role::Client, "001234").unwrap();
    let (host, host_tag) = host.finish(&client_message).unwrap();
    let (client, client_tag) = client.finish(&host_message).unwrap();
    assert_eq!(client_tag.len(), 32);
    assert_eq!(host.verify(&client_tag).unwrap().as_bytes(), client.verify(&host_tag).unwrap().as_bytes());
}

#[test]
fn mixed_versions_reject_wrong_codes_in_both_directions() {
    for (new_role, old_role) in [
        (current::Role::Host, legacy::Role::Client),
        (current::Role::Client, legacy::Role::Host),
    ] {
        let (new, new_message) = current::start(new_role, "001234").unwrap();
        let (old, old_message) = legacy::start(old_role, "001235").unwrap();
        let (new, new_tag) = new.finish(&old_message).unwrap();
        let (old, old_tag) = old.finish(&new_message).unwrap();
        assert!(new.verify(&old_tag).is_err());
        assert!(old.verify(&new_tag).is_err());
    }
}

#[test]
fn mixed_versions_reject_modified_truncated_and_reflected_confirmations() {
    for (new_role, old_role) in [
        (current::Role::Host, legacy::Role::Client),
        (current::Role::Client, legacy::Role::Host),
    ] {
        for invalid in ["modified", "truncated", "reflected"] {
            let (new, new_message) = current::start(new_role, "001234").unwrap();
            let (old, old_message) = legacy::start(old_role, "001234").unwrap();
            let (new, new_tag) = new.finish(&old_message).unwrap();
            let (old, old_tag) = old.finish(&new_message).unwrap();
            let (mut to_new, mut to_old) = (old_tag.clone(), new_tag.clone());
            match invalid {
                "modified" => { to_new[0] ^= 1; to_old[0] ^= 1; }
                "truncated" => { to_new.pop(); to_old.pop(); }
                _ => { to_new = new_tag; to_old = old_tag; }
            }
            assert!(new.verify(&to_new).is_err(), "{invalid}");
            assert!(old.verify(&to_old).is_err(), "{invalid}");
        }
    }
}
