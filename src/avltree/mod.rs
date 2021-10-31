mod rwlock;
mod seqlock;

pub use rwlock::RwLockAVLTree;
pub use seqlock::SeqLockAVLTree;

use crate::map::SequentialMap;
use std::{
    cmp::max,
    fmt::Debug,
    mem,
    ops::DerefMut,
    ptr::{drop_in_place, NonNull},
    usize,
};

pub struct AVLTree<K, V> {
    root: NonNull<Node<K, V>>, // root node is dummy for simplicity
}

impl<K: Debug, V: Debug> Debug for AVLTree<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            f.debug_struct("AVLTree")
                .field("root", self.root.as_ref())
                .finish()
        }
    }
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
    height: isize,
    left: Option<Box<Node<K, V>>>,
    right: Option<Box<Node<K, V>>>,
}

impl<K: Default, V: Default> Default for Node<K, V> {
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

    /// get the mutable reference of the child of the node by dir
    fn child_mut(&mut self, dir: Dir) -> &mut Option<Box<Node<K, V>>> {
        match dir {
            Dir::Left => &mut self.left,
            Dir::Right => &mut self.right,
            Dir::Eq => panic!("There is no 'Eq' child"),
        }
    }

    /// renew the height of the node from the childs
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

    /// get difference of the heights from the childs
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

        left_height - right_height
    }

    /// rotate left the node
    ///
    /// Change Parent-Right Child to Left Child-Parent, then return new parent(old right child).
    fn rotate_left(mut node: Box<Node<K, V>>) -> Box<Node<K, V>> {
        let mut new_parent = node.right.take().unwrap();
        let _ = mem::replace(&mut node.right, new_parent.left);
        new_parent.left = Some(node);

        new_parent
    }

    /// rotate right the node
    ///
    /// Change Left Child-Parent to Parent-Right Child, then return new parent(old left child).
    fn rotate_right(mut node: Box<Node<K, V>>) -> Box<Node<K, V>> {
        let mut new_parent = node.left.take().unwrap();
        let _ = mem::replace(&mut node.left, new_parent.right);
        new_parent.right = Some(node);

        new_parent
    }
}

/// manage the current state of the node
///
/// ancestors: the parents of the node
/// current: the node which it sees now.
/// dir: the direction that it moves on next. If Eq, the cursor cannot move since it arrived the destination node.
struct Cursor<K, V> {
    ancestors: Vec<(NonNull<Node<K, V>>, Dir)>,
    current: NonNull<Node<K, V>>,
    dir: Dir,
}

impl<'c, K, V> Cursor<K, V>
where
    K: Default + Ord + Clone,
    V: Default,
{
    fn new(tree: &AVLTree<K, V>) -> Cursor<K, V> {
        let cursor = Cursor {
            ancestors: Vec::with_capacity(tree.get_height() + 1),
            current: tree.root,
            dir: Dir::Right,
        };

        cursor
    }

    /// get the immutable reference of the next node by the direction
    fn next_node(&self) -> Option<&Box<Node<K, V>>> {
        unsafe {
            match self.dir {
                Dir::Left => self.current.as_ref().left.as_ref(),
                Dir::Right => self.current.as_ref().right.as_ref(),
                Dir::Eq => panic!("The node is already arrived."),
            }
        }
    }

    /// get the mutable reference of the next node by the direction
    fn next_node_mut(&mut self) -> &mut Option<Box<Node<K, V>>> {
        unsafe {
            match self.dir {
                Dir::Left => &mut self.current.as_mut().left,
                Dir::Right => &mut self.current.as_mut().right,
                Dir::Eq => panic!("The node is already arrived."),
            }
        }
    }

    /// move the cursor to the direction
    ///
    /// The cursor's dir is never changed by any functions. You should change it manually like `cursor.dir = Dir::Left`.
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

    /// move the node that has the greatest key on the left subtree
    ///
    /// This function is for removing the node that has two nodes.
    fn move_greatest_on_left_subtree(&mut self) {
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

    /// rebalance the nodes by the rule of AVL using the cursor's ancestors
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
    K: Default + Ord + Clone,
    V: Default,
{
    /// find the last state of the cursor by the key
    ///
    /// If there exists the key on the tree, the cursor's current is the node and the dir is Eq.
    /// If there does not exist the key on the tree, the cursor's current is leaf node and the dir is
    /// Left if the key is greater than the key of the node, or Right if the key is less than.
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

    /// get the height of the tree
    pub fn get_height(&self) -> usize {
        if let Some(node) = unsafe { self.root.as_ref().right.as_ref() } {
            node.height as usize
        } else {
            0
        }
    }
}

impl<K, V> SequentialMap<K, V> for AVLTree<K, V>
where
    K: Default + Ord + Clone,
    V: Default,
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

        let (left, right) = (current.left.is_some(), current.right.is_some());

        // special case: find largest node from left subtree, swap, and remove
        if left && right {
            let (mut parent, dir) = cursor.ancestors.last_mut().unwrap();
            let child = unsafe { parent.as_mut().child_mut(*dir).as_mut().unwrap() };

            cursor.move_greatest_on_left_subtree();

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

            return Ok(swap_node.value);
        }

        let (mut parent, dir) = cursor.ancestors.pop().unwrap();
        let child = unsafe { parent.as_mut().child_mut(dir) };
        let node = child.take().unwrap();

        if left {
            *child = node.left;
        } else if right {
            *child = node.right;
        }

        cursor.rebalance();
        Ok(node.value)
    }
}

impl<K, V> Drop for AVLTree<K, V> {
    fn drop(&mut self) {
        // since the struct had 'pointer' instead of 'ownership' of the root,
        // manually drop the root. Then, the childs are dropped recursively.
        unsafe { drop_in_place(self.root.as_mut()) };
    }
}
