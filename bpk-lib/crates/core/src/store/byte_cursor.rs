//! One little-endian byte cursor shared by the fixed-width binary entry codecs.
//!
//! The mmap cold-start index entries and the segment SIDX entries both decode
//! fixed-size records field by field, advancing a `pos` cursor. Each used to
//! carry its own drifted `get_le!` / `get_hash!` macro; both now delegate the
//! actual bounds-checked, cursor-advancing read to [`take`] so the read logic
//! lives in exactly one place.

/// Read `N` little-endian bytes from `buf` at `*pos`, advancing the cursor by
/// `N`.
///
/// Panics only if `buf` is shorter than `*pos + N`. Every caller validates the
/// whole record length up front (the record is a fixed size), so the panic is a
/// defensive invariant, never a reachable path on well-formed input — the same
/// contract the two hand-rolled macros carried before consolidation.
pub(crate) fn take<const N: usize>(buf: &[u8], pos: &mut usize) -> [u8; N] {
    let mut arr = [0u8; N];
    arr.copy_from_slice(&buf[*pos..*pos + N]);
    *pos += N;
    arr
}

#[cfg(test)]
mod tests {
    use super::take;

    #[test]
    fn take_advances_cursor_and_reads_little_endian() {
        let buf = [0x01, 0x00, 0x00, 0x00, 0xff, 0xff];
        let mut pos = 0usize;
        assert_eq!(u32::from_le_bytes(take::<4>(&buf, &mut pos)), 1);
        assert_eq!(pos, 4);
        assert_eq!(u16::from_le_bytes(take::<2>(&buf, &mut pos)), 0xffff);
        assert_eq!(pos, 6);
    }

    #[test]
    fn take_reads_a_32_byte_hash_window() {
        let mut buf = [0u8; 40];
        buf[8..40].copy_from_slice(&[7u8; 32]);
        let mut pos = 8usize;
        let hash: [u8; 32] = take::<32>(&buf, &mut pos);
        assert_eq!(hash, [7u8; 32]);
        assert_eq!(pos, 40);
    }
}
