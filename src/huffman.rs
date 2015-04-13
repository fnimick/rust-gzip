#[doc="

    Module: huffman

    This modules contains code to create huffman trees from ranges
    as defined in the gzip specification, and read values from a
    bitstream as interpreted by a huffman tree.

"]
use std;
use self::HuffmanNode::{Node, Leaf};
use gz_reader::GzBitReader;

/////////////////////////////////////////////////////////////////////
//                        Structs                                  //
/////////////////////////////////////////////////////////////////////

#[derive(Clone, Show)]
pub struct HuffmanRange {
    pub end: u32,
    pub bit_length: u32,
}

impl HuffmanRange {
    pub fn new() -> HuffmanRange {
        HuffmanRange { end: 0, bit_length: 0 }
    }
}

#[derive(Show, PartialEq)]
pub struct TreeNode {
    pub len: usize,
    pub bits: usize,
    pub label: usize
}

#[derive(Show, PartialEq)]
pub enum HuffmanNode {
    Node(Option<Box<HuffmanNode>>, Option<Box<HuffmanNode>>),
    Leaf(u32)
}

impl HuffmanNode {
    /// Traverse the Huffman Tree by reading sequential bytes
    pub fn read(&self, stream: &mut GzBitReader) -> Option<u32> {
        match self {
            &Leaf(v) => Some(v),
            &Node(ref left, ref right) => {
                let target = match try_opt!(stream.next_bit()) {
                    0 => try_ref_opt!(left),
                    1 => try_ref_opt!(right),
                    _ => { panic!("Bit greater than one, no bueno."); }
                };
                target.read(stream)
            }
        }
    }
}

/////////////////////////////////////////////////////////////////////
//                     Building the tree                           //
/////////////////////////////////////////////////////////////////////

/// Build the Huffman Tree from a set of Huffman Ranges
pub fn build_huffman_tree(ranges: &[HuffmanRange]) -> Option<HuffmanNode> {
    let max_bit_length: usize = try_opt!(ranges.iter()
                                         .map(|x| x.bit_length)
                                         .max()) as usize;
    let bl_count = count_bitlengths(ranges, max_bit_length);
    let mut next_code = compute_first_codes(&bl_count);
    let table: Vec<TreeNode> = compute_code_table(&mut next_code, ranges);
    let tree: HuffmanNode = build_tree(&table);
    Some(tree)
}

/// determine number of codes of each bit-length
/// returns a vector where the index corresponds to (bit_length - 1)
fn count_bitlengths(ranges: &[HuffmanRange], max_bit_length: usize) -> Vec<u32> {
    // Vec of size max_bit_length + 1, initialized to 0
    let mut bl_count: Vec<u32> = std::iter::repeat(0).take(max_bit_length).collect();

    let mut range_iter = ranges.iter();
    let mut old_range: &HuffmanRange = range_iter.next().unwrap();
    {
        if old_range.bit_length > 0 {
            let count_ref = bl_count.get_mut((old_range.bit_length - 1) as usize).unwrap();
            *count_ref += old_range.end + 1;
        }
    }

    for range in range_iter {
        if range.bit_length > 0 {
            let count_ref = bl_count.get_mut((range.bit_length - 1) as usize).unwrap();
            *count_ref += range.end - old_range.end;
        }
        old_range = range;
    }
    bl_count
}

#[cfg(test)]
mod count_bitlengths_tests {
    use super::{HuffmanRange, count_bitlengths};

    macro_rules! range {
        ( $( ($x:expr, $y:expr) ),* ) => {{
            let mut ranges = Vec::new();
            $(
                ranges.push(HuffmanRange {
                    end: $x,
                    bit_length: $y
                });
            )*
            ranges
        }};
    }

    #[test]
    fn test_count_bl() {
        let ranges = range![(1, 4), (4, 6), (6, 4),
                            (14, 5), (18, 6), (21, 4),
                            (26, 6)];
        let expect = vec![0, 0, 0, 7, 8, 12];
        assert_eq!(count_bitlengths(ranges.as_slice(), 6), expect);
    }
}

/// Figure out what the first code for each bit-length would be.
/// This is one more than the last code of the previous bit length,
/// left-shifted once. Returns a vector where the index corresponds
/// to (bit_length - 1)
fn compute_first_codes(bl_count: &Vec<u32>) -> Vec<u32> {
    let mut ret = Vec::new();
    let mut code: u32 = 0;
    // from the RFC
    for bits in (0 .. bl_count.len()) {
        if bits > 0 {
            code = ( code + bl_count[bits - 1] ) << 1;
        }
        ret.push(if bl_count[bits] > 0 { code } else { 0 });
    }
    ret
}

#[cfg(test)]
mod compute_first_codes_tests {
    use super::compute_first_codes;

    #[test]
    fn test_compute_codes() {
        let input = vec![0, 0, 0, 7, 8, 12];
        let expect = vec![0, 0, 0, 0, 14, 44];
        assert_eq!(compute_first_codes(&input), expect);
    }

    #[test]
    fn test_1_bit_codes() {
        let input = vec![1, 1, 1, 1, 0, 4];
        let expect = vec![0, 2, 6, 14, 0, 60];
        assert_eq!(compute_first_codes(&input), expect);
    }
}

/// Assign codes to each symbol in the each range of a given bitlength
fn compute_code_table(next_code: &mut Vec<u32>, ranges: &[HuffmanRange])
        -> Vec<TreeNode> {
    let mut ret = Vec::new();
    let mut active_range: usize = 0;
    let num_entries = ranges.get(ranges.len() - 1).unwrap().end;
    for n in 0 .. num_entries + 1 {
        if n > ranges[active_range].end {
            active_range += 1;
        }
        let bit_length = ranges[active_range].bit_length as usize;
        if bit_length > 0 {
            ret.push(TreeNode {
                len: bit_length,
                bits: next_code[bit_length - 1] as usize,
                label: n as usize
            });
            *next_code.get_mut(bit_length - 1).unwrap() += 1;
        }
    }
    ret
}

#[cfg(test)]
mod compute_code_table_tests {
    use super::{TreeNode, HuffmanRange, compute_code_table};

    macro_rules! range {
        ( $( ($x:expr, $y:expr) ),* ) => {{
            let mut ranges = Vec::new();
            $(
                ranges.push(HuffmanRange {
                    end: $x,
                    bit_length: $y
                });
            )*
            ranges
        }};
    }

    macro_rules! nodes {
        ( $( ($x:expr, $y:expr) ),* ) => {{
            let mut nodes = Vec::new();
            let mut count = -1;
            $(
                count += 1;
                nodes.push(TreeNode {
                    len: $x,
                    bits: $y,
                    label: count
                });
            )*
            nodes
        }};
    }

    #[test]
    fn test_compute_code_table() {
        let mut next_code = vec![0, 0, 0, 0, 14, 44];
        let ranges = range![(1, 4), (4, 6), (6, 4),
                            (14, 5), (18, 6), (21, 4),
                            (26, 6)];
        let expect = nodes![(4, 0), (4, 1), (6, 44), (6, 45), (6, 46),
                            (4, 2), (4, 3), (5, 14), (5, 15), (5, 16),
                            (5, 17), (5, 18), (5, 19), (5, 20), (5, 21),
                            (6, 47), (6, 48), (6, 49), (6, 50), (4, 4),
                            (4, 5), (4, 6), (6, 51), (6, 52), (6, 53),
                            (6, 54), (6, 55)];
        assert_eq!(compute_code_table(&mut next_code, ranges.as_slice()), expect);
    }
}

/// Create the Huffman tree from the code table
fn build_tree(code_table: &Vec<TreeNode>) -> HuffmanNode {
    let mut root = Node(None, None);
    for t_node in code_table.iter() {
        let bits = t_node.bits;
        let len = (t_node.len - 1) as isize;
        let label = t_node.label;
        make_tree(&mut root, bits, len, label);
    }
    root
}

#[cfg(test)]
mod build_tree_tests {
    use super::{build_tree, TreeNode};
    use super::HuffmanNode::{Node, Leaf};

    #[test]
    fn test_build_tree() {
        let input = vec![TreeNode {
            len: 4,
            bits: 5, // 0101
            label: 0
        }];
        assert_eq!(build_tree(&input), Node(
            Some(box Node(
                    None,
                    Some(box Node(
                            Some(box Node(None,
                                      Some(box Leaf(0)))),
                            None)))),
            None));

    }
}

/// Helper function for build_tree
fn make_tree(tree: &mut HuffmanNode, bits: usize, len: isize, label: usize) {
    match tree {
        &mut Leaf(_) => {
            panic!("This shouldn't have happened.");
        },
        &mut Node(ref mut left, ref mut right) => {
            match get_bit(bits, len as usize) {
                0 => { make_tree_side(left, bits, len - 1, label); },
                1 => { make_tree_side(right, bits, len - 1, label); },
                _ => { panic!("A bit was greater than 1, this is bad."); }
            }
        }
    }
}

/// Make one side of the tree
fn make_tree_side(t_side: &mut Option<Box<HuffmanNode>>, bits: usize, len: isize, value: usize) {
    match t_side {
        &mut None => { *t_side = Some(box make_new_tree(bits, len, value)); },
        &mut Some(ref mut t) => { make_tree(&mut **t, bits, len, value); },
    };
}

/// Create a new HuffmanNode based on the next set of bits to read
fn make_new_tree(bits: usize, len: isize, value: usize) -> HuffmanNode {
    if len < 0 {
        Leaf(value as u32)
    } else {
        match get_bit(bits, len as usize) {
            0 => Node(Some(box make_new_tree(bits, len - 1, value)), None),
            1 => Node(None, Some(box make_new_tree(bits, len - 1, value))),
            _ => { panic!("A bit was greater than 1, this is bad."); }
        }
    }
}

/// gets 'index' bit of input
fn get_bit(input: usize, index: usize) -> usize {
    if (input & (1 << index)) > 0 { 1 } else { 0 }
}

#[cfg(test)]
mod get_bit_tests {
    use super::get_bit;

    #[test]
    fn test_get_bit() {
        assert_eq!(get_bit(0x3, 0), 1);
        assert_eq!(get_bit(0x3, 1), 1);
        assert_eq!(get_bit(0x3, 2), 0);
        assert_eq!(get_bit(0x3, 3), 0);
    }
}
