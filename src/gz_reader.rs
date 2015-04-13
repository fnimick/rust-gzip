#[doc="
    Module: gz_reader

    This module provides an abstraction over the 'bit stream'
    of a gzip-compressed buffer.

"]
use cvec::Iter;

#[derive(Show)]
pub struct GzBitReader<'a> {
    iter: Iter<'a, u8>,
    buf: u8,
    mask: u8
}

/// Read the GZIP data bit by bit
impl<'a> GzBitReader<'a> {
    pub fn new(mut iter: Iter<'a, u8>) -> Option<GzBitReader<'a>> {
        let starting_buf = try_opt!(iter.next());
        Some(GzBitReader {
            iter: iter,
            buf: *starting_buf,
            mask: 0x01
        })
    }

    #[inline]
    /// Get the next bit from the "stream"
    pub fn next_bit(&mut self) -> Option<u32> {
        if self.mask == 0 {
            self.buf = *try_opt!(self.iter.next());
            self.mask = 0x01;
        }
        let bit = if (self.buf & self.mask) > 0 { 1 } else { 0 };
        self.mask <<= 1;
        Some(bit)
    }

    /// reads bits in least to most significant order
    pub fn read_bits(&mut self, count: u32) -> Option<u32> {
        let mut bit: u32;
        let mut value: u32 = 0;
        for i in (0 .. count) {
            bit = try_opt!(self.next_bit());
            value |= bit << i;
        }
        Some(value)
    }

    /// reads bits in most to least significant order
    pub fn read_bits_rev(&mut self, count: u32) -> Option<u32> {
        let mut bit: u32;
        let mut value: u32 = 0;
        for _ in (0 .. count) {
            value <<= 1;
            bit = try_opt!(self.next_bit());
            value |= bit;
        }
        Some(value)
    }
}

#[cfg(test)]
mod gz_reader_tests {
    use super::GzBitReader;
    use cvec::CVec;

    fn setup() -> CVec<u8> {
        let mut bytes: CVec<u8> = CVec::with_capacity(4).unwrap();
        // 00000001 00000010
        // 00000011 00000100
        bytes.push(1);
        bytes.push(2);
        bytes.push(3);
        bytes.push(4);
        bytes
    }

    #[test]
    fn test_read_bits() {
        let bytes = setup();
        let mut reader = GzBitReader::new(bytes.iter()).unwrap();
        assert_eq!(reader.read_bits(9), Some(1));
        assert_eq!(reader.read_bits(9), Some(385));
    }

    #[test]
    fn test_read_bits_rev() {
        let bytes = setup();
        let mut reader = GzBitReader::new(bytes.iter()).unwrap();
        assert_eq!(reader.read_bits_rev(9), Some(256));
        assert_eq!(reader.read_bits_rev(9), Some(259));
    }

    #[test]
    fn test_next_bit() {
        let bytes = setup();
        let mut reader = GzBitReader::new(bytes.iter()).unwrap();
        assert_eq!(reader.next_bit(), Some(1));
        for _ in 0..8 {
            assert_eq!(reader.next_bit(), Some(0));
        }
        assert_eq!(reader.next_bit(), Some(1));
        for _ in 0..6 {
            assert_eq!(reader.next_bit(), Some(0));
        }
        assert_eq!(reader.next_bit(), Some(1));
        assert_eq!(reader.next_bit(), Some(1));
        for _ in 0..8 {
            assert_eq!(reader.next_bit(), Some(0));
        }
        assert_eq!(reader.next_bit(), Some(1));
        for _ in 0..5 {
            assert_eq!(reader.next_bit(), Some(0));
        }
        assert_eq!(reader.next_bit(), None);
    }
}
