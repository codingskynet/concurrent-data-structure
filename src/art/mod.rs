use std::{
    marker::PhantomData,
    mem,
};

use either::Either;

use crate::map::SequentialMap;

struct NodeHeader {
    len: u8,          // the len of prefix
    prefix: [u8; 15], // prefix for path compression
}

/// the child node type
/// This is used for bitflag on child pointer.
const NODETYPE_MASK: usize = 0b111;
#[repr(usize)]
enum NodeType {
    Value = 0b000,
    Node4 = 0b001,
    Node16 = 0b010,
    Node48 = 0b011,
    Node256 = 0b100,
}

trait NodeOps<V> {
    fn insert(&mut self, key: u8, node: Node<V>);
    fn lookup(&self, key: u8) -> &Node<V>;
    fn remove(&mut self, key: u8) -> Node<V>;
}

/// the pointer struct for Nodes or value
struct Node<V> {
    pointer: usize,
    _marker: PhantomData<Box<V>>,
}

impl<V> Node<V> {
    fn deref(&self) -> Either<&dyn NodeOps<V>, &V> {
        unsafe {
            let pointer = self.pointer & !NODETYPE_MASK;
            let tag = mem::transmute(self.pointer & NODETYPE_MASK);

            match tag {
                NodeType::Value => Either::Right(&*(pointer as *const V)),
                NodeType::Node4 => Either::Left(&*(pointer as *const Node4<V>)),
                NodeType::Node16 => Either::Left(&*(pointer as *const Node16<V>)),
                NodeType::Node48 => Either::Left(&*(pointer as *const Node48<V>)),
                NodeType::Node256 => Either::Left(&*(pointer as *const Node256<V>)),
            }
        }
    }
}

struct Node4<V> {
    header: NodeHeader,
    keys: [u8; 4],
    children: [Node<V>; 4],
}

impl<V> NodeOps<V> for Node4<V> {}

struct Node16<V> {
    header: NodeHeader,
    keys: [u8; 16],
    children: [Node<V>; 16],
}

impl<V> NodeOps<V> for Node16<V> {}

struct Node48<V> {
    header: NodeHeader,
    keys: [u8; 256],
    children: [Node<V>; 48],
}

impl<V> NodeOps<V> for Node48<V> {}

struct Node256<V> {
    header: NodeHeader,
    children: [Node<V>; 256],
}

impl<V> NodeOps<V> for Node256<V> {}

pub struct ART<K, V> {
    root: Box<Node<V>>,
    _marker: PhantomData<K>,
}

impl<K, V> ART<K, V> {

}

impl<K: Eq, V> SequentialMap<K, V> for ART<K, V> {
    fn new() -> Self {
        todo!()
    }

    fn insert(&mut self, key: &K, value: V) -> Result<(), V> {
        todo!()
    }

    fn lookup(&self, key: &K) -> Option<&V> {
        todo!()
    }

    fn remove(&mut self, key: &K) -> Result<V, ()> {
        todo!()
    }
}
