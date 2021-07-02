pub trait SequentialMap<K: Eq + Copy, V> {
    fn insert(&mut self, key: &K, value: V) -> Result<(), V>;
    fn lookup(&self, key: &K) -> Option<&V>;
    fn delete(&mut self, key: &K) -> Result<V, ()>;
}
