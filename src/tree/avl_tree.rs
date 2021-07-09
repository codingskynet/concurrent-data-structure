use crate::map::SequentialMap;
use std::{cmp::max, mem, ptr::NonNull};

pub struct AVLTree<K: Ord + Clone, V> {
    root: NonNull<Box<Node<K, V>>>, // root node is dummy for simplicity
}

#[derive(Clone, Copy, PartialEq)]
enum Dir {
    Left,
    Eq,
    Right,
}

struct Node<K, V> {
    key: K,
    value: V,
    height: usize,
    left: Option<Box<Node<K, V>>>,
    right: Option<Box<Node<K, V>>>,
}

impl<K, V> Default for Node<K, V>
where
    K: Default,
    V: Default,
{
    fn default() -> Self {
        Node {
            key: K::default(),
            value: V::default(),
            height: 0,
            left: None,
            right: None,
        }
    }
}

impl<K, V> Node<K, V> {
    fn new(key: K, value: V) -> Node<K, V> {
        Node {
            key,
            value,
            height: 1,
            left: None,
            right: None,
        }
    }

    fn renew_height(&mut self) {
        let left_height = if let Some(node) = &self.left {
            node.height
        } else {
            0
        };

        let right_height = if let Some(node) = &self.right {
            node.height
        } else {
            0
        };

        self.height = max(left_height, right_height) + 1;
    }

    fn get_factor(&self) -> isize {
        let left_height = if let Some(node) = &self.left {
            node.height
        } else {
            0
        };

        let right_height = if let Some(node) = &self.right {
            node.height
        } else {
            0
        };

        if left_height > right_height {
            (left_height - right_height) as isize
        } else {
            -((right_height - left_height) as isize)
        }
    }

    fn rotate_left(mut node: Box<Node<K, V>>) -> Box<Node<K, V>> {
        let mut new_parent = node.right.take().unwrap();
        let _ = mem::replace(&mut node.right, new_parent.left);
        node.renew_height();

        new_parent.left = Some(node);
        new_parent.renew_height();

        new_parent
    }

    fn rotate_right(mut node: Box<Node<K, V>>) -> Box<Node<K, V>> {
        let mut new_parent = node.left.take().unwrap();
        let _ = mem::replace(&mut node.left, new_parent.right);
        node.renew_height();

        new_parent.right = Some(node);
        new_parent.renew_height();

        new_parent
    }
}

// manage each node's infomation
struct Cursor<K, V> {
    ancestors: Vec<(NonNull<Box<Node<K, V>>>, Dir)>,
    current: NonNull<Box<Node<K, V>>>,
    dir: Dir,
}

impl<'c, K: Default + Ord + Clone, V: Default> Cursor<K, V> {
    fn new(tree: &AVLTree<K, V>) -> Cursor<K, V> {
        Cursor {
            ancestors: vec![],
            current: tree.root,
            dir: Dir::Right,
        }
    }

    fn next_node(&self) -> Option<&Box<Node<K, V>>> {
        unsafe {
            match self.dir {
                Dir::Left => self.current.as_ref().left.as_ref(),
                Dir::Right => self.current.as_ref().right.as_ref(),
                Dir::Eq => panic!("The node is already arrived."),
            }
        }
    }

    fn next_node_mut(&mut self) -> &mut Option<Box<Node<K, V>>> {
        unsafe {
            match self.dir {
                Dir::Left => &mut self.current.as_mut().left,
                Dir::Right => &mut self.current.as_mut().right,
                Dir::Eq => panic!("The node is already arrived."),
            }
        }
    }

    fn move_next(&mut self) {
        unsafe {
            let next = match self.dir {
                Dir::Left => self.current.as_mut().left.as_mut().unwrap(),
                Dir::Right => self.current.as_mut().right.as_mut().unwrap(),
                Dir::Eq => panic!("The node is already arrived."),
            };

            let parent = mem::replace(&mut self.current, NonNull::new(next).unwrap());
            self.ancestors.push((parent, self.dir));
        }
    }

    fn rebalance(&mut self) {
        let parent_rotate_left = |mut node: Box<Node<K, V>>| -> Box<Node<K, V>> {
            let child_factor = node.right.as_ref().unwrap().get_factor();

            if child_factor > 0 {
                let right_child = node.right.take().unwrap();
                node.right = Some(Node::rotate_right(right_child));
            }

            Node::rotate_left(node)
        };

        let parent_rotate_right = |mut node: Box<Node<K, V>>| -> Box<Node<K, V>> {
            let child_factor = node.left.as_ref().unwrap().get_factor();

            if child_factor < 0 {
                let left_child = node.left.take().unwrap();
                node.left = Some(Node::rotate_left(left_child));
            }

            Node::rotate_right(node)
        };

        while let Some((mut node, dir)) = self.ancestors.pop() {
            // the root node for target node
            let root = unsafe { node.as_mut() };

            let target = match dir {
                Dir::Left => &mut root.left,
                Dir::Right => &mut root.right,
                _ => unreachable!(),
            };

            let factor = target.as_ref().unwrap().get_factor();

            match factor {
                -2 => *target = Some(parent_rotate_left(target.take().unwrap())),
                -1..=1 => {}
                2 => *target = Some(parent_rotate_right(target.take().unwrap())),
                _ => unreachable!(),
            }
        }
    }
}

impl<K, V> AVLTree<K, V>
where
    K: Default + Ord + Clone,
    V: Default,
{
    fn find(&self, key: &K) -> Cursor<K, V> {
        let mut cursor = Cursor::new(self);

        loop {
            if cursor.next_node().is_none() {
                return cursor;
            }

            cursor.move_next();

            unsafe {
                if *key == cursor.current.as_ref().key {
                    cursor.dir = Dir::Eq;
                    return cursor;
                } else if *key < cursor.current.as_ref().key {
                    cursor.dir = Dir::Left;
                } else {
                    // *key > next.key
                    cursor.dir = Dir::Right;
                }
            }
        }
    }
}

impl<K, V> SequentialMap<K, V> for AVLTree<K, V>
where
    K: Default + Ord + Clone,
    V: Default,
{
    fn new() -> Self {
        let mut root = Box::new(Node::default());

        AVLTree {
            root: NonNull::new(&mut root).unwrap(), // dummy node
        }
    }

    fn insert(&mut self, key: &K, value: V) -> Result<(), V> {
        let node = Box::new(Node::new(key.clone(), value));

        let mut cursor = self.find(key);

        if cursor.dir == Dir::Eq {
            return Err(node.value);
        }

        *(cursor.next_node_mut()) = Some(node);
        cursor.rebalance();

        Ok(())
    }

    fn lookup(&self, key: &K) -> Option<&V> {
        let cursor = self.find(key);

        unsafe {
            if cursor.dir == Dir::Eq {
                return Some(&cursor.current.as_ref().value);
            } else {
                return None;
            }
        }
    }

    fn remove(&mut self, key: &K) -> Result<V, ()> {
        todo!()
    }
}
