use std::fmt::Debug;
use std::ptr;
use std::{cmp::Ordering, mem, ptr::NonNull};

use crate::map::SequentialMap;

const B_MAX_NODES: usize = 11;
const B_MID_INDEX: usize = B_MAX_NODES / 2;

// TODO: optimize with MaybeUninit
struct Node<K, V> {
    size: usize,
    depth: usize,
    keys: [K; B_MAX_NODES],
    edges: [Box<Node<K, V>>; B_MAX_NODES + 1],
    values: [V; B_MAX_NODES],
}

impl<K, V> Drop for Node<K, V> {
    fn drop(&mut self) {
        if self.size > 0 {
            panic!("The node should be emptied before dropping!")
        } else {
            panic!("Please use mem::forget");
        }
    }
}

impl<K: Debug, V: Debug> Debug for Node<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("size", &self.size)
            .field("depth", &self.depth)
            .field("keys", &self.keys())
            .field("edges", &self.edges())
            .field("values", &self.values())
            .finish()
    }
}

impl<K, V> Node<K, V> {
    // since most of MaybeUnit APIs are experimental, I use very dangerous `mem::uninitialized until they become stable
    #[allow(deprecated)]
    fn new() -> Self {
        Self {
            size: 0,
            depth: 0,
            keys: unsafe { mem::uninitialized() },
            edges: unsafe { mem::uninitialized() },
            values: unsafe { mem::uninitialized() },
        }
    }
}

/// insert value into [T], which has one empty area on last.
/// ex) insert C at 1 into [A, B, uninit] => [A, C, B]
unsafe fn slice_insert<T>(ptr: &mut [T], index: usize, value: T) {
    let size = ptr.len();
    debug_assert!(size > index);

    let ptr = ptr.as_mut_ptr();

    if size > index + 1 {
        ptr::copy(ptr.add(index), ptr.add(index + 1), size - index - 1);
    }

    ptr::write(ptr.add(index), value);
}

/// remove value from [T] and remain last area without any init
/// ex) remove at 1 from [A, B, C] => [A, C, C(but you should not access here)]
unsafe fn slice_remove<T>(ptr: &mut [T], index: usize) -> T {
    let size = ptr.len();
    debug_assert!(size > index);

    let ptr = ptr.as_mut_ptr();
    let value = ptr::read(ptr.add(index));

    if size > index + 1 {
        ptr::copy(ptr.add(index + 1), ptr.add(index), size - index - 1);
    }

    value
}

enum InsertResult<K, V> {
    Fitted,
    Splitted {
        parent: (K, V),
        right: Box<Node<K, V>>,
    },
}

impl<K, V> Node<K, V> {
    fn keys(&self) -> &[K] {
        unsafe { self.keys.get_unchecked(..self.size) }
    }

    fn mut_keys(&mut self) -> &mut [K] {
        unsafe { self.keys.get_unchecked_mut(..self.size) }
    }

    fn values(&self) -> &[V] {
        unsafe { self.values.get_unchecked(..self.size) }
    }

    fn mut_values(&mut self) -> &mut [V] {
        unsafe { self.values.get_unchecked_mut(..self.size) }
    }

    fn edges(&self) -> &[Box<Node<K, V>>] {
        if self.depth > 0 {
            unsafe { self.edges.get_unchecked(..(self.size + 1)) }
        } else {
            &[]
        }
    }

    fn mut_edges(&mut self) -> &mut [Box<Node<K, V>>] {
        unsafe { self.edges.get_unchecked_mut(..(self.size + 1)) }
    }
}

impl<K: Ord, V> Node<K, V> {
    fn insert_leaf(&mut self, edge_index: usize, key: K, value: V) -> InsertResult<K, V> {
        if self.size < B_MAX_NODES {
            self.size += 1;

            unsafe {
                slice_insert(self.mut_keys(), edge_index, key);
                slice_insert(self.mut_values(), edge_index, value);
            }

            InsertResult::Fitted
        } else {
            // Split node into middle (key, value) and
            // two leaf nodes that the left one is connected with parent and the right one is disconnected.
            // Ex) On 2-3 tree, the leaf (key, value): parent-Node { data: [(1, 1), (2, 2)] } and try inserting (3, 3):
            // Make parent-[(1, 1)] and return InsertResult::Splitted { parent: (2, 2), right: Node { data: [(3, 3)] }}.

            let mut node = Box::new(Node::new());
            node.size = B_MAX_NODES - B_MID_INDEX;

            match edge_index.cmp(&B_MID_INDEX) {
                Ordering::Less => {
                    // on [(1, _), (2, _)], insert (0, _) with edge_index = 0
                    unsafe {
                        // TODO: can be optimized by partial copy between remove and insert
                        let mid = (
                            slice_remove(self.mut_keys(), B_MID_INDEX - 1),
                            slice_remove(self.mut_values(), B_MID_INDEX - 1),
                        );

                        slice_insert(self.mut_keys(), edge_index, key);
                        slice_insert(self.mut_values(), edge_index, value);

                        ptr::copy_nonoverlapping(
                            self.keys.as_mut_ptr().add(B_MID_INDEX),
                            node.keys.as_mut_ptr(),
                            B_MAX_NODES - B_MID_INDEX,
                        );
                        ptr::copy_nonoverlapping(
                            self.values.as_mut_ptr().add(B_MID_INDEX),
                            node.values.as_mut_ptr(),
                            B_MAX_NODES - B_MID_INDEX,
                        );

                        self.size = B_MID_INDEX;

                        // debug_assert!(self.data.len() == B_MID_INDEX);
                        // debug_assert!(node.data.len() == B_MID_INDEX);

                        InsertResult::Splitted {
                            parent: mid,
                            right: node,
                        }
                    }
                }
                Ordering::Equal => {
                    // on [(0, _), (2, _)], insert (1, _) with edge_index = 1
                    let mid = (key, value);

                    unsafe {
                        ptr::copy_nonoverlapping(
                            self.keys.as_mut_ptr().add(B_MID_INDEX),
                            node.keys.as_mut_ptr(),
                            B_MAX_NODES - B_MID_INDEX,
                        );
                        ptr::copy_nonoverlapping(
                            self.values.as_mut_ptr().add(B_MID_INDEX),
                            node.values.as_mut_ptr(),
                            B_MAX_NODES - B_MID_INDEX,
                        );
                    }

                    self.size = B_MID_INDEX;

                    // debug_assert!(self.data.len() == B_MID_INDEX);
                    // debug_assert!(node.data.len() == B_MID_INDEX);

                    InsertResult::Splitted {
                        parent: mid,
                        right: node,
                    }
                }
                Ordering::Greater => {
                    // on [(0, _), (1, _)], insert (2, _) with edge_index = 2
                    unsafe {
                        let mid = (
                            slice_remove(self.mut_keys(), B_MID_INDEX),
                            slice_remove(self.mut_values(), B_MID_INDEX),
                        );

                        slice_insert(self.mut_keys(), edge_index - 1, key);
                        slice_insert(self.mut_values(), edge_index - 1, value);

                        ptr::copy_nonoverlapping(
                            self.keys.as_mut_ptr().add(B_MID_INDEX),
                            node.keys.as_mut_ptr(),
                            B_MAX_NODES - B_MID_INDEX,
                        );
                        ptr::copy_nonoverlapping(
                            self.values.as_mut_ptr().add(B_MID_INDEX),
                            node.values.as_mut_ptr(),
                            B_MAX_NODES - B_MID_INDEX,
                        );

                        self.size = B_MID_INDEX;

                        // debug_assert!(self.data.len() == B_MID_INDEX);
                        // debug_assert!(node.data.len() == B_MID_INDEX);

                        InsertResult::Splitted {
                            parent: mid,
                            right: node,
                        }
                    }
                }
            }
        }
    }

    fn insert_inner(
        &mut self,
        edge_index: usize,
        key: K,
        value: V,
        edge: Box<Node<K, V>>,
    ) -> InsertResult<K, V> {
        if self.size < B_MAX_NODES {
            self.size += 1;

            unsafe {
                slice_insert(self.mut_keys(), edge_index, key);
                slice_insert(self.mut_values(), edge_index, value);
                slice_insert(self.mut_edges(), edge_index + 1, edge);
            }

            InsertResult::Fitted
        } else {
            // split node into middle (key, value) and
            // two leaf nodes that the left one is connected with parent and the right one is disconnected.
            // ex) Let Node { data: [(x, _), ... ]} Node_x.
            // On 2-3 tree, the leaf (key, value): parent-Node { data: [(1, 1), (5, 5)], edges: [Node_0, Node_2, Node_6] } and try inserting (3, 3) and Node_4:
            // Make parent-Node { data: [(1, 1)], edges: [Node_0, Node_2] }
            // and return InsertResult::Splitted { parent: (3, 3), right: Node { data: [(5, 5)], edges: [Node_4, Node_6]} }

            let mut node = Box::new(Node::new());
            node.size = B_MAX_NODES - B_MID_INDEX;
            node.depth = self.depth;

            match edge_index.cmp(&B_MID_INDEX) {
                Ordering::Less => {
                    // on Node { data: [(3, _), (5, _)], edges: [Node_0, Node_4, Node_6] }, insert (1, _) and Node_2 with edge_index = 0

                    unsafe {
                        // TODO: can be optimized by partial copy between remove and insert
                        let mid = (
                            slice_remove(self.mut_keys(), B_MID_INDEX - 1),
                            slice_remove(self.mut_values(), B_MID_INDEX - 1),
                        );

                        slice_insert(self.mut_keys(), edge_index, key);
                        slice_insert(self.mut_values(), edge_index, value);

                        ptr::copy_nonoverlapping(
                            self.keys.as_mut_ptr().add(B_MID_INDEX),
                            node.keys.as_mut_ptr(),
                            B_MAX_NODES - B_MID_INDEX,
                        );
                        ptr::copy_nonoverlapping(
                            self.values.as_mut_ptr().add(B_MID_INDEX),
                            node.values.as_mut_ptr(),
                            B_MAX_NODES - B_MID_INDEX,
                        );

                        ptr::copy_nonoverlapping(
                            self.edges.as_mut_ptr().add(B_MID_INDEX),
                            node.edges.as_mut_ptr(),
                            (B_MAX_NODES + 1) - B_MID_INDEX,
                        );
                        slice_insert(self.mut_edges(), edge_index + 1, edge);

                        self.size = B_MID_INDEX;

                        // debug_assert!(self.data.len() == B_MID_INDEX);
                        // debug_assert!(self.edges.len() == B_MID_INDEX + 1);
                        // debug_assert!(node.data.len() == B_MID_INDEX);
                        // debug_assert!(node.edges.len() == B_MID_INDEX + 1);

                        InsertResult::Splitted {
                            parent: mid,
                            right: node,
                        }
                    }
                }
                Ordering::Equal => {
                    // on Node { data: [(1, _), (5, _)], edges: [Node_0, Node_2, Node_6] }, insert (3, _) and Node_4 with edge_index = 1
                    unsafe {
                        let mid = (key, value);

                        ptr::copy_nonoverlapping(
                            self.keys.as_mut_ptr().add(B_MID_INDEX),
                            node.keys.as_mut_ptr(),
                            B_MAX_NODES - B_MID_INDEX,
                        );
                        ptr::copy_nonoverlapping(
                            self.values.as_mut_ptr().add(B_MID_INDEX),
                            node.values.as_mut_ptr(),
                            B_MAX_NODES - B_MID_INDEX,
                        );

                        ptr::write(node.edges.as_mut_ptr(), edge);
                        ptr::copy_nonoverlapping(
                            self.edges.as_mut_ptr().add(B_MID_INDEX + 1),
                            node.edges.as_mut_ptr().add(1),
                            (B_MAX_NODES + 1) - (B_MID_INDEX + 1),
                        );

                        self.size = B_MID_INDEX;

                        // debug_assert!(self.data.len() == B_MID_INDEX);
                        // debug_assert!(self.edges.len() == B_MID_INDEX + 1);
                        // debug_assert!(node.data.len() == B_MID_INDEX);
                        // debug_assert!(node.edges.len() == B_MID_INDEX + 1);

                        InsertResult::Splitted {
                            parent: mid,
                            right: node,
                        }
                    }
                }
                Ordering::Greater => {
                    // on Node { data: [(1, _), (3, _)], edges: [Node_0, Node_2, Node_4] }, insert (5, _) and Node_6 with edge_index = 2
                    unsafe {
                        let mid = (
                            slice_remove(self.mut_keys(), B_MID_INDEX),
                            slice_remove(self.mut_values(), B_MID_INDEX),
                        );

                        slice_insert(self.mut_keys(), edge_index - 1, key);
                        slice_insert(self.mut_values(), edge_index - 1, value);

                        ptr::copy_nonoverlapping(
                            self.keys.as_mut_ptr().add(B_MID_INDEX),
                            node.keys.as_mut_ptr(),
                            B_MAX_NODES - B_MID_INDEX,
                        );
                        ptr::copy_nonoverlapping(
                            self.values.as_mut_ptr().add(B_MID_INDEX),
                            node.values.as_mut_ptr(),
                            B_MAX_NODES - B_MID_INDEX,
                        );

                        ptr::copy_nonoverlapping(
                            self.edges.as_mut_ptr().add(B_MID_INDEX + 1),
                            node.edges.as_mut_ptr(),
                            (B_MAX_NODES + 1) - (B_MID_INDEX + 1),
                        );
                        slice_insert(node.mut_edges(), edge_index - B_MID_INDEX, edge);

                        self.size = B_MID_INDEX;

                        // debug_assert!(self.data.len() == B_MID_INDEX);
                        // debug_assert!(self.edges.len() == B_MID_INDEX + 1);
                        // debug_assert!(node.data.len() == B_MID_INDEX);
                        // debug_assert!(node.edges.len() == B_MID_INDEX + 1);

                        InsertResult::Splitted {
                            parent: mid,
                            right: node,
                        }
                    }
                }
            }
        }
    }

    /// find the leftmost node from the tree whose root is self
    fn find_begin(&mut self) -> (Vec<(NonNull<Node<K, V>>, usize)>, &mut Node<K, V>) {
        let mut parents = Vec::new();
        let mut target = NonNull::from(self);

        loop {
            let target_mut = unsafe { target.as_mut() };

            if target_mut.depth == 0 {
                return (parents, target_mut);
            }

            parents.push((target, 0));
            target = NonNull::from(target_mut.mut_edges().first_mut().unwrap().as_mut());
        }
    }

    /// find the rightmost node from the tree whose root is self
    fn find_end(&mut self) -> (Vec<(NonNull<Node<K, V>>, usize)>, &mut Node<K, V>) {
        let mut parents = Vec::new();
        let mut target = NonNull::from(self);

        loop {
            let target_mut = unsafe { target.as_mut() };

            if target_mut.depth == 0 {
                return (parents, target_mut);
            }

            parents.push((target, target_mut.size));
            target = NonNull::from(target_mut.mut_edges().last_mut().unwrap().as_mut());
        }
    }
}

struct Cursor<K, V> {
    ancestors: Vec<(NonNull<Node<K, V>>, usize)>, // (parent, index from parent.edges[index])
    current: NonNull<Node<K, V>>,
    result: SearchResult,
}

#[derive(Debug)]
enum SearchResult {
    Some { value_index: usize },   // the value of the key exists
    None { edge_index: usize },    // the value of the key does not exist
    Descent { edge_index: usize }, // need more search
    NodeSearch,                    // after Descent, the cursor needs NodeSearch
}

impl<K: Ord, V> Cursor<K, V> {
    fn new(tree: &BTree<K, V>) -> Self {
        let depth = unsafe { tree.root.as_ref().depth };

        Self {
            ancestors: Vec::with_capacity(depth + 1),
            current: tree.root,
            result: SearchResult::NodeSearch,
        }
    }

    fn search_in_node(self, key: &K) -> Self {
        let node = unsafe { self.current.as_ref() };

        for (index, k) in node.keys().iter().enumerate() {
            match key.cmp(k) {
                Ordering::Less => {
                    return Self {
                        result: SearchResult::Descent { edge_index: index },
                        ..self
                    }
                }
                Ordering::Equal => {
                    return Self {
                        result: SearchResult::Some { value_index: index },
                        ..self
                    }
                }
                Ordering::Greater => {}
            }
        }

        Self {
            result: SearchResult::Descent {
                edge_index: node.size,
            },
            ..self
        }
    }

    fn descend(mut self, edge_index: usize) -> Self {
        let current = unsafe { self.current.as_mut() };

        if current.depth == 0 {
            return Self {
                result: SearchResult::None { edge_index },
                ..self
            };
        }

        debug_assert!(current.size > 0);

        if edge_index <= current.size {
            let node = current.edges[edge_index].as_mut();
            let parent = mem::replace(&mut self.current, NonNull::new(node).unwrap());
            self.ancestors.push((parent, edge_index));

            Self {
                result: SearchResult::NodeSearch,
                ..self
            }
        } else {
            Self {
                result: SearchResult::None { edge_index },
                ..self
            }
        }
    }
}

pub struct BTree<K, V> {
    root: NonNull<Node<K, V>>,
    size: usize,
}

impl<K: Debug, V: Debug> Debug for BTree<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            f.debug_struct("BTree")
                .field("root", self.root.as_ref())
                .field("size", &self.size)
                .finish()
        }
    }
}

impl<K: Ord, V> BTree<K, V> {
    fn find(&self, key: &K) -> Cursor<K, V> {
        let mut cursor = Cursor::new(self);

        loop {
            // on the node, search with the key
            cursor = match cursor.result {
                SearchResult::None { .. } => return cursor,
                SearchResult::NodeSearch => cursor.search_in_node(key),
                _ => unreachable!(),
            };

            // if it needs to descend to child, do it.
            cursor = match cursor.result {
                SearchResult::Some { .. } => return cursor,
                SearchResult::Descent { edge_index } => cursor.descend(edge_index),
                _ => unreachable!(),
            };
        }
    }

    /// insert (key, value) and return root of the tree
    fn insert_recursive(&mut self, mut cursor: Cursor<K, V>, edge_index: usize, key: K, value: V) {
        let mut current = unsafe { cursor.current.as_mut() };

        let mut splitted = match current.insert_leaf(edge_index, key, value) {
            InsertResult::Fitted => return,
            InsertResult::Splitted { parent, right } => (parent, right),
        };

        let mut depth: usize = 1;

        // split & merge to maintain the invariant of B-Tree
        while let Some((mut ancestor, index)) = cursor.ancestors.pop() {
            current = unsafe { ancestor.as_mut() };

            let ((key, value), edge) = splitted;
            splitted = match current.insert_inner(index, key, value, edge) {
                InsertResult::Fitted => return,
                InsertResult::Splitted { parent, right } => (parent, right),
            };

            depth += 1;
        }

        let ((key, value), edge) = splitted;

        let mut root = Box::new(Node::new());
        root.size = 1;
        root.depth = depth;

        unsafe {
            ptr::write(root.keys.as_mut_ptr(), key);
            ptr::write(root.values.as_mut_ptr(), value);
            ptr::write(root.edges.as_mut_ptr(), Box::from_raw(current as *mut _));
            ptr::write(root.edges.as_mut_ptr().add(1), edge);
        }

        self.root = Box::leak(root).into();
    }

    fn remove_recursive(&mut self, mut cursor: Cursor<K, V>, value_index: usize) -> V {
        let current = unsafe { cursor.current.as_mut() };

        // let value = current.data.remove(value_index).1;
        // let value = current.values[value_index];

        let value = if current.depth == 0 {
            let value = unsafe {
                let _ = slice_remove(current.mut_keys(), value_index);
                slice_remove(current.mut_values(), value_index)
            };

            current.size -= 1;

            // if the leaf node has at least one or root, just return
            if current.size > 0 || unsafe { self.root.as_ref().depth } == 0 {
                return value;
            }

            value
        } else {
            // the current is internal node, then find predecessor node or successor node (key, value)

            // try replace with predecessor or successor
            // if the leaf node has at least two pairs of (key, value), just return after replacing since it does not need to rebalance
            let predecessor_edge = unsafe {
                &mut **(current.edges.get_unchecked_mut(value_index) as *mut Box<Node<K, V>>)
            };
            let (_, predecessor) = predecessor_edge.find_end();

            if predecessor.size > 1 {
                let idx = predecessor.size - 1;

                unsafe {
                    let swapped_k = slice_remove(predecessor.mut_keys(), idx);
                    let swapped_v = slice_remove(predecessor.mut_values(), idx);

                    predecessor.size -= 1;

                    ptr::write(current.keys.as_mut_ptr().add(value_index), swapped_k);
                    let value = mem::replace(&mut current.values[value_index], swapped_v);

                    return value;
                };
            } else {
                let successor_edge = unsafe {
                    &mut **(current.edges.get_unchecked_mut(value_index + 1)
                        as *mut Box<Node<K, V>>)
                };
                let (parents, successor) = successor_edge.find_begin();

                let value = unsafe {
                    let swapped_k = slice_remove(successor.mut_keys(), 0);
                    let swapped_v = slice_remove(successor.mut_values(), 0);

                    successor.size -= 1;

                    ptr::write(current.keys.as_mut_ptr().add(value_index), swapped_k);

                    mem::replace(&mut current.values[value_index], swapped_v)
                };

                if successor.size > 0 {
                    return value;
                }

                cursor.ancestors.push((cursor.current, value_index + 1));
                cursor.ancestors.extend(parents);

                value
            }
        };

        // start to move the empty node to root
        // I use left-hand rule
        while let Some((mut parent, edge_index)) = cursor.ancestors.pop() {
            let parent = unsafe { parent.as_mut() };
            // println!("parent: {:?}", parent);
            // let current = parent.edges[edge_index].as_mut();
            // the only one that uses right-hand rule since this is the rightmost node
            if edge_index == 0 {
                let right_sibling = unsafe {
                    &mut **(parent.edges.get_unchecked_mut(edge_index + 1) as *mut Box<Node<K, V>>)
                };

                // parent has one (key, value), therefore it is to be empty node.
                if parent.size == 1 {
                    debug_assert!(edge_index == 0);

                    if right_sibling.size == 1 {
                        // println!("CASE 1");
                        let current = unsafe { slice_remove(parent.mut_edges(), 0) };

                        right_sibling.size += 1;
                        unsafe {
                            slice_insert(
                                right_sibling.mut_keys(),
                                0,
                                ptr::read(parent.keys.as_ptr().add(0)),
                            );
                            slice_insert(
                                right_sibling.mut_values(),
                                0,
                                ptr::read(parent.values.as_ptr().add(0)),
                            );

                            if current.depth > 0 {
                                slice_insert(
                                    right_sibling.mut_edges(),
                                    0,
                                    ptr::read(current.edges.as_ptr().add(0)),
                                );
                            }
                        }
                        parent.size -= 1; // make empty node that has only one edge
                        debug_assert!(parent.size == 0);

                        mem::forget(current);
                    } else {
                        // println!("CASE 2");
                        let (new_parent_key, new_parent_value) = unsafe {
                            (
                                slice_remove(right_sibling.mut_keys(), 0),
                                slice_remove(right_sibling.mut_values(), 0),
                            )
                        };

                        let current = parent.edges[edge_index].as_mut();

                        current.size += 1;
                        unsafe {
                            ptr::write(
                                current.keys.as_mut_ptr().add(0),
                                mem::replace(&mut parent.keys[0], new_parent_key),
                            );
                            ptr::write(
                                current.values.as_mut_ptr().add(0),
                                mem::replace(&mut parent.values[0], new_parent_value),
                            );

                            if current.depth > 0 {
                                // current.size == 1
                                ptr::write(
                                    current.edges.as_mut_ptr().add(current.size),
                                    slice_remove(right_sibling.mut_edges(), 0),
                                );
                            }
                        }
                        right_sibling.size -= 1;

                        break;
                    }
                } else {
                    if right_sibling.size == 1 {
                        // println!("CASE 3");
                        right_sibling.size += 1;
                        unsafe {
                            slice_insert(
                                right_sibling.mut_keys(),
                                0,
                                slice_remove(parent.mut_keys(), edge_index),
                            );
                            slice_insert(
                                right_sibling.mut_values(),
                                0,
                                slice_remove(parent.mut_values(), edge_index),
                            );
                        }

                        let current = unsafe { slice_remove(parent.mut_edges(), edge_index) };
                        parent.size -= 1;

                        if current.depth > 0 {
                            unsafe {
                                slice_insert(
                                    right_sibling.mut_edges(),
                                    0,
                                    ptr::read(current.edges.as_ptr().add(0)),
                                );
                            }
                        }

                        mem::forget(current);
                        break;
                    } else {
                        // println!("CASE 4");
                        let current = unsafe {
                            &mut **(parent.edges.get_unchecked_mut(edge_index)
                                as *mut Box<Node<K, V>>)
                        };
                        current.size += 1;
                        unsafe {
                            ptr::write(
                                current.keys.as_mut_ptr().add(0),
                                slice_remove(parent.mut_keys(), edge_index),
                            );
                            ptr::write(
                                current.values.as_mut_ptr().add(0),
                                slice_remove(parent.mut_values(), edge_index),
                            );
                        };
                        debug_assert!(current.size == 1);

                        unsafe {
                            slice_insert(
                                parent.mut_keys(),
                                edge_index,
                                slice_remove(right_sibling.mut_keys(), 0),
                            );
                            slice_insert(
                                parent.mut_values(),
                                edge_index,
                                slice_remove(right_sibling.mut_values(), 0),
                            );

                            if current.depth > 0 {
                                let idx = current.size;

                                slice_insert(
                                    current.mut_edges(),
                                    idx,
                                    slice_remove(right_sibling.mut_edges(), 0),
                                );
                            }
                        }
                        right_sibling.size -= 1;

                        break;
                    }
                }
            } else {
                let left_sibling = unsafe {
                    &mut **(parent.edges.get_unchecked_mut(edge_index - 1) as *mut Box<Node<K, V>>)
                };

                if parent.size == 1 {
                    if left_sibling.size == 1 {
                        // println!("CASE 5");
                        let current = unsafe { ptr::read(parent.edges.as_ptr().add(edge_index)) };

                        // TODO: should use slice_insert?
                        left_sibling.size += 1;
                        unsafe {
                            ptr::write(
                                left_sibling.keys.as_mut_ptr().add(left_sibling.size - 1),
                                ptr::read(parent.keys.as_ptr().add(parent.size - 1)),
                            );
                            ptr::write(
                                left_sibling.values.as_mut_ptr().add(left_sibling.size - 1),
                                ptr::read(parent.values.as_ptr().add(parent.size - 1)),
                            );

                            if current.depth > 0 {
                                ptr::write(
                                    left_sibling.edges.as_mut_ptr().add(left_sibling.size),
                                    ptr::read(current.edges.as_ptr().add(0)),
                                );
                            }
                        };
                        parent.size -= 1;
                        debug_assert!(parent.size == 0);

                        mem::forget(current);
                    } else {
                        // CASE 6
                        // println!("CASE 6");
                        let current = parent.edges[edge_index].as_mut();

                        current.size += 1;
                        unsafe {
                            ptr::write(
                                current.keys.as_mut_ptr().add(0),
                                mem::replace(
                                    &mut parent.keys[parent.size - 1],
                                    ptr::read(
                                        left_sibling.keys.as_ptr().add(left_sibling.size - 1),
                                    ),
                                ),
                            );
                            ptr::write(
                                current.values.as_mut_ptr().add(0),
                                mem::replace(
                                    &mut parent.values[parent.size - 1],
                                    ptr::read(
                                        left_sibling.values.as_ptr().add(left_sibling.size - 1),
                                    ),
                                ),
                            );

                            if current.depth > 0 {
                                slice_insert(
                                    current.mut_edges(),
                                    0,
                                    ptr::read(left_sibling.edges.as_ptr().add(left_sibling.size)),
                                );
                            }
                        }
                        left_sibling.size -= 1;

                        break;
                    }
                } else {
                    if left_sibling.size == 1 {
                        // CASE 7
                        // println!("CASE 7");
                        left_sibling.size += 1;
                        unsafe {
                            ptr::write(
                                left_sibling.keys.as_mut_ptr().add(left_sibling.size - 1),
                                slice_remove(parent.mut_keys(), edge_index - 1),
                            );
                            ptr::write(
                                left_sibling.values.as_mut_ptr().add(left_sibling.size - 1),
                                slice_remove(parent.mut_values(), edge_index - 1),
                            );

                            let current = slice_remove(parent.mut_edges(), edge_index);

                            if current.depth > 0 {
                                ptr::write(
                                    left_sibling.edges.as_mut_ptr().add(left_sibling.size),
                                    ptr::read(current.edges.as_ptr().add(0)),
                                );
                            }

                            mem::forget(current);
                        }
                        parent.size -= 1;

                        break;
                    } else {
                        // CASE 8
                        // println!("CASE 8");
                        let current = parent.edges[edge_index].as_mut();

                        current.size += 1;
                        unsafe {
                            ptr::write(
                                current.keys.as_mut_ptr().add(0),
                                mem::replace(
                                    &mut parent.keys[edge_index - 1],
                                    ptr::read(
                                        left_sibling.keys.as_ptr().add(left_sibling.size - 1),
                                    ),
                                ),
                            );
                            ptr::write(
                                current.values.as_mut_ptr().add(0),
                                mem::replace(
                                    &mut parent.values[edge_index - 1],
                                    ptr::read(
                                        left_sibling.values.as_ptr().add(left_sibling.size - 1),
                                    ),
                                ),
                            );

                            if current.depth > 0 {
                                slice_insert(
                                    current.mut_edges(),
                                    0,
                                    ptr::read(left_sibling.edges.as_ptr().add(left_sibling.size)),
                                );
                            }
                        }
                        left_sibling.size -= 1;

                        break;
                    }
                }
            }
        }

        let root = unsafe { self.root.as_mut() };

        // root is now empty. Swap with unique edge
        if root.size == 0 {
            let old_root: Box<Node<K, V>> = unsafe { Box::from_raw(root as *mut _) };
            self.root = unsafe { Box::leak(ptr::read(old_root.edges.as_ptr().add(0))).into() };
            mem::forget(old_root);
        }

        value
    }

    pub fn assert(&self) {
        let root = unsafe { self.root.as_ref() };

        fn count_nodes<K: Ord, V>(
            node: &Node<K, V>,
            depth: usize,
            root_depth: usize,
            from: Option<&K>,
            to: Option<&K>,
        ) -> usize {
            if node.depth != root_depth {
                assert!(node.size > 0 && node.size <= B_MAX_NODES);
            }

            assert_eq!(node.depth, depth);

            if node.size > 0 {
                if let Some(from) = from {
                    assert!(from < node.keys.first().unwrap());
                }

                for two in node.keys().windows(2) {
                    assert!(two[0] < two[1]);
                }

                if let Some(to) = to {
                    assert!(node.keys().last().unwrap() < to);
                }
            }

            node.size
                + node
                    .edges()
                    .iter()
                    .enumerate()
                    .map(|(index, n)| {
                        let from = if index > 0 {
                            Some(&node.keys[index - 1])
                        } else {
                            None
                        };

                        let to = if index < node.size {
                            Some(&node.keys[index])
                        } else {
                            None
                        };
                        count_nodes(n, depth - 1, root_depth, from, to)
                    })
                    .sum::<usize>()
        }

        assert_eq!(
            count_nodes(root, root.depth, root.depth, None, None),
            self.size
        );
    }
}

impl<K, V> SequentialMap<K, V> for BTree<K, V>
where
    K: Ord + Clone + Debug,
    V: Debug,
{
    fn new() -> Self {
        Self {
            root: Box::leak(Box::new(Node::new())).into(),
            size: 0,
        }
    }

    fn insert(&mut self, key: &K, value: V) -> Result<(), V> {
        let cursor = self.find(key);

        match cursor.result {
            SearchResult::Some { .. } => Err(value),
            SearchResult::None { edge_index } => {
                self.insert_recursive(cursor, edge_index, key.clone(), value);
                self.size += 1;
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn lookup(&self, key: &K) -> Option<&V> {
        let cursor = self.find(key);

        match cursor.result {
            SearchResult::Some { value_index } => unsafe {
                Some(&cursor.current.as_ref().values[value_index])
            },
            SearchResult::None { .. } => None,
            _ => unreachable!(),
        }
    }

    fn remove(&mut self, key: &K) -> Result<V, ()> {
        let cursor = self.find(key);

        match cursor.result {
            SearchResult::Some { value_index } => {
                let value = self.remove_recursive(cursor, value_index);
                self.size -= 1;
                Ok(value)
            }
            SearchResult::None { .. } => Err(()),
            _ => unreachable!(),
        }
    }
}
