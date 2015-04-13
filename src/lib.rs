#![allow(unstable)]
#![feature(unsafe_destructor)]
#![feature(box_syntax)]

#[doc="

    module: lib

    This is the wrapper library for GZip decompression.
    It provides the interface to a C program. The C
    program is responsible for passing in the pointer to
    a gzip-compressed buffer, as well as its length.
    This library will return a pointer to a malloc'd
    buffer representing the decompressed contents of the
    original buffer.

"]

extern crate libc;

use libc::{c_int, c_uchar, c_void};
use std::ptr::null;
use cvec::CVec;

#[macro_use]
mod macros;
mod cvec;
mod gz;
mod header;
mod crc32;
mod inflate;
mod huffman;
mod gz_reader;

/////////////////////////////////////////////////////////////////////
//                   Decompression interface                       //
/////////////////////////////////////////////////////////////////////

/// The main decompression function
/// Assumption: The Vec given to this function is a gzipped buffer

#[no_mangle]
pub extern "C" fn decompress_gzip_to_heap(buf: *const c_void,
                                          buf_len: c_int,
                                          decompressed_len: *mut c_int)
        -> *mut c_void {
    let in_vec = try_bail!(unsafe { CVec::from_raw_buf(buf as *const c_uchar, buf_len as usize)});
    let out_vec = try_bail!(gz::decompress_gz(in_vec));
    unsafe {
        let (out_ptr, out_size) = out_vec.into_raw_buf();
        *decompressed_len = out_size as c_int;
        out_ptr as *mut c_void
    }
}

