//! Bound allocation while reading, not after loading an entire untrusted file.
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};
pub fn read_limited(path: &Path, max: usize) -> io::Result<Vec<u8>> {
    let file = File::open(path)?;
    if file.metadata()?.len() > max as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "file exceeds limit",
        ));
    }
    read_from(file, max)
}
fn read_from(reader: impl Read, max: usize) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take((max as u64).saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() > max {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "file exceeds limit",
        ));
    }
    Ok(bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_limit_and_empty_files_are_valid_but_an_extra_byte_is_not() {
        assert_eq!(read_from(&b"abc"[..], 3).unwrap(), b"abc");
        assert!(read_from(&b"abcd"[..], 3).is_err());
        assert!(read_from(&b""[..], 0).unwrap().is_empty());
    }
    #[test]
    fn stops_reading_a_growing_or_unbounded_source() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        struct Infinite(Arc<AtomicUsize>);
        impl Read for Infinite {
            fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
                out.fill(1);
                self.0.fetch_add(out.len(), Ordering::SeqCst);
                Ok(out.len())
            }
        }
        let count = Arc::new(AtomicUsize::new(0));
        assert!(read_from(Infinite(count.clone()), 128).is_err());
        assert_eq!(count.load(Ordering::SeqCst), 129);
    }
}
