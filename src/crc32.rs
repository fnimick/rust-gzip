#[doc="

    Module: crc32

    This module handles verifying the CRC in the GZip file

"]
use cvec;

const IEEE: u32 = 0xedb88320;

/// Cyclic Redundancy Check
struct Crc32 {
    table: [u32; 256],
    value: u32
}

impl Crc32 {
    /// Setup the CRC
    fn new() -> Crc32 {
        let mut c = Crc32 { table: [0; 256], value: 0xffffffff };
        for i in 0 .. 256 {
            let mut v = i as u32;
            for _ in 0 .. 8 {
                v = if v & 1 != 0 {
                    IEEE ^ (v >> 1)
                } else {
                    v >> 1
                }
            }
            c.table[i] = v;
        }
        c
    }

    /// Create the CRC for the given buffer
    fn sum(&mut self, mut buf: cvec::Iter<u8>) -> u32 {
        for &i in buf {
            self.value = self.table[((self.value ^ (i as u32)) & 0xFF) as usize] ^
                (self.value >> 8);
        }
        self.value ^ 0xffffffff
    }
}

/// Public interface for using the CRC
pub fn sum(buf: cvec::Iter<u8>) -> u32 {
    let mut c = Crc32::new();
    c.sum(buf)
}
