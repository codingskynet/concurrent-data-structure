use std::fmt::Debug;
use std::{cmp::Ordering, mem, ptr::NonNull};

use crate::map::SequentialMap;

const B_MAX_NODES: usize = 2;
const B_MID_INDEX: usize = B_MAX_NODES / 2;

// TODO: optimize with MaybeUninit
#[derive(Debug)]
struct Node<K, V> {
    size: usize,
    depth: usize,
    data: Vec<(K, V)>, // keys/values max size: B_MAX_NODES + 1 for violating invariant
    edges: Vec<Box<Node<K, V>>>, // max size: B_MAX_NODES + 2
}

impl<K, V> Node<K, V> {
    fn new() -> Self {
        Self {
            size: 0,
            depth: 0,
            data: Vec::with_capacity(B_MAX_NODES + 1),
            edges: Vec::with_capacity(B_MAX_NODES + 2),
        }
    }
}

// unsafe fn slice_insert<T>(ptr: &mut [T], index: usize, value: T) {
//     let size = ptr.len();
//     debug_assert!(size > index);

//     let ptr = ptr.as_mut_ptr();

//     if size > index + 1 {
//         ptr::copy(ptr.add(index), ptr.add(index + 1), size - index - 1);
//     }

//     *ptr.add(index) = value;
// }

// unsafe fn slice_remove<T>(ptr: &mut [T], index: usize) -> T {
//     let size = ptr.len();
//     debug_assert!(size > index);

//     let ptr = ptr.as_mut_ptr();
//     let value = ptr::read(ptr.add(index));

//     if size > index + 1 {
//         ptr::copy(ptr.add(index + 1), ptr.add(index), size - index - 1);
//     }

//     value
// }

enum InsertResult<K, V> {
    Fitted,
    Splitted {
        parent: (K, V),
        right: Box<Node<K, V>>,
    },
}

impl<K, V> Node<K, V>
where
    K: Debug,
    V: Debug,
{
    fn insert_leaf(&mut self, edge_index: usize, key: K, value: V) -> InsertResult<K, V> {
        self.size += 1;

        self.data.insert(edge_index, (key, value));

        if self.size <= B_MAX_NODES {
            return InsertResult::Fitted;
        }

        // violate B-Tree invariant, then split node
        let mut node = Box::new(Node::new());
        self.size = B_MAX_NODES / 2;
        node.size = B_MAX_NODES / 2;

        node.data = self.data.split_off(B_MID_INDEX + 1);

        let mid = self.data.pop().unwrap();

        InsertResult::Splitted {
            parent: mid,
            right: node,
        }
    }

    fn insert_inner(
        &mut self,
        edge_index: usize,
        key: K,
        value: V,
        edge: Box<Node<K, V>>,
    ) -> InsertResult<K, V> {
        self.size += 1;

        self.data.insert(edge_index, (key, value));
        self.edges.insert(edge_index + 1, edge);

        if self.size <= B_MAX_NODES {
            return InsertResult::Fitted;
        }

        // violate B-tree invariant, then split node with splitting edges
        let mut node = Box::new(Node::new());
        self.size = B_MAX_NODES / 2;
        node.size = B_MAX_NODES / 2;
        node.depth = self.depth;

        node.data = self.data.split_off(B_MID_INDEX + 1);
        node.edges = self.edges.split_off(B_MID_INDEX + 1);

        let mid = self.data.pop().unwrap();

        InsertResult::Splitted {
            parent: mid,
            right: node,
        }
    }
}

#[derive(Debug)]
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
        Self {
            ancestors: Vec::new(),
            current: tree.root,
            result: SearchResult::NodeSearch,
        }
    }

    fn search_in_node(self, key: &K) -> Self {
        let node = unsafe { self.current.as_ref() };

        for (index, (k, _)) in node.data.iter().enumerate() {
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
        match unsafe { self.current.as_mut().edges.get_mut(edge_index) } {
            Some(node) => {
                let parent = mem::replace(&mut self.current, NonNull::new(node.as_mut()).unwrap());
                self.ancestors.push((parent, edge_index));

                Self {
                    result: SearchResult::NodeSearch,
                    ..self
                }
            }
            None => Self {
                result: SearchResult::None { edge_index },
                ..self
            },
        }
    }
}

pub struct BTree<K, V> {
    root: NonNull<Node<K, V>>,
    size: usize,
}

impl<K, V> BTree<K, V>
where
    K: Ord + Debug,
    V: Debug,
{
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
        root.data.push((key, value));
        unsafe {
            root.edges.push(Box::from_raw(current as *mut _));
        }
        root.edges.push(edge);

        self.root = Box::leak(root).into();
    }

    fn remove_recursive(&mut self, mut cursor: Cursor<K, V>, value_index: usize) -> V {
        let current = unsafe { cursor.current.as_mut() };

        let value = current.data.remove(value_index).1;

        if current.depth == 0 {
            current.size -= 1;

            // exceptionally, if the root is leaf, just remove it
            if unsafe { self.root.as_ref().depth } == 0 {
                return value;
            }
        } else {
            // the current is internal node, then find predecessor node or successor node (key, value)

            // find predecessor
            let mut flag = false;
            {
                let mut parents: Vec<(NonNull<Node<K, V>>, usize)> =
                    vec![(cursor.current, value_index)];
                let mut target = NonNull::from(current.edges[value_index].as_mut());

                loop {
                    let target_mut = unsafe { target.as_mut() };

                    if target_mut.depth == 0 {
                        if target_mut.size == 1 {
                            break;
                        }

                        target_mut.size -= 1;
                        let swapped_datum = target_mut.data.pop().unwrap();
                        current.data.insert(value_index, swapped_datum);

                        cursor.ancestors.extend(parents);
                        flag = true;
                        break;
                    }

                    parents.push((target, target_mut.size));
                    target = NonNull::from(target_mut.edges.last_mut().unwrap().as_mut());
                }
            }

            // find successor
            if !flag {
                let mut parents: Vec<(NonNull<Node<K, V>>, usize)> =
                    vec![(cursor.current, value_index + 1)];
                let mut target = NonNull::from(current.edges[value_index + 1].as_mut());

                loop {
                    let target_mut = unsafe { target.as_mut() };

                    if target_mut.depth == 0 {
                        target_mut.size -= 1;
                        let swapped_datum = target_mut.data.pop().unwrap();

                        current.data.insert(value_index, swapped_datum);

                        cursor.ancestors.extend(parents);
                        break;
                    }

                    parents.push((target, target_mut.size));
                    target = NonNull::from(target_mut.edges.first_mut().unwrap().as_mut());
                }
            }
        }

        // there is no bubble since the leaf node has at least one (key, value)
        if current.depth == 0 && current.size > 0 {
            return value;
        }

        // start to move the empty node to root
        // I use left-hand rule
        while let Some((mut parent, edge_index)) = cursor.ancestors.pop() {
            let parent = unsafe { parent.as_mut() };
            // the only one that uses right-hand rule since this is the rightmost node
            if edge_index == 0 {
                let right_sibling_size = parent.edges[edge_index + 1].size;

                // parent has one (key, value), therefore it is to be empty node.
                if parent.size == 1 {
                    debug_assert!(edge_index == 0);

                    if right_sibling_size == 1 {
                        // CASE 1
                        let mut current = parent.edges.remove(0);
                        let right_sibling = parent.edges[edge_index].as_mut();

                        if let Some(edge) = current.edges.pop() {
                            right_sibling.edges.insert(0, edge);
                        } else {
                            debug_assert!(right_sibling.edges.len() == 0);
                        }

                        right_sibling.size += 1;
                        parent.size -= 1; // make empty node that has only one edge
                        debug_assert!(parent.size == 0);
                        right_sibling.data.insert(0, parent.data.pop().unwrap());

                        drop(current);
                    } else {
                        // CASE 2
                        let right_sibling = parent.edges[edge_index + 1].as_mut();
                        right_sibling.size -= 1;
                        let new_parent = right_sibling.data.remove(0);
                        let moved_edge = if right_sibling.depth != 0 {
                            Some(right_sibling.edges.remove(0))
                        } else {
                            None
                        };

                        let current = parent.edges[edge_index].as_mut();
                        current.size += 1;
                        current.data.push(parent.data.pop().unwrap());

                        if let Some(edge) = moved_edge {
                            current.edges.push(edge);
                        }

                        parent.data.push(new_parent);
                        break;
                    }
                } else {
                    if right_sibling_size == 1 {
                        // CASE 3
                        parent.size -= 1;
                        let new_sibling = parent.data.remove(edge_index);
                        let mut current = parent.edges.remove(edge_index);
                        let moved_edge = current.edges.pop();

                        let right_sibling = parent.edges[edge_index].as_mut();
                        right_sibling.size += 1;
                        right_sibling.data.insert(0, new_sibling);

                        if let Some(edge) = moved_edge {
                            right_sibling.edges.insert(0, edge);
                        }
                        drop(current);
                        break;
                    } else {
                        // CASE 4
                        let new_sibling = parent.data.remove(edge_index);
                        let right_sibling = parent.edges[edge_index + 1].as_mut();
                        right_sibling.size -= 1;
                        parent.data.insert(0, right_sibling.data.remove(0));

                        let moved_edge = if right_sibling.depth != 0 {
                            Some(right_sibling.edges.remove(0))
                        } else {
                            None
                        };

                        let current = parent.edges[edge_index].as_mut();
                        current.size += 1;
                        debug_assert!(current.size == 1);
                        current.data.push(new_sibling);

                        if let Some(edge) = moved_edge {
                            current.edges.push(edge);
                        }
                        break;
                    }
                }
            } else {
                let left_sibling = parent.edges[edge_index - 1].as_mut();

                if parent.size == 1 {
                    if left_sibling.size == 1 {
                        // CASE 5
                        let mut current = parent.edges.pop().unwrap();
                        let left_sibling = parent.edges.last_mut().unwrap();

                        if let Some(edge) = current.edges.pop() {
                            left_sibling.edges.push(edge);
                        } else {
                            debug_assert!(left_sibling.edges.len() == 0);
                        }

                        left_sibling.size += 1;
                        parent.size -= 1; // make empty node that has only one edge
                        debug_assert!(parent.size == 0);
                        left_sibling.data.push(parent.data.pop().unwrap());

                        drop(current);
                    } else {
                        // CASE 6
                        let left_sibling = parent.edges[edge_index - 1].as_mut();
                        left_sibling.size -= 1;
                        let new_parent = left_sibling.data.pop().unwrap();
                        let moved_edge = left_sibling.edges.pop();

                        let current = parent.edges[edge_index].as_mut();
                        current.size += 1;
                        current.data.push(parent.data.pop().unwrap());

                        if let Some(edge) = moved_edge {
                            current.edges.insert(0, edge);
                        }

                        parent.data.push(new_parent);
                        break;
                    }
                } else {
                    if left_sibling.size == 1 {
                        // CASE 7
                        parent.size -= 1;
                        let new_sibling = parent.data.remove(edge_index - 1);
                        let mut current = parent.edges.remove(edge_index);
                        let moved_edge = current.edges.pop();

                        let left_sibling = parent.edges[edge_index - 1].as_mut();
                        left_sibling.size += 1;
                        left_sibling.data.push(new_sibling);
                        if let Some(edge) = moved_edge {
                            left_sibling.edges.push(edge);
                        }
                        drop(current);
                        break;
                    } else {
                        // CASE 8
                        let new_sibling = parent.data.remove(edge_index - 1);
                        let left_sibling = parent.edges[edge_index - 1].as_mut();
                        left_sibling.size -= 1;
                        parent.data.push(left_sibling.data.pop().unwrap());
                        let moved_edge = left_sibling.edges.pop();

                        let current = parent.edges[edge_index].as_mut();
                        current.size += 1;
                        debug_assert!(current.size == 1);
                        current.data.insert(0, new_sibling);

                        if let Some(edge) = moved_edge {
                            current.edges.insert(0, edge);
                        }
                        break;
                    }
                }
            }
        }

        let root = unsafe { self.root.as_mut() };

        // root is now empty. Swap with unique edge
        if root.size == 0 {
            debug_assert!(root.edges.len() == 1);
            self.root = Box::leak(root.edges.pop().unwrap()).into();
        }

        value
    }

    pub fn print(&self)
    where
        K: Debug,
        V: Debug,
    {
        unsafe { println!("{:?}", self.root.as_ref()) };
    }

    pub fn assert(&self) {
        let root = unsafe { self.root.as_ref() };

        fn count_nodes<K, V>(node: &Node<K, V>, depth: usize, root_depth: usize) -> usize {
            if node.depth != root_depth {
                assert!(node.size > 0 && node.size <= B_MAX_NODES);
            }

            if depth != 0 {
                assert_eq!(node.size + 1, node.edges.len());
            }

            assert_eq!(node.size, node.data.len());
            assert_eq!(node.depth, depth);

            node.size
                + node
                    .edges
                    .iter()
                    .map(|node| count_nodes(node, depth - 1, root_depth))
                    .sum::<usize>()
        }

        assert_eq!(count_nodes(root, root.depth, root.depth), self.size);
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
                Some(&cursor.current.as_ref().data[value_index].1)
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
