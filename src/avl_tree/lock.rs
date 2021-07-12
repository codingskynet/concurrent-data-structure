use crossbeam_epoch::Atomic;
use crossbeam_epoch::Guard;
use crossbeam_utils::sync::ShardedLock;

use crate::map::ConcurrentMap;

pub struct LockAVLTree<K, V> {
    root: Atomic<Node<K, V>>,
}

struct Node<K, V> {
    key: K,
    inner: ShardedLock<NodeInner<K, V>>,
}

struct NodeInner<K, V> {
    value: Option<V>,
    left: Atomic<Node<K, V>>,
    right: Atomic<Node<K, V>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Dir {
    Left,
    Eq,
    Right,
}

struct Cursor<'g, K, V> {
    // ancestors: Vec<(NonNull<Node<K, V>>, Dir)>, // how to deal rwlock guard?
    // current: NonNull<Node<K, V>>, // how to deal rwlock guard?
    dir: Dir,
}

impl<K, V> ConcurrentMap<K, V> for LockAVLTree<K, V>
where
    K: Ord + Clone,
{
    fn new() -> Self {
        todo!()
    }

    fn insert(&self, key: &K, value: V, guard: &Guard) -> Result<(), V> {
        todo!()
    }

    fn lookup(&self, key: &K, guard: &Guard) -> Option<&V> {
        todo!()
    }

    fn remove(&self, key: &K, guard: &Guard) -> Result<V, ()> {
        todo!()
    }
}
