#[doc="

    Module: inflate

    This module contains the bulk of the heavy lifting involved
    in decompressing a gzip buffer. It parses the buffer to
    generate the huffman trees embedded in it, and then uses
    those huffman trees to decode the gzip into a buffer.

"]
use gz_reader::GzBitReader;
use cvec::Buf;
use huffman::{HuffmanNode, HuffmanRange};
use huffman::build_huffman_tree;

// These constants are defined by the GZIP standard
static CODE_LENGTH_OFFSETS: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];
static EXTRA_LENGTH_ADDEND: [usize; 20] = [
    11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227];
static EXTRA_DIST_ADDEND: [usize; 26] = [
    4, 6, 8, 12, 16, 24, 32, 48, 64, 96, 128, 192, 256, 384, 512, 768, 1024, 1536, 2048,
    3072, 4096, 6144, 8192, 12288, 16384, 24576];
static FIXED_TREE_RANGES: [HuffmanRange; 4] = [
    HuffmanRange { end: 143, bit_length: 8},
    HuffmanRange { end: 255, bit_length: 9},
    HuffmanRange { end: 279, bit_length: 7},
    HuffmanRange { end: 287, bit_length: 8}];


/////////////////////////////////////////////////////////////////////
//                  Tree Reading                                   //
/////////////////////////////////////////////////////////////////////

/// Builds the first tree from a gzip block header, used to encode
/// the following literals and distance tree
fn build_code_length_tree(stream: &mut GzBitReader, hclen: u32)
    -> Option<HuffmanNode>
{
    let mut code_length_ranges = Vec::new();
    let mut code_lengths = [0u32; 19];

    for i in 0 .. (hclen + 4) as usize {
        code_lengths[CODE_LENGTH_OFFSETS[i]] = try_opt!(stream.read_bits(3));
    }

    // make these ranges for the huffman tree routine
    let mut range = HuffmanRange::new();
    for i in (0 .. 19) {
        if i > 0 && code_lengths[i] != code_lengths[i-1] {
            code_length_ranges.push(range.clone());
        }
        range.end = i as u32;
        range.bit_length = code_lengths[i];
    }
    code_length_ranges.push(range.clone());
    build_huffman_tree(code_length_ranges.as_slice())
}

/// Reads a huffman tree from a GzBitReader and returns two trees:
/// the first is the literals tree, and the second is the distances tree
fn read_huffman_tree(stream: &mut GzBitReader) -> Option<(HuffmanNode, HuffmanNode)> {
    let hlit = try_opt!(stream.read_bits(5));
    let hdist = try_opt!(stream.read_bits(5));
    let hclen = try_opt!(stream.read_bits(4)); // max of 15

    let code_lengths_root = try_opt!(build_code_length_tree(stream, hclen));

    // now we read the literal/length alphabet, encoded with the huffman tree
    // we just built
    let mut i = 0;
    let mut alphabet: Vec<u32> = Vec::new();
    while i < (hlit + hdist + 258) {
        let code = try_opt!(code_lengths_root.read(stream));
        if code > 15 {
            let mut repeat_length = {
                if code == 16 {
                    try_opt!(stream.read_bits(2)) + 3
                } else if code == 17 {
                    try_opt!(stream.read_bits(3)) + 3
                } else if code == 18 {
                    try_opt!(stream.read_bits(7)) + 11
                } else { panic!("invalid code"); }
            } as i32;
            while repeat_length > 0 {
                if code == 16 {
                    let prev = *try_opt!(alphabet.get((i-1) as usize));
                    alphabet.push(prev);
                } else {
                    alphabet.push(0);
                }
                i += 1;
                repeat_length -= 1;
            }
        } else {
            alphabet.push(code);
            i += 1;
        }
    }

    // now alphabet lenths have been read, turn these into a range declaration and build
    // the final huffman code from it
    let mut range = HuffmanRange::new();
    let mut literals_ranges = Vec::new();
    for i in 0 .. (hlit + 257) as usize {
        if i > 0 && alphabet[i] != alphabet[i-1] {
            literals_ranges.push(range.clone());
        }
        range.end = i as u32;
        range.bit_length = alphabet[i];
    };
    literals_ranges.push(range.clone());

    let mut distances_ranges = Vec::new();
    let dist_start = hlit + 257;
    for i in dist_start as usize .. (hdist + dist_start + 1) as usize {
        if i > dist_start as usize && alphabet[i] != alphabet[i-1] {
            distances_ranges.push(range.clone());
        }
        range.end = i as u32 - dist_start;
        range.bit_length = alphabet[i];
    }
    distances_ranges.push(range);

    let literals_root = try_opt!(build_huffman_tree(literals_ranges.as_slice()));
    let distances_root = try_opt!(build_huffman_tree(distances_ranges.as_slice()));
    Some((literals_root, distances_root))
}

/// Create the fixed HuffmanTree (per the spec)
fn build_fixed_huffman_tree() -> Option<HuffmanNode> {
    build_huffman_tree(&FIXED_TREE_RANGES)
}

/////////////////////////////////////////////////////////////////////
//                    Inflating the data                           //
/////////////////////////////////////////////////////////////////////

/// Inflate the data segment based on the given Huffman Trees
/// Effect: the output will be stored in out
/// Success on a Some(()) result, failure on a None result
fn inflate_huffman_codes(stream: &mut GzBitReader,
                         literals_root: &HuffmanNode,
                         distances_root: Option<&HuffmanNode>,
                         out: &mut Buf)
        -> Option<()> {
    while let Some(code) = literals_root.read(stream) {
        if code >= 286 {
            return None;
        }
        if code < 256 {
            out.push(code as u8);
        } else if code == 256 { //stop code
            break;
        } else if code > 256 {
            let length = if code < 265 {
                code - 254
            } else {
                if code < 285 {
                    let extra_bits = try_opt!(stream.read_bits((code - 261) / 4));
                    extra_bits + EXTRA_LENGTH_ADDEND[((code - 266) + 1) as usize] as u32
                } else { 258 }
            };

            // now, the length is followed by the distance back
            let mut dist = match distances_root {
                None => {
                    try_opt!(stream.read_bits_rev(5)) // hardcoded distance
                },
                Some(distance_tree) => {
                    try_opt!(distance_tree.read(stream))
                }
            };

            if dist > 3 {
                let extra_dist = try_opt!(stream.read_bits((dist - 2) / 2));
                dist = extra_dist + EXTRA_DIST_ADDEND[(dist - 4) as usize] as u32;

            }
            out.copy_back_pointer(dist as usize, length as usize);
        }
    }
    Some(())
}

/// Inflate the given compressed stream into the out buffer
/// inflate() should be called with a GzBitReader starting at the head
/// of the first block
pub fn inflate(stream: &mut GzBitReader, out: &mut Buf) -> Option<()> {
    let fixed_tree = try_opt!(build_fixed_huffman_tree());
    let mut last_block = 0;
    while { last_block == 0 } {
        last_block = try_opt!(stream.next_bit());
        let block_format = try_opt!(stream.read_bits(2));
        match block_format {
            0x00 => {
                // uncompressed block type, not supported
                return None;
            },
            0x01 => {
                // fixed tree
                try_opt!(inflate_huffman_codes(stream, &fixed_tree, None, out));
            },
            0x02 => {
                // dynamic tree
                let (literals_tree, distances_tree) = try_opt!(read_huffman_tree(stream));
                try_opt!(inflate_huffman_codes(stream, &literals_tree, Some(&distances_tree), out));
            }
            _ => {
                println!("unsupported block");
                // unsupported block type
                return None;
            }
        }
    }
    Some(())
}
