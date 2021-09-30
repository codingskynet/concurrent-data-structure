use std::{
    cmp::Ordering,
    mem,
    ptr::NonNull,
};

use crate::map::SequentialMap;

const B_MAX_NODES: usize = 2;
const B_MID_INDEX: usize = B_MAX_NODES / 2;

// TODO: optimize with MaybeUninit
struct Node<K, V> {
    size: usize,
    keys: Vec<K>, // keys/values max size: B_MAX_NODES + 1 for violating invariant
    values: Vec<V>,
    edges: Vec<Box<Node<K, V>>>, // max size: B_MAX_NODES + 2
}

impl<K, V> Node<K, V> {
    fn new() -> Self {
        Self {
            size: 0,
            keys: Vec::with_capacity(B_MAX_NODES + 1),
            values: Vec::with_capacity(B_MAX_NODES + 1),
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
        // left: NonNull<Node<K, V>>,
        parent: (K, V),
        right: Box<Node<K, V>>,
    },
}

impl<K, V> Node<K, V> {
    fn insert_leaf(&mut self, edge_index: usize, key: K, value: V) -> InsertResult<K, V> {
        self.size += 1;

        self.keys.insert(edge_index, key);
        self.values.insert(edge_index, value);

        if self.size <= B_MAX_NODES {
            return InsertResult::Fitted;
        }

        // violate B-Tree invariant, then split node
        let mut node = Box::new(Node::new());
        self.size = B_MAX_NODES / 2;
        node.size = B_MAX_NODES / 2;
        node.keys = self.keys.split_off(B_MID_INDEX + 1);
        node.values = self.values.split_off(B_MID_INDEX + 1);

        let mid_key = node.keys.pop().unwrap();
        let mid_value = node.values.pop().unwrap();

       InsertResult::Splitted {
           parent: (mid_key, mid_value),
           right: node,
       }
    }

    fn insert_inner(&mut self, edge_index: usize, key: K, value: V, edge: Box<Node<K, V>>) -> InsertResult<K, V> {
        self.size += 1;

        self.keys.insert(edge_index, key);
        self.values.insert(edge_index, value);
        self.edges.insert(edge_index + 1, edge);

        if self.size <= B_MAX_NODES {
            return InsertResult::Fitted;
        }

        // violate B-tree invariant, then split node with splitting edges
        let mut node = Box::new(Node::new());
        self.size = B_MAX_NODES / 2;
        node.size = B_MAX_NODES / 2;
        node.keys = self.keys.split_off(B_MID_INDEX + 1);
        node.values = self.values.split_off(B_MID_INDEX + 1);
        node.edges = self.edges.split_off(B_MID_INDEX + 1);

        let mid_key = node.keys.pop().unwrap();
        let mid_value = node.values.pop().unwrap();

       InsertResult::Splitted {
           parent: (mid_key, mid_value),
           right: node,
       }
    }

    fn remove(&mut self, value_index: usize) -> V {
        todo!()
        // if self.size > B_MAX_NODES / 2 {
        //     unsafe {
        //         let _ = slice_remove(self.keys.get_unchecked_mut(..self.size), value_index);
        //         let value = slice_remove(self.values.get_unchecked_mut(..self.size), value_index);

        //         self.size -= 1;
        //         value.unwrap()
        //     }
        // } else {
        //     // merge & split to maintain the invariant of B-Tree
        //     todo!()
        // }
    }
}

struct Cursor<K, V> {
    ancestors: Vec<(NonNull<Node<K, V>>, usize)>, // (parent, index from parent.edges[index])
    current: NonNull<Node<K, V>>,
    result: SearchResult,
}

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
            result: SearchResult::Descent { edge_index: 0 }, // for beautiful recursive search, the root node is dummy
        }
    }

    fn search_in_node(self, key: &K) -> Self {
        let node = unsafe { self.current.as_ref() };

        for (index, k) in node.keys.iter().enumerate() {
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

    /// insert (key, value) and return root of the tree
    fn insert(mut self, edge_index: usize, key: K, value: V) -> NonNull<Node<K, V>> {
        let mut current = unsafe { self.current.as_mut() };

        let mut splitted = match current.insert_leaf(edge_index, key, value) {
            InsertResult::Fitted => {
                self.ancestors.push((self.current, edge_index));
                return self.ancestors.first().unwrap().0;
            },
            InsertResult::Splitted { parent, right } => (parent, right),
        };

        // split & merge to maintain the invariant of B-Tree
        while let Some((mut ancestor, index)) = self.ancestors.pop() {
            current = unsafe { ancestor.as_mut() };

            splitted = match current.insert_inner(index, splitted.0.0, splitted.0.1, splitted.1) {
                InsertResult::Fitted => {
                    self.ancestors.push((ancestor, index));
                    return self.ancestors.first().unwrap().0;
                },
                InsertResult::Splitted { parent, right } => (parent, right),
            }
        }

        let mut root = Box::new(Node::new());
        root.size = 1;
        root.keys.push(splitted.0.0);
        root.values.push(splitted.0.1);
        unsafe { root.edges.push(Box::from_raw(current as *mut _)); }
        root.edges.push(splitted.1);

        Box::leak(root).into()
    }

    fn remove(self, key: &K) -> V {
        // cursor.current.as_mut().remove(value_index)

        todo!()
    }
}

pub struct BTree<K, V> {
    root: NonNull<Node<K, V>>,
}

impl<K: Ord, V> BTree<K, V> {
    fn find(&self, key: &K) -> Cursor<K, V> {
        let mut cursor = Cursor::new(self);

        loop {
            cursor = match cursor.result {
                SearchResult::Some { .. } => return cursor,
                SearchResult::Descent { edge_index } => cursor.descend(edge_index),
                _ => unreachable!(),
            };

            cursor = match cursor.result {
                SearchResult::None { .. } => return cursor,
                SearchResult::NodeSearch => cursor.search_in_node(key),
                _ => unreachable!(),
            };
        }
    }
}

impl<K, V> SequentialMap<K, V> for BTree<K, V>
where
    K: Ord + Clone,
{
    fn new() -> Self {
        Self {
            root: Box::leak(Box::new(Node::new())).into(),
        }
    }

    fn insert(&mut self, key: &K, value: V) -> Result<(), V> {
        let cursor = self.find(key);

        match cursor.result {
            SearchResult::Some { .. } => Err(value),
            SearchResult::None { edge_index } => {
                self.root = cursor.insert(edge_index, key.clone(), value);
                Ok(())
            },
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
            SearchResult::Some { .. } => Ok(cursor.remove(key)),
            SearchResult::None { .. } => Err(()),
            _ => unreachable!(),
        }
    }
}
