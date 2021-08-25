use crossbeam_epoch::pin;
use crossbeam_epoch::Atomic;
use crossbeam_epoch::Guard;
use crossbeam_epoch::Owned;
use crossbeam_epoch::Shared;
use std::cmp::max;
use std::fmt::Debug;
use std::mem;
use std::mem::ManuallyDrop;
use std::sync::atomic::AtomicIsize;
use std::sync::atomic::Ordering;

use crate::lock::seqlock::ReadGuard;
use crate::lock::seqlock::SeqLock;
use crate::lock::seqlock::WriteGuard;
use crate::map::ConcurrentMap;

pub struct SeqLockAVLTree<K, V> {
    root: Atomic<Node<K, V>>,
}

#[derive(Debug)]
struct Node<K, V> {
    key: K,
    height: AtomicIsize,
    /// rwlock for shared mutable area
    inner: SeqLock<NodeInner<K, V>>,
}

#[derive(Debug)]
struct NodeInner<K, V> {
    value: Option<V>,
    left: Atomic<Node<K, V>>,
    right: Atomic<Node<K, V>>,
}

impl<K, V> NodeInner<K, V> {
    fn get_child(&self, dir: Dir) -> &Atomic<Node<K, V>> {
        match dir {
            Dir::Left => &self.left,
            Dir::Right => &self.right,
            Dir::Eq => unreachable!(),
        }
    }

    fn is_same_child(&self, dir: Dir, child: Shared<Node<K, V>>, guard: &Guard) -> bool {
        self.get_child(dir).load(Ordering::Relaxed, guard) == child
    }

    fn get_factor(&self, guard: &Guard) -> isize {
        let left = self.left.load(Ordering::Relaxed, guard);
        let right = self.right.load(Ordering::Relaxed, guard);

        let left_height = if !left.is_null() {
            unsafe { left.as_ref().unwrap().height.load(Ordering::Acquire) }
        } else {
            0
        };

        let right_height = if !right.is_null() {
            unsafe { right.as_ref().unwrap().height.load(Ordering::Acquire) }
        } else {
            0
        };

        left_height - right_height
    }

    fn get_new_height(&self, guard: &Guard) -> isize {
        let left = self.left.load(Ordering::Relaxed, guard);
        let right = self.right.load(Ordering::Relaxed, guard);

        let left = if !left.is_null() {
            unsafe { left.as_ref().unwrap().height.load(Ordering::Acquire) }
        } else {
            0
        };

        let right = if !right.is_null() {
            unsafe { right.as_ref().unwrap().height.load(Ordering::Acquire) }
        } else {
            0
        };

        max(left, right) + 1
    }
}

impl<K, V> Default for Node<K, V>
where
    K: Debug + Default,
    V: Debug + Default,
{
    fn default() -> Self {
        Self::new(K::default(), V::default())
    }
}

impl<K: Debug, V: Debug> Node<K, V> {
    fn new(key: K, value: V) -> Node<K, V> {
        Node {
            key,
            height: AtomicIsize::new(1),
            inner: SeqLock::new(NodeInner {
                value: Some(value),
                left: Atomic::null(),
                right: Atomic::null(),
            }),
        }
    }

    /// rotate left the node
    ///
    /// Change Parent-Right Child to Left Child-Parent, then return new parent(old right child).
    /// For simple managing locks, the function does not call lock, only use given lock guards.
    fn rotate_left<'g>(
        current: Shared<Node<K, V>>,
        current_guard: &WriteGuard<NodeInner<K, V>>,
        right_child_guard: &WriteGuard<NodeInner<K, V>>,
        guard: &'g Guard,
    ) -> Shared<'g, Node<K, V>> {
        let right_child_left_child = right_child_guard.left.load(Ordering::Relaxed, guard);
        let new_parent = current_guard
            .right
            .swap(right_child_left_child, Ordering::Relaxed, guard);
        right_child_guard.left.store(current, Ordering::Relaxed);

        new_parent
    }

    /// rotate right the node
    ///
    /// Change Left Child-Parent to Parent-Right Child, then return new parent(old left child).
    /// For simple managing locks, the function does not call lock, only use given lock guards.
    fn rotate_right<'g>(
        current: Shared<Node<K, V>>,
        current_guard: &WriteGuard<NodeInner<K, V>>,
        left_child_guard: &WriteGuard<NodeInner<K, V>>,
        guard: &'g Guard,
    ) -> Shared<'g, Node<K, V>> {
        let left_child_right_child = left_child_guard.right.load(Ordering::Relaxed, guard);
        let new_parent = current_guard
            .left
            .swap(left_child_right_child, Ordering::Relaxed, guard);
        left_child_guard.right.store(current, Ordering::Relaxed);

        new_parent
    }

    /// cleanup moving to ancestor
    ///
    /// If the node does not have full childs, delete it and move child to its position.
    /// If successing to defer_destroy it, return true else false.
    fn try_cleanup(
        current: Shared<Node<K, V>>,
        parent: Shared<Node<K, V>>,
        dir: Dir,
        guard: &Guard,
    ) -> bool {
        let parent_ref = unsafe { parent.as_ref().unwrap() };

        let current_ref = unsafe { current.as_ref().unwrap() };
        let read_guard = unsafe { current_ref.inner.read_lock() };

        // only already logically removed node can be cleaned up
        if read_guard.value.is_none() {
            let (left, right) = (
                read_guard.left.load(Ordering::Relaxed, guard).is_null(),
                read_guard.right.load(Ordering::Relaxed, guard).is_null(),
            );

            // if the node has one or zero node, the node can be directly removed
            if left || right {
                let parent_write_guard = parent_ref.inner.write_lock();

                // check if current's parent is even parent now
                if parent_write_guard.is_same_child(dir, current, guard) {
                    let write_guard = current_ref.inner.write_lock();

                    let (left, right) = (
                        write_guard.left.load(Ordering::Relaxed, guard),
                        write_guard.right.load(Ordering::Relaxed, guard),
                    );

                    // re-check if it can be removed
                    if write_guard.value.is_none() && (left.is_null() || right.is_null()) {
                        let replace_node = if !left.is_null() {
                            write_guard
                                .left
                                .swap(Shared::null(), Ordering::Relaxed, guard)
                        } else {
                            write_guard
                                .right
                                .swap(Shared::null(), Ordering::Relaxed, guard)
                        };

                        let current = parent_write_guard.get_child(dir).swap(
                            replace_node,
                            Ordering::Relaxed,
                            guard,
                        );

                        drop(parent_write_guard);
                        drop(write_guard);

                        // request deallocate removed node
                        unsafe {
                            guard.defer_destroy(current);
                        }

                        return true;
                    }
                }
            }
        }

        false
    }

    /*
    /// rebalance from current to grand_parent and renew all changed nodes
    ///
    /// If the relation among the nodes is not changed and the heights are needed to rotate, do it.
    fn try_rebalance<'g>(
        parent: Shared<Node<K, V>>,
        (root, root_dir): &(Shared<Node<K, V>>, Dir), // if rotating, root's child pointer should be rewritten
        guard: &'g Guard,
    ) {
        let parent_guard = unsafe { parent.as_ref().unwrap().inner.read().unwrap() };

        if (-1..=1).contains(&parent_guard.get_factor(guard)) {
            return;
        }

        drop(parent_guard);

        let root_guard = unsafe { root.as_ref().unwrap().inner.write().unwrap() };

        if !root_guard.is_same_child(*root_dir, parent, guard) {
            // The parent is separated from root between parent's read and write guard
            return;
        }

        let parent_ref = unsafe { parent.as_ref().unwrap() };
        let parent_guard = parent_ref.inner.write().unwrap();
        let mut current: Shared<Node<K, V>>;
        let mut current_guard: WriteGuard<NodeInner<K, V>>;

        if parent_guard.get_factor(guard) <= -2 {
            // R* rotation
            current = parent_guard.right.load(Ordering::Relaxed, guard);
            let current_ref = unsafe { current.as_ref().unwrap() };
            current_guard = current_ref.inner.write().unwrap();

            if current_guard.get_factor(guard) > 0 {
                // partial RL rotation
                let left_child = current_guard.left.load(Ordering::Relaxed, guard);

                let left_child_guard =
                    unsafe { left_child.as_ref().unwrap().inner.write().unwrap() };

                parent_guard.right.store(
                    Node::rotate_right(current, &current_guard, &left_child_guard, guard),
                    Ordering::Relaxed,
                );

                unsafe {
                    current
                        .as_ref()
                        .unwrap()
                        .height
                        .store(current_guard.get_new_height(guard), Ordering::Release)
                };

                current = left_child;
                current_guard = left_child_guard;
            }

            // RR rotation
            root_guard.get_child(*root_dir).store(
                Node::rotate_left(parent, &parent_guard, &current_guard, guard),
                Ordering::Relaxed,
            );
        } else if parent_guard.get_factor(guard) >= 2 {
            // L* rotation
            current = parent_guard.left.load(Ordering::Relaxed, guard);
            let current_ref = unsafe { current.as_ref().unwrap() };
            current_guard = current_ref.inner.write().unwrap();

            if current_guard.get_factor(guard) < 0 {
                // partial LR rotation
                let right_child = current_guard.right.load(Ordering::Relaxed, guard);

                let right_child_guard =
                    unsafe { right_child.as_ref().unwrap().inner.write().unwrap() };

                parent_guard.left.store(
                    Node::rotate_left(current, &current_guard, &right_child_guard, guard),
                    Ordering::Relaxed,
                );

                unsafe {
                    current
                        .as_ref()
                        .unwrap()
                        .height
                        .store(current_guard.get_new_height(guard), Ordering::Release)
                };

                current = right_child;
                current_guard = right_child_guard;
            }

            // LL rotation
            root_guard.get_child(*root_dir).store(
                Node::rotate_right(parent, &parent_guard, &current_guard, guard),
                Ordering::Relaxed,
            );
        } else {
            // The structure is changed stable between read guard and write guard.
            return;
        }

        unsafe {
            parent
                .as_ref()
                .unwrap()
                .height
                .store(parent_guard.get_new_height(guard), Ordering::Release);
            current
                .as_ref()
                .unwrap()
                .height
                .store(current_guard.get_new_height(guard), Ordering::Release);
        }
    }
    */
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Dir {
    Left,
    Eq,
    Right,
}

struct Cursor<'g, K, V> {
    ancestors: Vec<(Shared<'g, Node<K, V>>, ReadGuard<'g, NodeInner<K, V>>, Dir)>,
    current: Shared<'g, Node<K, V>>,
    /// the read lock for current node's inner
    /// It keeps current node's inner and is for hand-over-hand locking.
    inner_guard: ReadGuard<'g, NodeInner<K, V>>,
    dir: Dir,
}

impl<'g, K, V> Cursor<'g, K, V>
where
    K: Debug + PartialOrd,
    V: Debug,
{
    fn new(tree: &SeqLockAVLTree<K, V>, guard: &'g Guard) -> Cursor<'g, K, V> {
        let root = tree.root.load(Ordering::Relaxed, guard);
        let inner_guard = unsafe { root.as_ref().unwrap().inner.read_lock() };

        let cursor = Cursor {
            ancestors: vec![],
            current: root,
            inner_guard,
            dir: Dir::Right,
        };

        cursor
    }

    /// find the last state of the cursor by the key
    ///
    /// If there exists the key on the tree, the cursor's current is the node and the dir is Eq.
    /// If there does not exist the key on the tree, the cursor's current is leaf node and the dir is
    /// Left if the key is greater than the key of the node, or Right if the key is less than.
    fn find(&mut self, key: &K, guard: &'g Guard) {
        loop {
            if self.move_next(guard).is_err() {
                break;
            }

            unsafe {
                if *key == self.current.as_ref().unwrap().key {
                    self.dir = Dir::Eq;
                    break;
                } else if *key < self.current.as_ref().unwrap().key {
                    self.dir = Dir::Left;
                } else {
                    // *key > next.key
                    self.dir = Dir::Right;
                }
            }
        }
    }

    /// move to parent until self.inner_guard is valid
    fn recover(&mut self) {
        while let Some((parent, parent_read_guard, dir)) = self.ancestors.pop() {
            if parent_read_guard.validate() {
                self.current = parent;
                self.inner_guard = parent_read_guard;
                self.dir = dir;
                return;
            }

            // now parent is root
            if self.ancestors.is_empty() {
                self.current = parent;
                self.inner_guard = parent_read_guard;
                self.dir = dir;
                println!("Move to root!");
            }
        }
    }

    /// move the cursor to the direction using hand-over-hand locking
    ///
    /// If the cursor can move to next, return Ok(()) else Err(())
    /// The cursor's dir is never changed by any functions. You should change it manually like `cursor.dir = Dir::Left`.
    fn move_next(&mut self, guard: &'g Guard) -> Result<(), ()> {
        let next = match self.dir {
            Dir::Left => self.inner_guard.left.load(Ordering::Relaxed, guard),
            Dir::Right => self.inner_guard.right.load(Ordering::Relaxed, guard),
            Dir::Eq => panic!("The node is already arrived."),
        };

        if !self.inner_guard.validate() {
            // Optimistic read lock is failed. Retry
            // How to deal with the invalidated guard when it is due to rebalance?

            self.recover();
            return Ok(());
        }

        if next.is_null() {
            return Err(());
        }

        let next_guard = unsafe { next.as_ref().unwrap().inner.read_lock() };

        // replace with current's read guard, then store parent_guard in cursor
        let parent = mem::replace(&mut self.current, next);
        let parent_guard = mem::replace(&mut self.inner_guard, next_guard);
        self.ancestors.push((parent, parent_guard, self.dir));

        Ok(())
    }

    /// try to cleanup and rebalance the node
    /// TODO: manage repair operation by unique on current waiting list
    fn repair(mut cursor: Cursor<'g, K, V>, guard: &'g Guard) {
        while let Some((parent, _, dir)) = cursor.ancestors.pop() {
            if !Node::try_cleanup(cursor.current, parent, dir, guard) {
                {
                    let current = unsafe { cursor.current.as_ref().unwrap() };

                    unsafe {
                        while !current
                            .inner
                            .read(|read_guard| {
                                current
                                    .height
                                    .store(read_guard.get_new_height(guard), Ordering::Release);
                            })
                            .is_some()
                        {}
                    };
                }

                // the cursor.current is alive, so try rebalancing
                // if let Some(root_pair) = cursor.ancestors.last() {
                //     Node::try_rebalance(parent, root_pair, guard);
                // }
            }

            cursor.current = parent;
        }
    }
}

impl<K, V> SeqLockAVLTree<K, V>
where
    K: Default + Ord + Clone + Debug,
    V: Default + Debug,
{
    /// get the height of the tree
    pub fn get_height(&self, guard: &Guard) -> usize {
        unsafe {
            self.root
                .load(Ordering::Relaxed, guard)
                .as_ref()
                .unwrap()
                .inner
                .write_lock()
                .right
                .load(Ordering::Relaxed, guard)
                .as_ref()
                .unwrap()
                .height
                .load(Ordering::Acquire) as usize
        }
    }

    /// print tree structure
    pub fn print(&self, guard: &Guard) {
        fn print<K: Debug, V: Debug>(node: Shared<Node<K, V>>, guard: &Guard) -> String {
            if node.is_null() {
                return "null".to_string();
            }

            let node = unsafe { node.as_ref().unwrap() };
            let node_inner = node.inner.write_lock();

            format!(
                "{{key: {:?},  height: {}, value: {:?}, left: {}, right: {}}}",
                node.key,
                node.height.load(Ordering::SeqCst),
                node_inner.value,
                print(node_inner.left.load(Ordering::Relaxed, guard), guard),
                print(node_inner.right.load(Ordering::Relaxed, guard), guard)
            )
        }

        println!("{}", print(self.root.load(Ordering::Relaxed, guard), guard));
    }
}

impl<K, V> ConcurrentMap<K, V> for SeqLockAVLTree<K, V>
where
    K: Ord + Clone + Default + Debug,
    V: Clone + Default + Debug,
{
    fn new() -> Self {
        SeqLockAVLTree {
            root: Atomic::new(Node::default()),
        }
    }

    fn insert(&self, key: &K, value: V, guard: &Guard) -> Result<(), V> {
        let node = Node::new(key.clone(), value.clone());
        let mut cursor = Cursor::new(self, guard);

        loop {
            // if the cursor is invalid, then move up until cursor.inner_guard is valid
            cursor.recover();

            cursor.find(key, guard);

            if cursor.dir == Dir::Eq && cursor.inner_guard.value.is_some() {
                return Err(value);
            }

            // let current = unsafe { cursor.current.as_ref().unwrap() };
            let mut write_guard = if let Ok(guard) = cursor.inner_guard.clone().upgrade() {
                guard
            } else {
                continue;
            };

            // check if the current is alive now by checking parent node. If disconnected, retry
            if let Some((_, read_guard, dir)) = cursor.ancestors.last() {
                if !read_guard.is_same_child(*dir, cursor.current, guard) {
                    // Before inserting, the current is already disconnected.
                    continue;
                }
            }

            match cursor.dir {
                Dir::Left => {
                    if !write_guard.left.load(Ordering::Relaxed, guard).is_null() {
                        continue; // some thread already writed. Retry
                    }

                    write_guard.left.store(Owned::new(node), Ordering::Relaxed);
                }
                Dir::Right => {
                    if !write_guard.right.load(Ordering::Relaxed, guard).is_null() {
                        continue; // some thread already writed. Retry
                    }

                    write_guard.right.store(Owned::new(node), Ordering::Relaxed);
                }
                Dir::Eq => {
                    if write_guard.value.is_some() {
                        return Err(value);
                    }

                    write_guard.value = Some(value);
                }
            }

            drop(write_guard);

            Cursor::repair(cursor, guard);

            return Ok(());
        }
    }

    fn lookup(&self, key: &K, guard: &Guard) -> Option<V> {
        let mut cursor = Cursor::new(self, guard);
        cursor.find(key, guard);

        if cursor.dir == Dir::Eq {
            return cursor.inner_guard.value.clone();
        } else {
            return None;
        }
    }

    fn remove(&self, key: &K, guard: &Guard) -> Result<V, ()> {
        let mut cursor = Cursor::new(self, guard);
        cursor.find(key, guard);

        if cursor.dir != Dir::Eq {
            return Err(());
        }

        let mut write_guard = if let Ok(guard) = cursor.inner_guard.clone().upgrade() {
            guard
        } else {
            return Err(());
        };

        if let Some(value) = (*write_guard).value.take() {
            drop(write_guard);
            Cursor::repair(cursor, guard);
            return Ok(value);
        } else {
            return Err(());
        }
    }
}

impl<K, V> Drop for SeqLockAVLTree<K, V> {
    fn drop(&mut self) {
        let pin = pin();
        let mut nodes = vec![mem::replace(&mut self.root, Atomic::null())];
        while let Some(node) = nodes.pop() {
            let node = unsafe { node.into_owned() };
            let mut write_guard = node.inner.write_lock();

            let left = mem::replace(&mut write_guard.left, Atomic::null());
            let right = mem::replace(&mut write_guard.right, Atomic::null());

            if !left.load(Ordering::Relaxed, &pin).is_null() {
                nodes.push(left);
            }
            if !right.load(Ordering::Relaxed, &pin).is_null() {
                nodes.push(right);
            }
        }
    }
}
