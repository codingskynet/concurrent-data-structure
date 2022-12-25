use crossbeam_epoch::Atomic;

struct Node<K, V> {
    key: K,
    value: V,
    next: Atomic<Node<K, V>>,
}
