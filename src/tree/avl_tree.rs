use crate::map::SequentialMap;

struct AVLTree<K: Ord + Clone, V> {
    root: Box<Node<K, V>>, // root node is dummy for simplicity
}

enum Dir {
    Left,
    Right,
}

struct Node<K, V> {
    key: K,
    value: V,
    height: usize,
    left: Option<Box<Node<K, V>>>,
    right: Option<Box<Node<K, V>>>,
}

struct Cursor<'c, K, V> {
    ancestors: Vec<(&'c mut Node<K, V>, Dir)>,
    current: &'c mut Node<K, V>,
    dir: Dir,
}

impl<K, V> SequentialMap<K, V> for AVLTree<K, V>
where
    K: Ord + Clone,
{
    fn new() -> Self {
        todo!()
    }

    fn insert(&mut self, key: &K, value: V) -> Result<(), V> {
        todo!()
    }

    fn lookup(&self, key: &K) -> Option<&V> {
        todo!()
    }

    fn remove(&mut self, key: &K) -> Result<V, ()> {
        todo!()
    }
}
