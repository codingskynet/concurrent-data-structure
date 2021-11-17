use std::{
    cmp::Ordering,
    marker::PhantomData,
    mem,
    ptr::{self, NonNull},
};

use either::Either;

use crate::{map::SequentialMap, util::slice_insert};

struct NodeHeader {
    len: u32,         // the len of prefix
    prefix: [u8; 12], // prefix for path compression
}

impl Default for NodeHeader {
    #[allow(deprecated)]
    fn default() -> Self {
        unsafe {
            Self {
                len: 0,
                prefix: mem::uninitialized(),
            }
        }
    }
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
    fn is_full(&self) -> bool;
    fn is_shrinkable(&self) -> bool;
    fn insert(&mut self, key: u8, node: Node<V>) -> Result<(), Node<V>>;
    fn lookup(&self, key: u8) -> Option<&Node<V>>;
    fn update(&mut self, key: u8, node: Node<V>) -> Result<Node<V>, Node<V>>;
    fn remove(&mut self, key: u8) -> Result<Node<V>, ()>;
}

/// the pointer struct for Nodes or value
struct Node<V> {
    pointer: usize,
    _marker: PhantomData<Box<V>>,
}

impl<V> Node<V> {
    fn deref(&self) -> Either<&dyn NodeOps<V>, &NodeV<V>> {
        unsafe {
            let pointer = self.pointer & !NODETYPE_MASK;
            let tag = mem::transmute(self.pointer & NODETYPE_MASK);

            match tag {
                NodeType::Value => Either::Right(&*(pointer as *const NodeV<V>)),
                NodeType::Node4 => Either::Left(&*(pointer as *const Node4<V>)),
                NodeType::Node16 => Either::Left(&*(pointer as *const Node16<V>)),
                NodeType::Node48 => Either::Left(&*(pointer as *const Node48<V>)),
                NodeType::Node256 => Either::Left(&*(pointer as *const Node256<V>)),
            }
        }
    }

    fn deref_mut(&mut self) -> Either<&mut dyn NodeOps<V>, &mut NodeV<V>> {
        unsafe {
            let pointer = self.pointer & !NODETYPE_MASK;
            let tag = mem::transmute(self.pointer & NODETYPE_MASK);

            match tag {
                NodeType::Value => Either::Right(&mut *(pointer as *mut NodeV<V>)),
                NodeType::Node4 => Either::Left(&mut *(pointer as *mut Node4<V>)),
                NodeType::Node16 => Either::Left(&mut *(pointer as *mut Node16<V>)),
                NodeType::Node48 => Either::Left(&mut *(pointer as *mut Node48<V>)),
                NodeType::Node256 => Either::Left(&mut *(pointer as *mut Node256<V>)),
            }
        }
    }

    fn new(node: impl NodeOps<V>, node_type: NodeType) -> Self {
        let node = Box::into_raw(Box::new(node));

        Self {
            pointer: node as usize | node_type as usize,
            _marker: PhantomData,
        }
    }

    const fn null() -> Self {
        Self {
            pointer: 0,
            _marker: PhantomData,
        }
    }

    #[inline]
    fn is_null(&self) -> bool {
        self.pointer == 0
    }

    fn node_type(&self) -> NodeType {
        unsafe { mem::transmute(self.pointer & NODETYPE_MASK) }
    }

    /// extend node to bigger one only if necessary
    fn extend(&mut self) {}

    /// shrink node to smaller one only if necessary
    fn shrink(&mut self) {}
}

struct NodeV<V> {
    key: Box<[u8]>,
    value: V,
}

struct Node4<V> {
    header: NodeHeader,
    len: usize,
    keys: [u8; 4],
    children: [Node<V>; 4],
}

impl<V> Default for Node4<V> {
    #[allow(deprecated)]
    fn default() -> Self {
        unsafe {
            Self {
                header: Default::default(),
                len: 0,
                keys: mem::uninitialized(),
                children: mem::uninitialized(),
            }
        }
    }
}

impl<V> Node4<V> {
    fn keys(&self) -> &[u8] {
        unsafe { self.keys.get_unchecked(..self.len as usize) }
    }

    fn mut_keys(&mut self) -> &mut [u8] {
        unsafe { self.keys.get_unchecked_mut(..self.len as usize) }
    }

    fn children(&self) -> &[Node<V>] {
        unsafe { self.children.get_unchecked(..self.len as usize) }
    }

    fn mut_children(&mut self) -> &mut [Node<V>] {
        unsafe { self.children.get_unchecked_mut(..self.len as usize) }
    }
}

impl<V> NodeOps<V> for Node4<V> {
    #[inline]
    fn is_full(&self) -> bool {
        self.len == 4
    }

    #[inline]
    fn is_shrinkable(&self) -> bool {
        false
    }

    fn insert(&mut self, key: u8, node: Node<V>) -> Result<(), Node<V>> {
        // since the &mut self is the pointer of Node4<V>, not the pointer of Node<V>,
        // simple extension like this is impossble.
        // if self.len == 4 {
        //     unsafe {
        //         let pointer = self as *const Node4<V> as *mut Node<V>;
        //         let extended = Node::new(
        //             Node16::from(ptr::read(pointer as *const Node4<V>)),
        //             NodeType::Node16,
        //         );
        //         *(pointer as *mut Node<V>) = extended;
        //         return (*pointer).deref_mut().left().unwrap().insert(key, node);
        //     }
        // }

        for (index, k) in self.keys().iter().enumerate() {
            match key.cmp(k) {
                Ordering::Less => unsafe {
                    self.len += 1;
                    slice_insert(self.mut_keys(), index, key);
                    slice_insert(self.mut_children(), index, node);
                    return Ok(());
                },
                Ordering::Equal => return Err(node),
                Ordering::Greater => {}
            }
        }

        Err(node)
    }

    fn lookup(&self, key: u8) -> Option<&Node<V>> {
        for (index, k) in self.keys().iter().enumerate() {
            if key == *k {
                return unsafe { Some(self.children.get_unchecked(index)) };
            }
        }

        None
    }

    fn update(&mut self, key: u8, node: Node<V>) -> Result<Node<V>, Node<V>> {
        for (index, k) in self.keys().iter().enumerate() {
            match key.cmp(k) {
                Ordering::Less => {}
                Ordering::Equal => unsafe {
                    let node = mem::replace(self.children.get_unchecked_mut(index), node);
                    return Ok(node);
                },
                Ordering::Greater => {}
            }
        }

        Err(node)
    }

    fn remove(&mut self, key: u8) -> Result<Node<V>, ()> {
        todo!()
    }
}

struct Node16<V> {
    header: NodeHeader,
    len: usize,
    keys: [u8; 16],
    children: [Node<V>; 16],
}

impl<V> Default for Node16<V> {
    #[allow(deprecated)]
    fn default() -> Self {
        unsafe {
            Self {
                header: Default::default(),
                len: 0,
                keys: mem::uninitialized(),
                children: mem::uninitialized(),
            }
        }
    }
}

impl<V> Node16<V> {
    fn keys(&self) -> &[u8] {
        unsafe { self.keys.get_unchecked(..self.len as usize) }
    }

    fn mut_keys(&mut self) -> &mut [u8] {
        unsafe { self.keys.get_unchecked_mut(..self.len as usize) }
    }

    fn children(&self) -> &[Node<V>] {
        unsafe { self.children.get_unchecked(..self.len as usize) }
    }

    fn mut_children(&mut self) -> &mut [Node<V>] {
        unsafe { self.children.get_unchecked_mut(..self.len as usize) }
    }

    fn from(node: Node4<V>) -> Self {
        let mut new = Self::default();
        new.header = node.header;
        new.len = node.len;

        unsafe {
            ptr::copy_nonoverlapping(node.keys.as_ptr(), new.keys.as_mut_ptr(), node.len as usize);
            ptr::copy_nonoverlapping(
                node.children.as_ptr(),
                new.children.as_mut_ptr(),
                node.len as usize,
            );
        }

        new
    }
}

impl<V> NodeOps<V> for Node16<V> {
    #[inline]
    fn is_full(&self) -> bool {
        self.len == 16
    }

    #[inline]
    fn is_shrinkable(&self) -> bool {
        self.len <= 4
    }

    fn insert(&mut self, key: u8, node: Node<V>) -> Result<(), Node<V>> {
        todo!()
    }

    fn lookup(&self, key: u8) -> Option<&Node<V>> {
        todo!()
    }

    fn update(&mut self, key: u8, node: Node<V>) -> Result<Node<V>, Node<V>> {
        todo!()
    }

    fn remove(&mut self, key: u8) -> Result<Node<V>, ()> {
        todo!()
    }
}

struct Node48<V> {
    header: NodeHeader,
    len: usize,
    keys: [u8; 256],
    children: [Node<V>; 48],
}

impl<V> Default for Node48<V> {
    #[allow(deprecated)]
    fn default() -> Self {
        unsafe {
            Self {
                header: Default::default(),
                len: 0,
                keys: mem::uninitialized(),
                children: mem::uninitialized(),
            }
        }
    }
}

impl<V> Node48<V> {
    fn keys(&self) -> &[u8] {
        unsafe { self.keys.get_unchecked(..self.len as usize) }
    }

    fn mut_keys(&mut self) -> &mut [u8] {
        unsafe { self.keys.get_unchecked_mut(..self.len as usize) }
    }

    fn children(&self) -> &[Node<V>] {
        unsafe { self.children.get_unchecked(..self.len as usize) }
    }

    fn mut_children(&mut self) -> &mut [Node<V>] {
        unsafe { self.children.get_unchecked_mut(..self.len as usize) }
    }

    fn from(node: Node16<V>) -> Self {
        let mut new = Self::default();

        unsafe {
            for (index, key) in node.keys().iter().enumerate() {
                *new.keys.get_unchecked_mut(*key as usize) = index as u8;
            }

            ptr::copy_nonoverlapping(
                node.children.as_ptr(),
                new.children.as_mut_ptr(),
                node.len as usize,
            );
        }

        new.header = node.header;
        new.len = node.len;

        new
    }
}

impl<V> NodeOps<V> for Node48<V> {
    #[inline]
    fn is_full(&self) -> bool {
        self.len == 48
    }

    #[inline]
    fn is_shrinkable(&self) -> bool {
        self.len <= 16
    }

    fn insert(&mut self, key: u8, node: Node<V>) -> Result<(), Node<V>> {
        todo!()
    }

    fn lookup(&self, key: u8) -> Option<&Node<V>> {
        todo!()
    }

    fn update(&mut self, key: u8, node: Node<V>) -> Result<Node<V>, Node<V>> {
        todo!()
    }

    fn remove(&mut self, key: u8) -> Result<Node<V>, ()> {
        todo!()
    }
}

struct Node256<V> {
    header: NodeHeader,
    len: usize,
    children: [Node<V>; 256],
}

impl<V> Default for Node256<V> {
    #[allow(deprecated)]
    fn default() -> Self {
        unsafe {
            Self {
                header: Default::default(),
                len: 0,
                children: mem::uninitialized(),
            }
        }
    }
}

impl<V> Node256<V> {
    fn from(node: Node48<V>) -> Self {
        let mut new = Self::default();

        unsafe {
            for key in node.keys() {
                *new.children.get_unchecked_mut(*key as usize) = ptr::read(
                    node.children
                        .get_unchecked(*node.keys.get_unchecked(*key as usize) as usize),
                );
            }
        }

        new.header = node.header;
        new.len = node.len;

        new
    }
}

impl<V> NodeOps<V> for Node256<V> {
    #[inline]
    fn is_full(&self) -> bool {
        self.len == 256
    }

    #[inline]
    fn is_shrinkable(&self) -> bool {
        self.len <= 48
    }

    fn insert(&mut self, key: u8, node: Node<V>) -> Result<(), Node<V>> {
        todo!()
    }

    fn lookup(&self, key: u8) -> Option<&Node<V>> {
        todo!()
    }

    fn update(&mut self, key: u8, node: Node<V>) -> Result<Node<V>, Node<V>> {
        todo!()
    }

    fn remove(&mut self, key: u8) -> Result<Node<V>, ()> {
        todo!()
    }
}

trait Encodable {
    fn encode(self) -> Vec<u8>;
}

struct Cursor<V> {
    parent: Option<NonNull<Node<V>>>,
    current: NonNull<Node<V>>,
}

pub struct ART<K, V> {
    root: NonNull<Node<V>>,
    _marker: PhantomData<K>,
}

impl<K, V> ART<K, V> {}

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
