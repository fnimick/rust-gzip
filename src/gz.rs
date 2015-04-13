#[doc="

    Module: gz

    This provides the Rust interface to gzip decompression.
    It serves a similar function to lib, except it has no
    code to interface with C.

"]
use cvec::{CVec, Buf, Iter};
use libc::c_uint;

use header;
use crc32;
use gz_reader::GzBitReader;
use inflate::inflate;

// every gzip file is at least 10 bytes, if not, it's invalid
const GZIP_MIN_LEN: usize = 40;
const GZIP_FILESIZE_OFFSET: usize = 4;
const GZIP_CRC_OFFSET: usize = 8;
const GZIP_FOOTER_LEN: usize = 8;

/// Decompress the given compressed buffer
pub fn decompress_gz(buffer: Buf) -> Option<Buf> {
    if buffer.len() < GZIP_MIN_LEN {
        return None;
    }
    let out_len = get_uncompressed_len(&buffer);
    let crc = get_crc(&buffer);
    let header = try_opt!(header::parse_header(&buffer));
    let mut out_buf = try_opt!(CVec::with_capacity(out_len));
    decompress_raw(buffer.limit_iter(header.header_len, buffer.len() - GZIP_FOOTER_LEN),
                   &mut out_buf);
    if check_crc(&out_buf, crc) {
        Some(out_buf)
    } else {
        None
    }
}

/////////////////////////////////////////////////////////////////////
//                       Helper functions                          //
/////////////////////////////////////////////////////////////////////

/// Decompress the buffer into out_buf
/// Helper function for decompress
fn decompress_raw(buffer: Iter<u8>, out_buf: &mut Buf) {
    let mut gz_reader = match GzBitReader::new(buffer) {
        Some(g) => g,
        None => { return; }
    };
    match inflate(&mut gz_reader, out_buf) {
        Some(()) => {},
        None => { out_buf.clear(); }
    }
}

/// Get the length of the uncompressed file
fn get_uncompressed_len(buffer: &Buf) -> usize {
    assert!(buffer.len() > GZIP_MIN_LEN);
    buffer.get_wide::<c_uint>(buffer.len() - GZIP_FILESIZE_OFFSET).unwrap() as usize
}

/// Get the CRC of the uncompressed file
fn get_crc(buffer: &Buf) -> c_uint {
    assert!(buffer.len() > GZIP_MIN_LEN);
    buffer.get_wide::<c_uint>(buffer.len() - GZIP_CRC_OFFSET).unwrap()
}

/// Verify that the CRC matches what we expect
fn check_crc(buffer: &Buf, crc: c_uint) -> bool {
    crc32::sum(buffer.iter()) == crc
}

#[cfg(test)]
mod get_tests {
    use super::{get_crc, get_uncompressed_len};
    use cvec::{CVec, Buf};

    fn setup() -> Buf {
        let mut bytes: CVec<u8> = CVec::with_capacity(4).unwrap();
        // 00000001 00000010
        // 00000011 00000100
        for _ in 0..40 {
            bytes.push(1);
        }
        for i in 0..8  {
            bytes.push(i);
        }
        bytes
    }

    #[test]
    fn test_get_crc() {
        let buf: Buf = setup();
        assert_eq!(get_crc(&buf), 0x03020100);
    }

    #[test]
    fn test_get_uncompressed_len() {
        let buf: Buf = setup();
        assert_eq!(get_uncompressed_len(&buf), 0x07060504);
    }
}
