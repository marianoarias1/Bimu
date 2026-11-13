//! Shell bitmaps use BGRA, sometimes with premultiplied alpha.
pub fn rgba(width: usize, height: usize, bytes: &[u8]) -> Option<Vec<u8>> {
    if width == 0
        || height == 0
        || width > 160
        || height > 120
        || width.checked_mul(height)?.checked_mul(4)? != bytes.len()
    {
        return None;
    }
    let has_alpha = bytes.chunks_exact(4).any(|p| p[3] != 0);
    let mut out = Vec::with_capacity(bytes.len());
    for p in bytes.chunks_exact(4) {
        let alpha = if has_alpha { p[3] } else { 255 };
        let color = |c: u8| {
            if alpha == 0 {
                0
            } else {
                ((u16::from(c) * 255 + u16::from(alpha) / 2) / u16::from(alpha)).min(255) as u8
            }
        };
        out.extend_from_slice(&[color(p[2]), color(p[1]), color(p[0]), alpha]);
    }
    Some(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn converts_channel_order_and_preserves_transparency() {
        assert_eq!(
            rgba(2, 1, &[10, 20, 30, 255, 20, 30, 40, 128]).unwrap(),
            [30, 20, 10, 255, 80, 60, 40, 128]
        );
        assert_eq!(rgba(1, 1, &[10, 20, 30, 0]).unwrap(), [30, 20, 10, 255]);
    }
    #[test]
    fn refuses_large_or_truncated_bitmaps() {
        assert!(rgba(161, 1, &[]).is_none());
        assert!(rgba(1, 121, &[]).is_none());
        assert!(rgba(1, 1, &[0; 3]).is_none());
    }
}
