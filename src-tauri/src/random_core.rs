//! Security-sensitive randomness comes directly from the OS and fails closed.
use rand::{rngs::SysRng, TryCryptoRng, TryRng};

const UNAVAILABLE: &str = "Windows no pudo generar datos seguros. Volvé a intentarlo.";
const CODE_SPACE: u32 = 1_000_000;
// Largest multiple of CODE_SPACE below 2^32. Reject the remainder to avoid bias.
const CODE_CEILING: u32 = u32::MAX - u32::MAX % CODE_SPACE;

pub fn token() -> Result<u64, String> { token_with(&mut SysRng) }
pub fn key() -> Result<[u8; 32], String> { key_with(&mut SysRng) }
pub fn pairing_code() -> Result<String, String> { code_with(&mut SysRng) }

fn token_with(rng: &mut impl TryCryptoRng) -> Result<u64, String> {
    rng.try_next_u64().map_err(|_| UNAVAILABLE.to_owned())
}
fn key_with(rng: &mut impl TryCryptoRng) -> Result<[u8; 32], String> {
    let mut bytes = [0; 32];
    rng.try_fill_bytes(&mut bytes).map_err(|_| UNAVAILABLE.to_owned())?;
    Ok(bytes)
}
fn code_with(rng: &mut impl TryCryptoRng) -> Result<String, String> {
    loop {
        let value = rng.try_next_u32().map_err(|_| UNAVAILABLE.to_owned())?;
        if value < CODE_CEILING { return Ok(format!("{:06}", value % CODE_SPACE)); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::VecDeque, io};

    // Deterministic source only for boundary/error tests; never used by callers.
    struct Source(VecDeque<u32>);
    impl Source { fn new(values: &[u32]) -> Self { Self(values.iter().copied().collect()) } }
    impl TryRng for Source {
        type Error = io::Error;
        fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
            self.0.pop_front().ok_or_else(|| io::Error::other("test source exhausted"))
        }
        fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
            Ok((u64::from(self.try_next_u32()?) << 32) | u64::from(self.try_next_u32()?))
        }
        fn try_fill_bytes(&mut self, bytes: &mut [u8]) -> Result<(), Self::Error> {
            for chunk in bytes.chunks_mut(4) {
                let word = self.try_next_u32()?.to_le_bytes();
                chunk.copy_from_slice(&word[..chunk.len()]);
            }
            Ok(())
        }
    }
    impl TryCryptoRng for Source {}

    #[test]
    fn os_source_generates_values_for_the_existing_protocol() {
        let code = pairing_code().expect("OS entropy must be available");
        assert_eq!(code.len(), 6);
        assert!(code.bytes().all(|byte| byte.is_ascii_digit()));
        assert!(token().is_ok());
        assert!(key().is_ok());
    }
    #[test]
    fn six_digit_codes_keep_leading_zeroes_and_the_full_range() {
        for (value, expected) in [(0, "000000"), (7, "000007"), (999_999, "999999"), (1_000_000, "000000"), (CODE_CEILING - 1, "999999")] {
            assert_eq!(code_with(&mut Source::new(&[value])).unwrap(), expected);
        }
    }
    #[test]
    fn code_generation_rejects_the_biased_remainder() {
        let mut source = Source::new(&[CODE_CEILING, u32::MAX, 123_456]);
        assert_eq!(code_with(&mut source).unwrap(), "123456");
        assert!(source.0.is_empty());
    }
    #[test]
    fn source_failure_never_returns_a_code_token_or_partial_key() {
        assert_eq!(code_with(&mut Source::new(&[u32::MAX])).unwrap_err(), UNAVAILABLE);
        assert_eq!(token_with(&mut Source::new(&[1])).unwrap_err(), UNAVAILABLE);
        assert_eq!(key_with(&mut Source::new(&[1, 2])).unwrap_err(), UNAVAILABLE);
    }
    #[test]
    fn stored_secret_and_transfer_token_keep_their_existing_sizes() {
        assert_eq!(key_with(&mut Source::new(&[0x12345678; 8])).unwrap(), [0x78, 0x56, 0x34, 0x12].repeat(8).as_slice());
        assert_eq!(token_with(&mut Source::new(&[0x12345678, 0x9abcdef0])).unwrap(), 0x123456789abcdef0);
    }
}
