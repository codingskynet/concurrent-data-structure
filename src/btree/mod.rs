use std::{cmp::Ordering, mem::{self, MaybeUninit}, ops::DerefMut, ptr::{self, NonNull}};

use crate::map::SequentialMap;

const B_MAX_NODES: usize = 2;

// TODO: optimize with MaybeUninit
struct Node<K, V> {
    size: usize,
    keys: [Option<K>; B_MAX_NODES],
    values: [Option<V>; B_MAX_NODES],
    edges: [Option<Box<Node<K, V>>>; B_MAX_NODES + 1],
}

impl<K, V> Node<K, V> {
    fn new() -> Self {
        Self {
            size: 0,
            keys: Default::default(),
            values: Default::default(),
            edges: Default::default(),
        }
    }
}

unsafe fn slice_insert<T>(ptr: &mut [T], index: usize, value: T) {
    let size = ptr.len();
    debug_assert!(size > index);

    let ptr = ptr.as_mut_ptr();

    if size > index + 1{
        ptr::copy(ptr.add(index), ptr.add(index + 1), size - index - 1);
    }

    *ptr.add(index) = value;
}

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

impl<K, V> Node<K, V> {
    fn insert(&mut self, edge_index: usize, key: K, value: V) {
        if self.size < B_MAX_NODES {
            self.size += 1;

            unsafe {
                slice_insert(self.keys.get_unchecked_mut(..self.size), edge_index, Some(key));
                slice_insert(self.values.get_unchecked_mut(..self.size), edge_index, Some(value));
            }
        } else {
            // split & merge to maintain the invariant of B-Tree
            todo!()
        }
    }

    fn remove(&mut self, value_index: usize) -> V {
        if self.size > B_MAX_NODES / 2 {
            unsafe {
                let _ = slice_remove(self.keys.get_unchecked_mut(..self.size), value_index);
                let value = slice_remove(self.values.get_unchecked_mut(..self.size), value_index);

                self.size -= 1;
                value.unwrap()
            }
        } else {
            // merge & split to maintain the invariant of B-Tree
            todo!()
        }
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

        for (index, k) in node.keys[..node.size].iter().enumerate() {
            match key.cmp(k.as_ref().unwrap()) {
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
        match unsafe { self.current.as_mut().edges[edge_index].as_mut() } {
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
        let mut cursor = self.find(key);

        match cursor.result {
            SearchResult::Some { .. } => return Err(value),
            SearchResult::None { edge_index } => unsafe {
                cursor
                    .current
                    .as_mut()
                    .insert(edge_index, key.clone(), value)
            },
            _ => unreachable!(),
        }

        Ok(())
    }

    fn lookup(&self, key: &K) -> Option<&V> {
        let cursor = self.find(key);

        match cursor.result {
            SearchResult::Some { value_index } => unsafe {
                cursor.current.as_ref().values[value_index].as_ref()
            },
            SearchResult::None { .. } => None,
            _ => unreachable!(),
        }
    }

    fn remove(&mut self, key: &K) -> Result<V, ()> {
        let mut cursor = self.find(key);

        match cursor.result {
            SearchResult::Some { value_index } => unsafe { Ok(cursor.current.as_mut().remove(value_index)) },
            SearchResult::None { .. } => Err(()),
            _ => unreachable!(),
        }
    }
}
