use std::{
    cmp::{min, Ordering},
    marker::PhantomData,
    mem,
    ptr::{self, NonNull},
};

use arr_macro::arr;
use either::Either;
use std::fmt::Debug;

use crate::{
    left_or,
    map::SequentialMap,
    util::{slice_insert, slice_remove},
};

const PREFIX_LEN: usize = 12;
const KEY_ENDMARK: u8 = 0xff;
struct NodeHeader {
    len: u32,                 // the len of prefix
    prefix: [u8; PREFIX_LEN], // prefix for path compression
}

impl Debug for NodeHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            f.debug_struct("NodeHeader")
                .field("len", &self.len)
                .field("prefix", &self.prefix.get_unchecked(..self.len as usize))
                .finish()
        }
    }
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
#[derive(Debug, PartialEq)]
enum NodeType {
    Value = 0b000,
    Node4 = 0b001,
    Node16 = 0b010,
    Node48 = 0b011,
    Node256 = 0b100,
}

trait NodeOps<V> {
    fn header(&self) -> &NodeHeader;
    fn header_mut(&mut self) -> &mut NodeHeader;
    fn size(&self) -> usize;
    fn is_full(&self) -> bool;
    fn is_shrinkable(&self) -> bool;
    fn get_any_child(&self) -> Option<NodeV<V>>;
    fn insert(&mut self, key: u8, node: Node<V>) -> Result<(), Node<V>>;
    fn lookup(&self, key: u8) -> Option<&Node<V>>;
    fn lookup_mut(&mut self, key: u8) -> Option<&mut Node<V>>;
    fn update(&mut self, key: u8, node: Node<V>) -> Result<Node<V>, Node<V>>;
    fn remove(&mut self, key: u8) -> Result<Node<V>, ()>;
}

/// the pointer struct for Nodes or value
struct Node<V> {
    pointer: usize,
    _marker: PhantomData<Box<V>>,
}

impl<V: Debug> Debug for Node<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            let pointer = self.pointer & !NODETYPE_MASK;
            let tag = mem::transmute(self.pointer & NODETYPE_MASK);

            match tag {
                NodeType::Value => (&*(pointer as *const NodeV<V>)).fmt(f),
                NodeType::Node4 => (&*(pointer as *const Node4<V>)).fmt(f),
                NodeType::Node16 => (&*(pointer as *const Node16<V>)).fmt(f),
                NodeType::Node48 => (&*(pointer as *const Node48<V>)).fmt(f),
                NodeType::Node256 => (&*(pointer as *const Node256<V>)).fmt(f),
            }
        }
    }
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

    fn deref_mut(&self) -> Either<&mut dyn NodeOps<V>, &mut NodeV<V>> {
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

    fn inner<T>(self) -> Box<T> {
        // TODO: how to improve this function safely(self.node_type() == T::node_type())
        unsafe {
            let pointer = self.pointer & !NODETYPE_MASK;
            // let tag = mem::transmute(self.pointer & NODETYPE_MASK);

            Box::from_raw(pointer as *mut T)
        }
    }

    fn new<T>(node: T, node_type: NodeType) -> Self {
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
    fn extend(&mut self) {
        if self.deref().is_right() {
            return;
        }

        if !self.deref().left().unwrap().is_full() {
            return;
        }

        let node_type = self.node_type();
        let node = self.deref_mut().left().unwrap();

        match node_type {
            NodeType::Value => unreachable!(),
            NodeType::Node4 => unsafe {
                let node = node as *const dyn NodeOps<V> as *const Node4<V>;
                let new = Box::new(Node16::from(ptr::read(node)));
                self.pointer = Box::into_raw(new) as usize | NodeType::Node16 as usize;
            },
            NodeType::Node16 => unsafe {
                let node = node as *const dyn NodeOps<V> as *const Node16<V>;
                let new = Box::new(Node48::from(ptr::read(node)));
                self.pointer = Box::into_raw(new) as usize | NodeType::Node48 as usize;
            },
            NodeType::Node48 => unsafe {
                let node = node as *const dyn NodeOps<V> as *const Node48<V>;
                let new = Box::new(Node256::from(ptr::read(node)));
                self.pointer = Box::into_raw(new) as usize | NodeType::Node256 as usize;
            },
            NodeType::Node256 => {}
        }
    }

    /// shrink node to smaller one only if necessary
    fn shrink(&mut self) {
        if self.deref().is_right() {
            return;
        }

        if !self.deref().left().unwrap().is_shrinkable() {
            return;
        }

        let node_type = self.node_type();
        let node = self.deref_mut().left().unwrap();

        match node_type {
            NodeType::Value => unreachable!(),
            NodeType::Node4 => {}
            NodeType::Node16 => unsafe {
                let node = node as *const dyn NodeOps<V> as *const Node16<V>;
                let new = Box::new(Node4::from(ptr::read(node)));
                self.pointer = Box::into_raw(new) as usize | node_type as usize;
            },
            NodeType::Node48 => unsafe {
                let node = node as *const dyn NodeOps<V> as *const Node48<V>;
                let new = Box::new(Node16::from(ptr::read(node)));
                self.pointer = Box::into_raw(new) as usize | node_type as usize;
            },
            NodeType::Node256 => unsafe {
                let node = node as *const dyn NodeOps<V> as *const Node256<V>;
                let new = Box::new(Node48::from(ptr::read(node)));
                self.pointer = Box::into_raw(new) as usize | node_type as usize;
            },
        }
    }

    /// compare the keys from depth to header.len
    fn prefix_match(keys: &[u8], node: &dyn NodeOps<V>, depth: usize) -> Result<(), usize> {
        let header = node.header();

        for (index, prefix) in unsafe {
            header
                .prefix
                .get_unchecked(..header.len as usize)
                .iter()
                .enumerate()
        } {
            if keys[depth + index] != *prefix {
                return Err(depth + index);
            }
        }

        if header.len > PREFIX_LEN as u32 {
            // check strictly by using leaf node
            let any_child = node.get_any_child().unwrap();

            let mut d = depth + PREFIX_LEN;

            while d < depth + header.len as usize {
                if keys[d] != any_child.key[d] {
                    return Err(d);
                }

                d += 1;
            }
        }

        Ok(())
    }
}

struct NodeV<V> {
    key: Box<[u8]>,
    value: V,
}

impl<V: Debug> Debug for NodeV<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeV")
            .field("key", &self.key)
            .field("value", &self.value)
            .finish()
    }
}

impl<V> NodeV<V> {
    fn new(key: Vec<u8>, value: V) -> Self {
        Self {
            key: key.into(),
            value,
        }
    }
}

struct Node4<V> {
    header: NodeHeader,
    len: usize,
    keys: [u8; 4],
    children: [Node<V>; 4],
}

impl<V: Debug> Debug for Node4<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node4")
            .field("header", &self.header)
            .field("len", &self.len)
            .field("keys", &self.keys())
            .field("children", &self.children())
            .finish()
    }
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

impl<V> From<Node16<V>> for Node4<V> {
    fn from(node: Node16<V>) -> Self {
        debug_assert!(node.len <= 4);

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
    fn header(&self) -> &NodeHeader {
        &self.header
    }

    fn header_mut(&mut self) -> &mut NodeHeader {
        &mut self.header
    }

    fn size(&self) -> usize {
        self.len
    }

    fn is_full(&self) -> bool {
        self.len == 4
    }

    fn is_shrinkable(&self) -> bool {
        false
    }

    fn get_any_child(&self) -> Option<NodeV<V>> {
        todo!()
    }

    fn insert(&mut self, key: u8, node: Node<V>) -> Result<(), Node<V>> {
        debug_assert!(!self.is_full());

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

        let index = self.len;
        unsafe {
            self.len += 1;
            slice_insert(self.mut_keys(), index, key);
            slice_insert(self.mut_children(), index, node);
        }

        Ok(())
    }

    fn lookup(&self, key: u8) -> Option<&Node<V>> {
        for (index, k) in self.keys().iter().enumerate() {
            if key == *k {
                return unsafe { Some(self.children.get_unchecked(index)) };
            }
        }

        None
    }

    fn lookup_mut(&mut self, key: u8) -> Option<&mut Node<V>> {
        for (index, k) in self.keys().iter().enumerate() {
            if key == *k {
                return unsafe { Some(self.children.get_unchecked_mut(index)) };
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
        debug_assert!(self.len != 0);

        for (index, k) in self.keys().iter().enumerate() {
            match key.cmp(k) {
                Ordering::Less => {}
                Ordering::Equal => unsafe {
                    let _ = slice_remove(self.mut_keys(), index);
                    let node = slice_remove(self.mut_children(), index);
                    self.len -= 1;
                    return Ok(node);
                },
                Ordering::Greater => {}
            }
        }

        Err(())
    }
}

struct Node16<V> {
    header: NodeHeader,
    len: usize,
    keys: [u8; 16],
    children: [Node<V>; 16],
}

impl<V: Debug> Debug for Node16<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node16")
            .field("header", &self.header)
            .field("len", &self.len)
            .field("keys", &self.keys())
            .field("children", &self.children())
            .finish()
    }
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

impl<V> From<Node4<V>> for Node16<V> {
    fn from(node: Node4<V>) -> Self {
        debug_assert!(node.len == 4);

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

impl<V> From<Node48<V>> for Node16<V> {
    fn from(node: Node48<V>) -> Self {
        debug_assert!(node.len <= 16);

        let mut new = Self::default();
        new.header = node.header;
        new.len = node.len;

        unsafe {
            let mut i = 0;
            for (key, index) in node.keys.iter().enumerate() {
                if *index != 0xff {
                    *new.keys.get_unchecked_mut(i) = key as u8;
                    *new.children.get_unchecked_mut(i) =
                        ptr::read(node.children.get_unchecked(*index as usize));
                    i += 1;
                }
            }
        }

        new
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
}

impl<V> NodeOps<V> for Node16<V> {
    fn header(&self) -> &NodeHeader {
        &self.header
    }

    fn header_mut(&mut self) -> &mut NodeHeader {
        &mut self.header
    }

    fn size(&self) -> usize {
        self.len
    }

    fn is_full(&self) -> bool {
        self.len == 16
    }

    fn is_shrinkable(&self) -> bool {
        self.len <= 4
    }

    fn get_any_child(&self) -> Option<NodeV<V>> {
        todo!()
    }

    fn insert(&mut self, key: u8, node: Node<V>) -> Result<(), Node<V>> {
        debug_assert!(!self.is_full());

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

        let index = self.len;
        unsafe {
            self.len += 1;
            slice_insert(self.mut_keys(), index, key);
            slice_insert(self.mut_children(), index, node);
        }

        Ok(())
    }

    fn lookup(&self, key: u8) -> Option<&Node<V>> {
        for (index, k) in self.keys().iter().enumerate() {
            if key == *k {
                return unsafe { Some(self.children.get_unchecked(index)) };
            }
        }

        None
    }

    fn lookup_mut(&mut self, key: u8) -> Option<&mut Node<V>> {
        for (index, k) in self.keys().iter().enumerate() {
            if key == *k {
                return unsafe { Some(self.children.get_unchecked_mut(index)) };
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
        debug_assert!(self.len != 0);

        for (index, k) in self.keys().iter().enumerate() {
            match key.cmp(k) {
                Ordering::Less => {}
                Ordering::Equal => unsafe {
                    let _ = slice_remove(self.mut_keys(), index);
                    let node = slice_remove(self.mut_children(), index);
                    self.len -= 1;
                    return Ok(node);
                },
                Ordering::Greater => {}
            }
        }

        Err(())
    }
}
struct Node48<V> {
    header: NodeHeader,
    len: usize,
    keys: [u8; 256],
    children: [Node<V>; 48],
}

impl<V: Debug> Debug for Node48<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let valid_keys = self
            .keys
            .iter()
            .enumerate()
            .filter(|(_, index)| **index != 0xff)
            .map(|(key, _)| key)
            .collect::<Vec<_>>();

        f.debug_struct("Node48")
            .field("header", &self.header)
            .field("len", &self.len)
            .field("keys", &valid_keys)
            .field("children", &self.children())
            .finish()
    }
}

impl<V> Default for Node48<V> {
    #[allow(deprecated)]
    fn default() -> Self {
        Self {
            header: Default::default(),
            len: 0,
            keys: arr![0xff; 256], // the invalid index is 0xff
            children: arr![Node::null(); 48],
        }
    }
}

impl<V> From<Node16<V>> for Node48<V> {
    fn from(node: Node16<V>) -> Self {
        debug_assert!(node.len == 16);

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

impl<V> From<Node256<V>> for Node48<V> {
    fn from(node: Node256<V>) -> Self {
        debug_assert!(node.len <= 48);

        let mut new = Self::default();

        unsafe {
            // TODO: child is dropping?
            for (key, child) in node.children.iter().enumerate() {
                if !child.is_null() {
                    new.len += 1;
                    *new.keys.get_unchecked_mut(key) = (new.len - 1) as u8;
                    *new.children.get_unchecked_mut(new.len - 1) = ptr::read(child);
                }
            }
        }

        new.header = node.header;
        new.len = node.len;

        new
    }
}

impl<V> Node48<V> {
    fn children(&self) -> &[Node<V>] {
        unsafe { self.children.get_unchecked(..self.len as usize) }
    }

    fn mut_children(&mut self) -> &mut [Node<V>] {
        unsafe { self.children.get_unchecked_mut(..self.len as usize) }
    }
}

impl<V> NodeOps<V> for Node48<V> {
    fn header(&self) -> &NodeHeader {
        &self.header
    }

    fn header_mut(&mut self) -> &mut NodeHeader {
        &mut self.header
    }

    fn size(&self) -> usize {
        self.len
    }

    fn is_full(&self) -> bool {
        self.len == 48
    }

    fn is_shrinkable(&self) -> bool {
        self.len <= 16
    }

    fn get_any_child(&self) -> Option<NodeV<V>> {
        todo!()
    }

    fn insert(&mut self, key: u8, node: Node<V>) -> Result<(), Node<V>> {
        debug_assert!(!self.is_full());

        let index = unsafe { self.keys.get_unchecked_mut(key as usize) };

        if *index != 0xff {
            Err(node)
        } else {
            unsafe {
                *self.children.get_unchecked_mut(self.len) = node;
            }

            *index = self.len as u8;
            self.len += 1;
            Ok(())
        }
    }

    fn lookup(&self, key: u8) -> Option<&Node<V>> {
        let index = unsafe { self.keys.get_unchecked(key as usize) };

        if *index == 0xff {
            None
        } else {
            unsafe { Some(self.children.get_unchecked(*index as usize)) }
        }
    }

    fn lookup_mut(&mut self, key: u8) -> Option<&mut Node<V>> {
        let index = unsafe { self.keys.get_unchecked(key as usize) };

        if *index == 0xff {
            None
        } else {
            unsafe { Some(self.children.get_unchecked_mut(*index as usize)) }
        }
    }

    fn update(&mut self, key: u8, node: Node<V>) -> Result<Node<V>, Node<V>> {
        let index = unsafe { self.keys.get_unchecked_mut(key as usize) };

        if *index == 0xff {
            Err(node)
        } else {
            let child = unsafe { self.children.get_unchecked_mut(*index as usize) };
            let old = mem::replace(child, node);
            Ok(old)
        }
    }

    fn remove(&mut self, key: u8) -> Result<Node<V>, ()> {
        let index = unsafe { self.keys.get_unchecked(key as usize).clone() };

        if index == 0xff {
            Err(())
        } else {
            unsafe {
                let node = slice_remove(self.mut_children(), index as usize);
                *self.keys.get_unchecked_mut(key as usize) = 0xff;
                self.len -= 1;
                Ok(node)
            }
        }
    }
}

struct Node256<V> {
    header: NodeHeader,
    len: usize,
    children: [Node<V>; 256],
}

impl<V: Debug> Debug for Node256<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let valid_children = self
            .children
            .iter()
            .enumerate()
            .filter(|(_, child)| !child.is_null())
            .collect::<Vec<_>>();

        f.debug_struct("Node256")
            .field("header", &self.header)
            .field("len", &self.len)
            .field("children", &valid_children)
            .finish()
    }
}

impl<V> Default for Node256<V> {
    #[allow(deprecated)]
    fn default() -> Self {
        Self {
            header: Default::default(),
            len: 0,
            children: arr![Node::null(); 256],
        }
    }
}

impl<V> From<Node48<V>> for Node256<V> {
    fn from(node: Node48<V>) -> Self {
        debug_assert!(node.len == 48);

        let mut new = Self::default();

        unsafe {
            for (key, index) in node.keys.iter().enumerate() {
                if *index != 0xff {
                    *new.children.get_unchecked_mut(key) =
                        ptr::read(node.children.get_unchecked(*index as usize));
                }
            }
        }

        new.header = node.header;
        new.len = node.len;

        new
    }
}

impl<V> NodeOps<V> for Node256<V> {
    fn header(&self) -> &NodeHeader {
        &self.header
    }

    fn header_mut(&mut self) -> &mut NodeHeader {
        &mut self.header
    }

    fn size(&self) -> usize {
        self.len
    }

    fn is_full(&self) -> bool {
        self.len == 256
    }

    fn is_shrinkable(&self) -> bool {
        self.len <= 48
    }

    fn get_any_child(&self) -> Option<NodeV<V>> {
        todo!()
    }

    fn insert(&mut self, key: u8, node: Node<V>) -> Result<(), Node<V>> {
        let child = unsafe { self.children.get_unchecked_mut(key as usize) };

        if child.is_null() {
            *child = node;
            Ok(())
        } else {
            Err(node)
        }
    }

    fn lookup(&self, key: u8) -> Option<&Node<V>> {
        let child = unsafe { self.children.get_unchecked(key as usize) };

        if child.is_null() {
            None
        } else {
            Some(child)
        }
    }

    fn lookup_mut(&mut self, key: u8) -> Option<&mut Node<V>> {
        let child = unsafe { self.children.get_unchecked_mut(key as usize) };

        if child.is_null() {
            None
        } else {
            Some(child)
        }
    }

    fn update(&mut self, key: u8, node: Node<V>) -> Result<Node<V>, Node<V>> {
        let child = unsafe { self.children.get_unchecked_mut(key as usize) };

        if child.is_null() {
            Err(node)
        } else {
            let old = mem::replace(child, node);
            Ok(old)
        }
    }

    fn remove(&mut self, key: u8) -> Result<Node<V>, ()> {
        let child = unsafe { self.children.get_unchecked_mut(key as usize) };

        if child.is_null() {
            Err(())
        } else {
            let node = mem::replace(child, Node::null());
            Ok(node)
        }
    }
}

pub trait Encodable {
    fn encode(&self) -> Vec<u8>;
}

impl Encodable for String {
    fn encode(&self) -> Vec<u8> {
        let mut array = self.clone().into_bytes();
        array.push(KEY_ENDMARK); // prevent to certain string cannot be the prefix of another string
        array
    }
}

pub struct ART<K, V> {
    root: Node<V>,
    _marker: PhantomData<K>,
}

impl<K, V: Debug> Debug for ART<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ART").field("root", &self.root).finish()
    }
}

impl<K, V> ART<K, V> {}

impl<K: Eq + Encodable, V: Debug> SequentialMap<K, V> for ART<K, V> {
    fn new() -> Self {
        let root = Node::new(Node256::<V>::default(), NodeType::Node256);

        Self {
            root,
            _marker: PhantomData,
        }
    }

    fn insert(&mut self, key: &K, value: V) -> Result<(), V> {
        let keys = key.encode();
        let mut depth = 0;
        let mut common_prefix: u32 = 0;
        let mut current = NonNull::new(&mut self.root).unwrap();

        while depth < keys.len() {
            let current_ref = unsafe { current.as_mut() };
            let node = left_or!(current_ref.deref_mut(), break);

            if let Err(common_depth) = Node::prefix_match(&keys, node, depth) {
                // println!("same common prefix");
                common_prefix = (common_depth - depth) as u32;
                break;
            }

            let prefix = node.header().len;

            if let Some(node) = node.lookup_mut(keys[depth + prefix as usize]) {
                depth += 1 + prefix as usize;
                current = NonNull::new(node).unwrap();
            } else {
                // println!("cannot find {} on {:?}", keys[depth], current_ref);
                common_prefix = prefix;
                break;
            }
        }

        let current_ref = unsafe { current.as_mut() };
        current_ref.extend();

        // println!("current: {:?}", current_ref);

        match current_ref.deref_mut() {
            Either::Left(node) => {
                let key = keys[depth];
                let new = NodeV::new(keys.clone(), value);

                if common_prefix == node.header().len {
                    // just insert value into this node
                    let insert = node.insert(key, Node::new(new, NodeType::Value));
                    debug_assert!(insert.is_ok());
                } else {
                    // split prefix
                    let mut inter_node = Node4::<V>::default();

                    unsafe {
                        ptr::copy_nonoverlapping(
                            keys.as_ptr().add(depth),
                            inter_node.header.prefix.as_mut_ptr(),
                            common_prefix as usize,
                        );
                    }

                    inter_node.header.len = common_prefix;

                    // re-set the old's prefix
                    let header = node.header_mut();
                    let prefix = header.prefix.clone();
                    unsafe {
                        ptr::copy_nonoverlapping(
                            prefix.as_ptr(),
                            header.prefix.as_mut_ptr(),
                            (header.len - common_prefix) as usize,
                        )
                    };
                    header.len = header.len - common_prefix;

                    let current = unsafe { current.as_mut() };
                    let old = mem::replace(current, Node::new(inter_node, NodeType::Node4));
                    let current = current.deref_mut().left().unwrap();

                    let insert_old =
                        current.insert(node.header().prefix[depth + common_prefix as usize], old);
                    debug_assert!(insert_old.is_ok());
                    let insert_new = current.insert(key, Node::new(new, NodeType::Value));
                    debug_assert!(insert_new.is_ok());
                }

                Ok(())
            }
            Either::Right(nodev) => {
                if depth == keys.len() {
                    return Err(value);
                }

                let new = NodeV::new(keys.clone(), value);

                // insert inter node with zero prefix
                // ex) 'aE', 'aaE'
                // println!("split with same index {}", keys[depth]);
                let mut common_prefix = 0;

                while keys[depth + common_prefix] == nodev.key[depth + common_prefix] {
                    common_prefix += 1;
                }

                // println!(
                //     "{}, common_prefix: {}, {}",
                //     depth,
                //     common_prefix,
                //     nodev.key[depth + common_prefix]
                // );

                let mut inter_node = Node4::<V>::default();
                unsafe {
                    ptr::copy_nonoverlapping(
                        keys.as_ptr().add(depth),
                        inter_node.header.prefix.as_mut_ptr(),
                        min(common_prefix, PREFIX_LEN),
                    );
                }
                inter_node.header.len = min(common_prefix, PREFIX_LEN) as u32;

                let current = unsafe { current.as_mut() };
                let old = mem::replace(current, Node::new(inter_node, NodeType::Node4));

                let current = current.deref_mut().left().unwrap();

                let insert_old = current.insert(nodev.key[depth + common_prefix], old);
                debug_assert!(insert_old.is_ok());
                let insert_new =
                    current.insert(keys[depth + common_prefix], Node::new(new, NodeType::Value));
                debug_assert!(insert_new.is_ok());

                Ok(())
            }
        }
    }

    fn lookup(&self, key: &K) -> Option<&V> {
        let keys = key.encode();
        let mut depth = 0;

        let mut current = &self.root;

        while depth < keys.len() {
            let node = left_or!(current.deref(), break);
            depth += node.header().len as usize;

            if let Some(node) = node.lookup(keys[depth]) {
                depth += 1;
                current = node;
            } else {
                return None;
            }
        }

        match current.deref() {
            Either::Left(_) => None,
            Either::Right(nodev) => {
                if *nodev.key == keys {
                    Some(&nodev.value)
                } else {
                    None
                }
            }
        }
    }

    fn remove(&mut self, key: &K) -> Result<V, ()> {
        let keys = key.encode();
        let mut depth = 0;

        let mut parent = None;
        let mut current = NonNull::new(&mut self.root).unwrap();

        while depth < keys.len() {
            let current_ref = unsafe { current.as_mut() };
            let node = current_ref.deref_mut().unwrap_left();
            depth += node.header().len as usize;

            if let Some(node) = node.lookup_mut(keys[depth]) {
                // println!("{:?}, key: {}", node, keys[depth]);

                if node.node_type() == NodeType::Value {
                    if *node.deref().right().unwrap().key == keys {
                        break;
                    } else {
                        return Err(());
                    }
                }

                depth += 1;
                parent = Some(current);
                current = NonNull::new(node).unwrap();
            } else {
                return Err(());
            }
        }

        let current = unsafe { current.as_mut() };
        let current_ref = current.deref_mut().left().unwrap();
        let node = current_ref.remove(keys[depth]);
        debug_assert!(node.is_ok());
        let node = node.unwrap().inner::<NodeV<V>>();

        if current_ref.size() == 1 {
            // path compression
            todo!()
        }

        if let Some(mut parent) = parent {
            if current_ref.size() == 0 {
                // remove the node
                // println!("empty");
                let parent = unsafe { parent.as_mut() };
                let parent_ref = parent.deref_mut().left().unwrap();

                let remove = parent_ref.remove(keys[depth - current_ref.header().len as usize - 1]);
                debug_assert!(remove.is_ok());
                let remove = remove.unwrap();
                debug_assert_eq!(remove.deref().left().unwrap().size(), 0);
                debug_assert_eq!(remove.node_type(), NodeType::Node4);
                remove.inner::<Node4<V>>();
            } else if current_ref.is_shrinkable() {
                // shrink the node
                // println!("shrinkable");
                current.shrink();
            }
        }

        Ok(node.value)
    }
}
