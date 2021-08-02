use crossbeam_epoch::pin;
use crossbeam_epoch::Atomic;
use crossbeam_epoch::Guard;
use crossbeam_epoch::Owned;
use crossbeam_epoch::Shared;
use crossbeam_utils::sync::ShardedLock;
use crossbeam_utils::sync::ShardedLockReadGuard;
use std::cmp::max;
use std::fmt::Debug;
use std::mem;
use std::mem::ManuallyDrop;
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

    fn renew_height(&mut self, guard: &Guard) {
        let left = self.left.load(Ordering::Relaxed, guard);
        let right = self.right.load(Ordering::Relaxed, guard);

        let left_guard = if !left.is_null() {
            unsafe { Some(left.as_ref().unwrap().inner.read().unwrap()) }
        } else {
            None
        };

        let right_guard = if !right.is_null() {
            unsafe { Some(right.as_ref().unwrap().inner.read().unwrap())}
        } else {
            None
        };

        let left_height = if let Some(read_guard) = &left_guard {
            read_guard.height
        } else {
            0
        };

        let right_height = if let Some(read_guard) = &right_guard {
            read_guard.height
        } else {
            0
        };

        self.height = max(left_height, right_height) + 1;
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
            inner: ShardedLock::new(NodeInner {
                value: Some(value),
                height: 1,
                left: Atomic::null(),
                right: Atomic::null(),
            }),
        }
    }

    /// cleanup moving to ancestor
    ///
    /// If the node does not have full childs, delete it and move child to its position.
    fn try_cleanup(
        current: Shared<Node<K, V>>,
        parent: Shared<Node<K, V>>,
        dir: Dir,
        guard: &Guard,
    ) {
        let parent_ref = unsafe { parent.as_ref().unwrap() };

        let current_ref = unsafe { current.as_ref().unwrap() };
        let read_guard = current_ref
            .inner
            .read()
            .expect(&format!("Faild to load {:?} read lock", current_ref.key));

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
                    == current
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
                    }
                }
            }
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

impl<'g, K, V> Cursor<'g, K, V>
where
    K: Debug,
    V: Debug,
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
            inner_guard: ManuallyDrop::new(inner_guard),
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

        // replace with current's read guard, then unlock parent read guard by dropping
        let mut parent_guard = mem::replace(&mut self.inner_guard, ManuallyDrop::new(next_guard));

        unsafe {
            ManuallyDrop::drop(&mut parent_guard);
        }
    }

    /// try to cleanup and rebalance the node
    /// TODO: manage repair operation by unique on current waiting list
    fn repair(mut cursor: Cursor<'g, K, V>, guard: &'g Guard) {
        while let Some((parent, dir)) = cursor.ancestors.pop() {
            Node::try_cleanup(cursor.current, parent, dir, guard);

            // TODO: Node::try_rebalance(current, (parent, dir), (grand_parent, dir), guard)

            let current_ref = unsafe { cursor.current.as_ref().unwrap() };
            current_ref.inner.write().unwrap().renew_height(guard);

            cursor.current = parent;
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

    /// print tree structure
    pub fn print(&self, guard: &Guard) {
        fn print<K: Debug, V: Debug>(node: Shared<Node<K, V>>, guard: &Guard) -> String {
            if node.is_null() {
                return "null".to_string();
            }

            let node = unsafe { node.as_ref().unwrap() };
            let node_inner = node.inner.read().unwrap();

            format!(
                "{{key: {:?},  value: {:?}, left: {}, right: {}}}",
                node.key,
                node_inner.value,
                print(node_inner.left.load(Ordering::Relaxed, guard), guard),
                print(node_inner.right.load(Ordering::Relaxed, guard), guard)
            )
        }

        println!("{}", print(self.root.load(Ordering::Relaxed, guard), guard));
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
            let mut cursor = self.find(key, guard);

            if cursor.dir == Dir::Eq && cursor.inner_guard.value.is_some() {
                unsafe {
                    ManuallyDrop::drop(&mut cursor.inner_guard);
                }

                let node_inner = node
                    .inner
                    .into_inner()
                    .expect("Failed to get data from node");
                return Err(node_inner.value.unwrap());
            }

            let current = unsafe { cursor.current.as_ref().unwrap() };

            // unlock read lock and lock write lock... very inefficient, need upgrade from read lock to write lock
            unsafe {
                ManuallyDrop::drop(&mut cursor.inner_guard);
            }

            // check if the current is alive now by checking parent node. If disconnected, retry
            let parent_read_guard = if let Some((parent, dir)) = cursor.ancestors.last() {
                let parent = unsafe { parent.as_ref().unwrap() };
                Some((
                    parent
                        .inner
                        .read()
                        .expect(&format!("Failed to load {:?} read lock", parent.key)),
                    dir,
                ))
            } else {
                None
            };

            if let Some((parent_read_guard, dir)) = &parent_read_guard {
                if parent_read_guard
                    .get_child(**dir)
                    .load(Ordering::Relaxed, guard)
                    != cursor.current
                {
                    // Before inserting, the current is already disconnected.
                    continue;
                }
            }

            let mut write_guard = current
                .inner
                .write()
                .expect(&format!("Failed to load {:?} write lock", current.key));

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

            drop(write_guard);
            drop(parent_read_guard);

            Cursor::repair(cursor, guard);

            return Ok(());
        }
    }

    fn lookup(&self, key: &K, guard: &Guard) -> Option<V> {
        let mut cursor = self.find(key, guard);

        if cursor.dir == Dir::Eq {
            let inner_guard = ManuallyDrop::into_inner(cursor.inner_guard);
            return inner_guard.value.clone();
        } else {
            unsafe { ManuallyDrop::drop(&mut cursor.inner_guard) };
            return None;
        }
    }

    fn remove(&self, key: &K, guard: &Guard) -> Result<V, ()> {
        let mut cursor = self.find(key, guard);

        let current = unsafe { cursor.current.as_ref().unwrap() };
        unsafe { ManuallyDrop::drop(&mut cursor.inner_guard) };

        if cursor.dir != Dir::Eq {
            return Err(());
        }

        // unlock read lock and lock write lock... very inefficient, need upgrade from read lock to write lock
        let mut write_guard = current
            .inner
            .write()
            .expect(&format!("Failed to load {:?} write lock", current.key));

        if write_guard.value.is_none() {
            return Err(());
        }

        let value = write_guard.value.take().unwrap();
        drop(write_guard);

        Cursor::repair(cursor, guard);

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
