use crossbeam_epoch::Atomic;
use crossbeam_epoch::Guard;
use crossbeam_epoch::Owned;
use crossbeam_epoch::Shared;
use crossbeam_utils::sync::ShardedLock;
use crossbeam_utils::sync::ShardedLockReadGuard;
use std::fmt::Debug;
use std::mem;
use std::sync::atomic::Ordering;

use crate::map::ConcurrentMap;

pub struct RwLockAVLTree<K, V> {
    root: Atomic<Node<K, V>>,
}

#[derive(Debug)]
struct Node<K, V> {
    key: K,
    /// rwlock for shared mutable area
    inner: ShardedLock<NodeInner<K, V>>,
}

#[derive(Debug)]
struct NodeInner<K, V> {
    value: Option<V>,
    height: isize,
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

    fn get_child_mut(&mut self, dir: Dir) -> &mut Atomic<Node<K, V>> {
        match dir {
            Dir::Left => &mut self.left,
            Dir::Right => &mut self.right,
            Dir::Eq => unreachable!(),
        }
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
            inner: ShardedLock::new(NodeInner {
                value: Some(value),
                height: 1,
                left: Atomic::null(),
                right: Atomic::null(),
            }),
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
    inner_guard: ShardedLockReadGuard<'g, NodeInner<K, V>>,
    dir: Dir,
}

impl<'g, K, V> Cursor<'g, K, V>
where
    K: Debug,
{
    fn new(tree: &RwLockAVLTree<K, V>, guard: &'g Guard) -> Cursor<'g, K, V> {
        let root = tree.root.load(Ordering::Relaxed, guard);
        let inner_guard = unsafe {
            root.as_ref()
                .unwrap()
                .inner
                .read()
                .expect("Failed to load root read lock")
        };

        let cursor = Cursor {
            ancestors: vec![],
            current: root,
            inner_guard,
            dir: Dir::Right,
        };

        cursor
    }

    /// get the immutable reference of the next node by the direction using read lock
    ///
    /// This function also returns readlock guard. Therefore, caller should handle with the guard manually.
    fn next_node(&self, guard: &'g Guard) -> Shared<Node<K, V>> {
        match self.dir {
            Dir::Left => self.inner_guard.left.load(Ordering::Relaxed, guard),
            Dir::Right => self.inner_guard.right.load(Ordering::Relaxed, guard),
            Dir::Eq => panic!("The node is already arrived."),
        }
    }

    /// move the cursor to the direction using hand-over-hand locking
    ///
    /// The cursor's dir is never changed by any functions. You should change it manually like `cursor.dir = Dir::Left`.
    fn move_next(&mut self, guard: &'g Guard) {
        let next = match self.dir {
            Dir::Left => self.inner_guard.left.load(Ordering::Relaxed, guard),
            Dir::Right => self.inner_guard.right.load(Ordering::Relaxed, guard),
            Dir::Eq => panic!("The node is already arrived."),
        };

        let next_node = unsafe { next.as_ref().unwrap() };
        let next_guard = next_node
            .inner
            .read()
            .expect(&format!("Failed to load {:?} read lock", next_node.key));

        let parent = mem::replace(&mut self.current, next);
        self.ancestors.push((parent, self.dir));

        // replace with current's read guard, then unlock parent read guard by dropping after scope
        let _ = mem::replace(&mut self.inner_guard, next_guard);
    }

    /// cleanup moving to ancestor
    ///
    /// If the node does not have full childs, delete it and move child to its position.
    fn cleanup(mut self, guard: &'g Guard) {
        while let Some((parent, dir)) = self.ancestors.pop() {
            let parent_ref = unsafe { parent.as_ref().unwrap() };

            let current = self.current;
            let current_ref = unsafe { current.as_ref().unwrap() };
            let read_guard = self.inner_guard;

            // only already logically removed node can be cleaned up
            if read_guard.value.is_none() {
                let (left, right) = (
                    read_guard.left.load(Ordering::Relaxed, guard).is_null(),
                    read_guard.right.load(Ordering::Relaxed, guard).is_null(),
                );

                // if the node has one or zero node, the node can be directly removed
                if left || right {
                    drop(read_guard);

                    let parent_write_guard = parent_ref
                        .inner
                        .write()
                        .expect(&format!("Faild to load {:?} write lock", parent_ref.key));

                    // check if current's parent is even parent now
                    if parent_write_guard
                        .get_child(dir)
                        .load(Ordering::Relaxed, guard)
                        == self.current
                    {
                        let write_guard = current_ref
                            .inner
                            .write()
                            .expect(&format!("Faild to load {:?} write lock", current_ref.key));

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

                            let _ = parent_write_guard.get_child(dir).swap(
                                replace_node,
                                Ordering::Relaxed,
                                guard,
                            );

                            // drop(write_guard);
                            // drop(parent_write_guard);

                            // request deallocate removed node
                            unsafe {
                                guard.defer_destroy(current);
                            }
                        }
                    }
                }
            }

            self.current = parent;
            self.inner_guard = parent_ref
                .inner
                .read()
                .expect(&format!("Failed to load {:?} read lock", parent_ref.key));
        }
    }
}

impl<K, V> RwLockAVLTree<K, V>
where
    K: Default + Ord + Clone + Debug,
    V: Default + Debug,
{
    /// find the last state of the cursor by the key
    ///
    /// If there exists the key on the tree, the cursor's current is the node and the dir is Eq.
    /// If there does not exist the key on the tree, the cursor's current is leaf node and the dir is
    /// Left if the key is greater than the key of the node, or Right if the key is less than.
    fn find<'g>(&self, key: &K, guard: &'g Guard) -> Cursor<'g, K, V> {
        let mut cursor = Cursor::new(self, guard);

        loop {
            let next = cursor.next_node(guard);

            // TODO: consider tag for removing
            if unsafe { next.as_ref().is_none() } {
                return cursor;
            }

            cursor.move_next(guard);

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
    pub fn get_height(&self, guard: &Guard) -> usize {
        unsafe {
            self.root
                .load(Ordering::Relaxed, guard)
                .as_ref()
                .unwrap()
                .inner
                .read()
                .expect("Failed to load root read lock")
                .right
                .load(Ordering::Relaxed, guard)
                .as_ref()
                .unwrap()
                .inner
                .read()
                .expect("Failed to load root right read lock")
                .height as usize
        }
    }
}

impl<K, V> ConcurrentMap<K, V> for RwLockAVLTree<K, V>
where
    K: Ord + Clone + Default + Debug,
    V: Clone + Default + Debug,
{
    fn new() -> Self {
        RwLockAVLTree {
            root: Atomic::new(Node::default()),
        }
    }

    fn insert(&self, key: &K, value: V, guard: &Guard) -> Result<(), V> {
        let node = Node::new(key.clone(), value);

        // TODO: it can be optimized by re-search nearby ancestors
        loop {
            let cursor = self.find(key, guard);

            if cursor.dir == Dir::Eq && cursor.inner_guard.value.is_some() {
                let node_inner = node
                    .inner
                    .into_inner()
                    .expect("Failed to get data from node");
                return Err(node_inner.value.unwrap());
            }

            let current = unsafe { cursor.current.as_ref().unwrap() };

            // unlock read lock and lock write lock... very inefficient, need upgrade from read lock to write lock
            let read_guard = cursor.inner_guard;
            drop(read_guard);
            let mut write_guard = current
                .inner
                .write()
                .expect(&format!("Failed to load {:?} write lock", current.key));

            unsafe {
                match cursor.dir {
                    Dir::Left => {
                        if write_guard
                            .left
                            .load(Ordering::Relaxed, guard)
                            .as_ref()
                            .is_some()
                        {
                            continue; // some thread already writed. Retry
                        }

                        write_guard.left.store(Owned::new(node), Ordering::Relaxed)
                    }
                    Dir::Right => {
                        if write_guard
                            .right
                            .load(Ordering::Relaxed, guard)
                            .as_ref()
                            .is_some()
                        {
                            continue; // some thread already writed. Retry
                        }

                        write_guard.right.store(Owned::new(node), Ordering::Relaxed)
                    }
                    Dir::Eq => {
                        let value = node
                            .inner
                            .into_inner()
                            .expect("Failed to get data from node")
                            .value
                            .unwrap();

                        if write_guard.value.is_some() {
                            return Err(value);
                        }

                        write_guard.value = Some(value);
                    }
                }
            }

            // unsafe {
            //     cursor.current.as_mut().renew_height();
            // }
            // cursor.rebalance();

            return Ok(());
        }
    }

    fn lookup(&self, key: &K, guard: &Guard) -> Option<V> {
        let cursor = self.find(key, guard);

        if cursor.dir == Dir::Eq {
            return cursor.inner_guard.value.clone();
        } else {
            return None;
        }
    }

    fn remove(&self, key: &K, guard: &Guard) -> Result<V, ()> {
        let mut cursor = self.find(key, guard);

        if cursor.dir != Dir::Eq {
            return Err(());
        }

        let current = unsafe { cursor.current.as_ref().unwrap() };

        // unlock read lock and lock write lock... very inefficient, need upgrade from read lock to write lock
        let read_guard = cursor.inner_guard;
        drop(read_guard);
        let mut write_guard = current
            .inner
            .write()
            .expect(&format!("Failed to load {:?} write lock", current.key));

        if write_guard.value.is_none() {
            return Err(());
        }

        let value = write_guard.value.take().unwrap();
        drop(write_guard);

        cursor.inner_guard = current
            .inner
            .read()
            .expect(&format!("Failed to load {:?} read lock", current.key));
        cursor.cleanup(guard);

        Ok(value)
    }
}
