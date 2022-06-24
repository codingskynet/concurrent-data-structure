use crossbeam_epoch::pin;
use crossbeam_epoch::unprotected;
use crossbeam_epoch::Atomic;
use crossbeam_epoch::Guard;
use crossbeam_epoch::Owned;
use crossbeam_epoch::Shared;
use crossbeam_utils::sync::ShardedLock;
use crossbeam_utils::sync::ShardedLockReadGuard;
use crossbeam_utils::sync::ShardedLockWriteGuard;
use std::cmp::max;
use std::fmt::Debug;
use std::mem;
use std::mem::ManuallyDrop;
use std::sync::atomic::AtomicIsize;
use std::sync::atomic::Ordering;

use crate::map::ConcurrentMap;

struct Node<K, V> {
    key: K,
    height: AtomicIsize,
    inner: ShardedLock<NodeInner<K, V>>,
}

impl<K: Debug, V: Debug> Debug for Node<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("key", &self.key)
            .field("height", &self.height.load(Ordering::Relaxed))
            .field("inner", &*self.inner.read().unwrap())
            .finish()
    }
}
struct NodeInner<K, V> {
    value: Option<V>,
    left: Atomic<Node<K, V>>,
    right: Atomic<Node<K, V>>,
}

impl<K: Debug, V: Debug> Debug for NodeInner<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            let mut result = f.debug_struct("NodeInner");
            result.field("value", &self.value);

            if let Some(left) = self.left.load(Ordering::Acquire, unprotected()).as_ref() {
                result.field("left", &left);
            } else {
                result.field("left", &"null");
            }

            if let Some(right) = self.right.load(Ordering::Acquire, unprotected()).as_ref() {
                result.field("right", &right);
            } else {
                result.field("right", &"null");
            }

            result.finish()
        }
    }
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

impl<K: Default, V: Default> Default for Node<K, V> {
    fn default() -> Self {
        Self::new(K::default(), V::default())
    }
}

impl<K, V> Node<K, V> {
    fn new(key: K, value: V) -> Node<K, V> {
        Node {
            key,
            height: AtomicIsize::new(1),
            inner: ShardedLock::new(NodeInner {
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
        current_guard: &ShardedLockWriteGuard<NodeInner<K, V>>,
        right_child_guard: &ShardedLockWriteGuard<NodeInner<K, V>>,
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
        current_guard: &ShardedLockWriteGuard<NodeInner<K, V>>,
        left_child_guard: &ShardedLockWriteGuard<NodeInner<K, V>>,
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
        let read_guard = current_ref.inner.read().unwrap();

        // only already logically removed node can be cleaned up
        if read_guard.value.is_none() {
            let (left, right) = (
                read_guard.left.load(Ordering::Relaxed, guard).is_null(),
                read_guard.right.load(Ordering::Relaxed, guard).is_null(),
            );

            // if the node has one or zero node, the node can be directly removed
            if left || right {
                drop(read_guard);

                let parent_write_guard = parent_ref.inner.write().unwrap();

                // check if current's parent is even parent now
                if parent_write_guard.is_same_child(dir, current, guard) {
                    let write_guard = current_ref.inner.write().unwrap();

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
        let mut current_guard: ShardedLockWriteGuard<NodeInner<K, V>>;

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
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Dir {
    Left,
    Eq,
    Right,
}

struct Cursor<'g, K, V> {
    ancestors: Vec<(Shared<'g, Node<K, V>>, Dir)>,
    current: Shared<'g, Node<K, V>>,
    /// the read lock for current node's inner
    /// It keeps current node's inner and is for hand-over-hand locking.
    inner_guard: ManuallyDrop<ShardedLockReadGuard<'g, NodeInner<K, V>>>,
    dir: Dir,
}

impl<'g, K, V> Cursor<'g, K, V> {
    fn new(tree: &RwLockAVLTree<K, V>, guard: &'g Guard) -> Cursor<'g, K, V> {
        let root = tree.root.load(Ordering::Relaxed, guard);
        let inner_guard = unsafe { root.as_ref().unwrap().inner.read().unwrap() };

        let cursor = Cursor {
            ancestors: Vec::with_capacity(tree.get_height() + 5),
            current: root,
            inner_guard: ManuallyDrop::new(inner_guard),
            dir: Dir::Right,
        };

        cursor
    }

    /// move the cursor to the direction using hand-over-hand locking
    ///
    /// The cursor's dir is never changed by any functions. You should change it manually like `cursor.dir = Dir::Left`.
    fn move_next(&mut self, guard: &'g Guard) -> Result<(), ()> {
        let next = match self.dir {
            Dir::Left => self.inner_guard.left.load(Ordering::Relaxed, guard),
            Dir::Right => self.inner_guard.right.load(Ordering::Relaxed, guard),
            Dir::Eq => panic!("The node is already arrived."),
        };

        if next.is_null() {
            return Err(());
        }

        let next_node = unsafe { next.as_ref().unwrap() };
        let next_guard = next_node.inner.read().unwrap();

        let parent = mem::replace(&mut self.current, next);
        self.ancestors.push((parent, self.dir));

        // replace with current's read guard, then unlock parent read guard by dropping
        let mut parent_guard = mem::replace(&mut self.inner_guard, ManuallyDrop::new(next_guard));

        unsafe {
            ManuallyDrop::drop(&mut parent_guard);
        }

        Ok(())
    }

    /// try to cleanup and rebalance the node
    /// TODO: manage repair operation by unique on current waiting list
    fn repair(mut cursor: Cursor<'g, K, V>, guard: &'g Guard) {
        while let Some((parent, dir)) = cursor.ancestors.pop() {
            if !Node::try_cleanup(cursor.current, parent, dir, guard) {
                {
                    let current = unsafe { cursor.current.as_ref().unwrap() };
                    let current_guard = current.inner.read().unwrap();

                    current
                        .height
                        .store(current_guard.get_new_height(guard), Ordering::Release);
                }

                // the cursor.current is alive, so try rebalancing
                if let Some(root_pair) = cursor.ancestors.last() {
                    Node::try_rebalance(parent, root_pair, guard);
                }
            }

            cursor.current = parent;
        }
    }
}

pub struct RwLockAVLTree<K, V> {
    root: Atomic<Node<K, V>>,
}

impl<K: Debug, V: Debug> Debug for RwLockAVLTree<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            f.debug_struct("RwLockAVLTree")
                .field(
                    "root",
                    self.root
                        .load(Ordering::Acquire, unprotected())
                        .as_ref()
                        .unwrap(),
                )
                .finish()
        }
    }
}

impl<K, V> RwLockAVLTree<K, V> {
    /// find the last state of the cursor by the key
    ///
    /// If there exists the key on the tree, the cursor's current is the node and the dir is Eq.
    /// If there does not exist the key on the tree, the cursor's current is leaf node and the dir is
    /// Left if the key is greater than the key of the node, or Right if the key is less than.
    fn find<'g>(&self, key: &K, guard: &'g Guard) -> Cursor<'g, K, V>
    where
        K: Ord,
    {
        let mut cursor = Cursor::new(self, guard);

        loop {
            if cursor.move_next(guard).is_err() {
                return cursor;
            }

            unsafe {
                if *key == cursor.current.as_ref().unwrap().key {
                    cursor.dir = Dir::Eq;
                    return cursor;
                } else if *key < cursor.current.as_ref().unwrap().key {
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
        unsafe {
            if let Some(node) = self
                .root
                .load(Ordering::Relaxed, &pin())
                .as_ref()
                .unwrap()
                .inner
                .read()
                .unwrap()
                .right
                .load(Ordering::Acquire, &pin())
                .as_ref()
            {
                node.height.load(Ordering::Relaxed) as usize
            } else {
                0
            }
        }
    }
}

impl<K, V> ConcurrentMap<K, V> for RwLockAVLTree<K, V>
where
    K: Ord + Clone + Default,
    V: Default,
{
    fn new() -> Self {
        RwLockAVLTree {
            root: Atomic::new(Node::default()),
        }
    }

    fn insert(&self, key: &K, value: V) -> Result<(), V> {
        let guard = pin();

        let node = Node::new(key.clone(), value);

        // TODO: it can be optimized by re-search nearby ancestors
        loop {
            let mut cursor = self.find(key, &guard);

            // unlock read lock and lock write lock... very inefficient, need upgrade from read lock to write lock
            unsafe {
                ManuallyDrop::drop(&mut cursor.inner_guard);
            }

            if cursor.dir == Dir::Eq && cursor.inner_guard.value.is_some() {
                let node_inner = node.inner.into_inner().unwrap();
                return Err(node_inner.value.unwrap());
            }

            let current = unsafe { cursor.current.as_ref().unwrap() };

            // check if the current is alive now by checking parent node. If disconnected, retry
            // TODO: is it efficient? It needs to check only whether the current is connected, not checking the current's parent is changed.
            let parent_read_guard = if let Some((parent, dir)) = cursor.ancestors.last() {
                let parent_read_guard = unsafe { parent.as_ref().unwrap().inner.read().unwrap() };
                if !parent_read_guard.is_same_child(*dir, cursor.current, &guard) {
                    // Before inserting, the current is already disconnected.
                    continue;
                }
                Some(parent_read_guard)
            } else {
                None
            };

            let mut write_guard = current.inner.write().unwrap();

            drop(parent_read_guard);

            match cursor.dir {
                Dir::Left => {
                    if !write_guard.left.load(Ordering::Relaxed, &guard).is_null() {
                        continue; // some thread already writed. Retry
                    }

                    write_guard.left.store(Owned::new(node), Ordering::Relaxed);
                }
                Dir::Right => {
                    if !write_guard.right.load(Ordering::Relaxed, &guard).is_null() {
                        continue; // some thread already writed. Retry
                    }

                    write_guard.right.store(Owned::new(node), Ordering::Relaxed);
                }
                Dir::Eq => {
                    let value = node.inner.into_inner().unwrap().value.unwrap();

                    if write_guard.value.is_some() {
                        return Err(value);
                    }

                    write_guard.value = Some(value);
                }
            }

            drop(write_guard);

            Cursor::repair(cursor, &guard);

            return Ok(());
        }
    }

    fn lookup<F, R>(&self, key: &K, f: F) -> R
    where
        F: FnOnce(Option<&V>) -> R,
    {
        let guard = pin();

        let mut cursor = self.find(key, &guard);

        if cursor.dir == Dir::Eq {
            unsafe {
                ManuallyDrop::drop(&mut cursor.inner_guard);
            }
            let current = unsafe { cursor.current.as_ref().unwrap() };
            let write_guard = current.inner.write().unwrap();

            return f(write_guard.value.as_ref());
        } else {
            return f(None);
        }
    }

    fn get(&self, key: &K) -> Option<V>
    where
        V: Clone,
    {
        let guard = pin();

        let mut cursor = self.find(key, &guard);

        if cursor.dir == Dir::Eq {
            let inner_guard = ManuallyDrop::into_inner(cursor.inner_guard);
            return inner_guard.value.clone();
        } else {
            unsafe { ManuallyDrop::drop(&mut cursor.inner_guard) };
            return None;
        }
    }

    fn remove(&self, key: &K) -> Result<V, ()> {
        let guard = pin();

        let mut cursor = self.find(key, &guard);

        let current = unsafe { cursor.current.as_ref().unwrap() };
        unsafe { ManuallyDrop::drop(&mut cursor.inner_guard) };

        if cursor.dir != Dir::Eq {
            return Err(());
        }

        // unlock read lock and lock write lock... very inefficient, need upgrade from read lock to write lock
        let mut write_guard = current.inner.write().unwrap();

        if write_guard.value.is_none() {
            return Err(());
        }

        let value = write_guard.value.take().unwrap();
        drop(write_guard);

        Cursor::repair(cursor, &guard);

        Ok(value)
    }
}

impl<K, V> Drop for RwLockAVLTree<K, V> {
    fn drop(&mut self) {
        let pin = pin();
        let mut nodes = vec![mem::replace(&mut self.root, Atomic::null())];
        while let Some(node) = nodes.pop() {
            let node = unsafe { node.into_owned() };
            let mut write_guard = node.inner.write().unwrap();

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
