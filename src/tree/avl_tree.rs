use crate::map::SequentialMap;
use std::{
    cmp::max,
    fmt::Debug,
    mem,
    ops::DerefMut,
    ptr::{drop_in_place, NonNull},
};

// how to show the structure of node
// use: unsafe { println!("Show tree info:\n{:?}", self.root.as_ref()) };
pub struct AVLTree<K: Ord + Clone, V> {
    root: NonNull<Node<K, V>>, // root node is dummy for simplicity
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Dir {
    Left,
    Eq,
    Right,
}

#[derive(Debug)]
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
        Self::new(K::default(), V::default())
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

    fn child_mut(&mut self, dir: Dir) -> &mut Option<Box<Node<K, V>>> {
        match dir {
            Dir::Left => &mut self.left,
            Dir::Right => &mut self.right,
            Dir::Eq => panic!("There is no 'eq' child"),
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
        new_parent.left = Some(node);

        new_parent
    }

    fn rotate_right(mut node: Box<Node<K, V>>) -> Box<Node<K, V>> {
        let mut new_parent = node.left.take().unwrap();
        let _ = mem::replace(&mut node.left, new_parent.right);
        new_parent.right = Some(node);

        new_parent
    }
}

// manage each node's infomation
#[derive(Debug)]
struct Cursor<K, V> {
    ancestors: Vec<(NonNull<Node<K, V>>, Dir)>,
    current: NonNull<Node<K, V>>,
    dir: Dir,
}

impl<'c, K, V> Cursor<K, V>
where
    K: Default + Ord + Clone + Debug,
    V: Default + Debug,
{
    fn new(tree: &AVLTree<K, V>) -> Cursor<K, V> {
        let cursor = Cursor {
            ancestors: vec![],
            current: tree.root,
            dir: Dir::Right,
        };

        cursor
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

            let parent = mem::replace(&mut self.current, NonNull::new(next.deref_mut()).unwrap());
            self.ancestors.push((parent, self.dir));
        }
    }

    fn find_largest_on_left_subtree(&mut self) {
        if self.dir != Dir::Eq {
            panic!("The node is not arrived at Eq.")
        }

        self.dir = Dir::Left;
        if self.next_node().is_none() {
            self.dir = Dir::Eq;
            return;
        }
        self.move_next();

        self.dir = Dir::Right;
        while self.next_node().is_some() {
            self.move_next();
        }

        self.dir = Dir::Eq;
    }

    fn rebalance(&mut self) {
        let parent_rotate_left = |mut node: Box<Node<K, V>>| -> Box<Node<K, V>> {
            let child_factor = node.right.as_ref().unwrap().get_factor();

            if child_factor > 0 {
                let right_child = node.right.take().unwrap();
                let mut right_child = Node::rotate_right(right_child);
                right_child.right.as_mut().unwrap().renew_height();
                node.right = Some(right_child);
            }

            Node::rotate_left(node)
        };

        let parent_rotate_right = |mut node: Box<Node<K, V>>| -> Box<Node<K, V>> {
            let child_factor = node.left.as_ref().unwrap().get_factor();

            if child_factor < 0 {
                let left_child = node.left.take().unwrap();
                let mut left_child = Node::rotate_left(left_child);
                left_child.left.as_mut().unwrap().renew_height();
                node.left = Some(left_child);
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
                -2 => {
                    let mut new_target = parent_rotate_left(target.take().unwrap());
                    new_target.left.as_mut().unwrap().renew_height();
                    new_target.renew_height();
                    *target = Some(new_target);
                }
                -1..=1 => target.as_mut().unwrap().renew_height(),
                2 => {
                    let mut new_target = parent_rotate_right(target.take().unwrap());
                    new_target.right.as_mut().unwrap().renew_height();
                    new_target.renew_height();
                    *target = Some(new_target);
                }
                _ => unreachable!(),
            }
        }
    }
}

impl<K, V> AVLTree<K, V>
where
    K: Default + Ord + Clone + Debug,
    V: Default + Debug,
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

    pub fn get_height(&self) -> usize {
        unsafe { self.root.as_ref().right.as_ref().unwrap().height }
    }
}

impl<K, V> SequentialMap<K, V> for AVLTree<K, V>
where
    K: Default + Ord + Clone + Debug,
    V: Default + Debug,
{
    fn new() -> Self {
        let root = Box::new(Node::default());

        let tree = AVLTree {
            root: Box::leak(root).into(),
        };

        tree
    }

    fn insert(&mut self, key: &K, value: V) -> Result<(), V> {
        let node = Box::new(Node::new(key.clone(), value));

        let mut cursor = self.find(key);

        if cursor.dir == Dir::Eq {
            return Err(node.value);
        }

        *(cursor.next_node_mut()) = Some(node);

        unsafe {
            cursor.current.as_mut().renew_height();
        }
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
        let mut cursor = self.find(key);

        if cursor.dir != Dir::Eq {
            return Err(());
        }

        let current = unsafe { cursor.current.as_ref() };

        match (current.left.is_some(), current.right.is_some()) {
            // find largest node from left subtree, swap, and remove
            (true, true) => {
                let (mut parent, dir) = cursor.ancestors.last_mut().unwrap();
                let child = unsafe { parent.as_mut().child_mut(*dir).as_mut().unwrap() };

                cursor.find_largest_on_left_subtree();

                let (mut swap_node_parent, dir) = cursor.ancestors.pop().unwrap();
                let swap_node_ptr = unsafe { swap_node_parent.as_mut().child_mut(dir) };
                let swap_node = swap_node_ptr.as_mut().unwrap();

                mem::swap(&mut child.key, &mut swap_node.key);
                mem::swap(&mut child.value, &mut swap_node.value);

                let swap_node = swap_node_ptr.take().unwrap();
                if swap_node.left.is_some() {
                    *swap_node_ptr = swap_node.left;
                }

                cursor.rebalance();

                Ok(swap_node.value)
            }
            (true, false) => {
                let (mut parent, dir) = cursor.ancestors.pop().unwrap();
                let child = unsafe { parent.as_mut().child_mut(dir) };
                let node = child.take().unwrap();
                *child = node.left;
                cursor.rebalance();

                Ok(node.value)
            }
            (false, true) => {
                let (mut parent, dir) = cursor.ancestors.pop().unwrap();
                let child = unsafe { parent.as_mut().child_mut(dir) };
                let node = child.take().unwrap();
                *child = node.right;
                cursor.rebalance();

                Ok(node.value)
            }
            (false, false) => {
                let (mut parent, dir) = cursor.ancestors.pop().unwrap();
                let node = unsafe { parent.as_mut().child_mut(dir).take() };
                cursor.rebalance();

                Ok(node.unwrap().value)
            }
        }
    }
}

impl<K: Ord + Clone, V> Drop for AVLTree<K, V> {
    fn drop(&mut self) {
        // since the struct had 'pointer' instead of 'ownership' of the root,
        // manually drop the root. Then, the childs are dropped recursively.
        unsafe { drop_in_place(self.root.as_mut()) };
    }
}
