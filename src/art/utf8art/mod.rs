use std::{cmp::min, fmt::Debug, mem::MaybeUninit};

use uninit::uninit_array;

const PREFIX_LEN: usize = 12;
const KEY_ENDMARK: u8 = 0xff; // invalid on utf-8. Thus, use it for preventing that any key cannot be the prefix of another key.
struct NodeHeader {
    len: u32,                              // the len of prefix
    prefix: [MaybeUninit<u8>; PREFIX_LEN], // prefix for path compression
}

impl Debug for NodeHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            f.debug_struct("NodeHeader")
                .field("len", &self.len)
                .field(
                    "prefix",
                    &self
                        .prefix
                        .get_unchecked(..min(PREFIX_LEN, self.len as usize)),
                )
                .finish()
        }
    }
}

impl Default for NodeHeader {
    fn default() -> Self {
        Self {
            len: 0,
            prefix: uninit_array![u8; PREFIX_LEN],
        }
    }
}

/// the child node type
/// This is used for bitflag on child pointer.
const NODETYPE_MASK: usize = 0b111;
#[repr(usize)]
#[derive(Debug, PartialEq)]
enum NodeType {
    Value = 0b000,
    Node4 = 0b001,
    Node16 = 0b010,
    Node48 = 0b011,
    Node256 = 0b100,
}
