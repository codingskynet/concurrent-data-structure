use crossbeam_epoch::Guard;

pub trait SequentialMap<K: Ord + Clone, V> {
    fn new() -> Self;

    /// Insert (key, vaule) into the map.
    /// 
    /// If success, return Ok(()).
    /// If fail, return Err(value) that you tried to insert.
    fn insert(&mut self, key: &K, value: V) -> Result<(), V>;

    /// Lookup (key, value) from the map with the key.
    ///
    /// If success, return the reference of the value.
    /// If fail, return None.
    fn lookup(&self, key: &K) -> Option<&V>;

    /// Remove (key, value) from the map with the key.
    ///
    /// If success, return Ok(value) which is inserted before.
    /// If fail, return Err(()).
    fn remove(&mut self, key: &K) -> Result<V, ()>;
}

pub trait ConcurrentMap<K: Ord + Clone, V> {
    fn new() -> Self;

    /// Insert (key, vaule) into the map.
    /// 
    /// If success, return Ok(()).
    /// If fail, return Err(value) that you tried to insert.
    fn insert(&self, key: &K, value: V, guard: &Guard) -> Result<(), V>;

    /// Lookup (key, value) from the map with the key.
    ///
    /// If success, return the reference of the value.
    /// If fail, return None.
    fn lookup(&self, key: &K, guard: &Guard) -> Option<V>;

    /// Remove (key, value) from the map with the key.
    ///
    /// If success, return Ok(value) which is inserted before.
    /// If fail, return Err(()).
    fn remove(&self, key: &K, guard: &Guard) -> Result<V, ()>;
}
