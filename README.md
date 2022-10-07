# Concurrent Data Structure for Rust

## Goal & Status
Implement sequential, lock-based and lock-free concurrent data structures below:

|            | Stack | Queue | Linked List | AVL Tree | B-Tree |
|------------|-------|-------|-------------|----------|--------|
| Sequential | Done  | Done  |    Done     |   Done   |  Done  |
| Lock-based | Done  | Done  |             |   Done   |        |
| Lock-free  | Done  | Done  |             |          |        |

## Benchmark
You can run bench like this:
```bash
cargo criterion --bench {bench_name} --no-default-features # default feature has accumulating stats on available structure.
```

Available Benches:
- stack
- queue
- avltree
- btrees

## Detail
### Lock
- Common SpinLock and SeqLock
- [Flat Combining Lock](https://people.csail.mit.edu/shanir/publications/Flat%20Combining%20SPAA%2010.pdf)

### Stack
- Lock Stack(based on std::sync::Mutex and spin lock)
- [Treiber's Stack](https://dominoweb.draco.res.ibm.com/58319a2ed2b1078985257003004617ef.html)
- [Elimination-Backoff Stack](https://people.csail.mit.edu/shanir/publications/Lock_Free.pdf)

### Queue
- Lock Queue(based on std::sync::Mutex and spin lock)
- [Two Lock Queue](https://www.cs.rochester.edu/~scott/papers/1996_PODC_queues.pdf)
- [Michael-Scott queue](https://www.cs.rochester.edu/~scott/papers/1996_PODC_queues.pdf)

### Linked List
TODO: implement Harris linked list

### AVL Tree
- concurrent AVL tree with RwLock(crossbeam_utils::sync::ShardedLock), SeqLock

### B-Tree
TODO: ?

## Reference
- https://github.com/kaist-cp/cs431/
- The Art of Multiprocessor Programming
- https://stanford-ppl.github.io/website/papers/ppopp207-bronson.pdf
- https://www.cs.tau.ac.il/~shanir/concurrent-data-structures.pdf
- http://www.vldb.org/pvldb/vol4/p795-sewall.pdf
- https://www.cs.umanitoba.ca/~hacamero/Research/RBTreesKim.pdf
- http://www.vldb.org/pvldb/vol11/p553-arulraj.pdf
- https://www.cs.cmu.edu/~yihans/papers/tutorial.pdf
- https://dominoweb.draco.res.ibm.com/58319a2ed2b1078985257003004617ef.html
- https://people.csail.mit.edu/shanir/publications/Lock_Free.pdf
- https://www.cs.rochester.edu/~scott/papers/1996_PODC_queues.pdf
