# Concurrent Data Structure for Rust

## Goal & Status
Implement sequential, lock-based and lock-free concurrent data structures below:

|            | Stack | Queue | Linked List | AVL Tree | HashTable |
|------------|-------|-------|-------------|----------|-----------|
| Sequential | Done  | Done  |    Done     |   Done   |           |
| Lock-based | Done  | Done  |             |   Done   |           |
| Lock-free  | Done  | Done  |             |          |           |

## Benchmark
You can run bench like this:
```bash
cargo install criterion
# default feature has accumulating stats on available structure.
cargo criterion --bench {bench_name} --no-default-features
```

Available Benches:
- stack
- queue
- avltree
- btrees

## Profile

### Use CDS stats
Several cds has its own statistics. Use it by printing on test.

### Flamegraph
```bash
cargo install flamegraph
sudo cargo flamegraph --no-default-features --test tests -- {test_name}
```

## Detail
### Lock
- common spin lock and sequece lock(SeqLock)
- flat combining lock

### Stack
- lock stack(based on std::sync::Mutex and spin lock)
- Treiber's Stack
- Elimination-Backoff Stack

### Queue
- lock queue(based on std::sync::Mutex and spin lock)
- two lock queue
- FCQueue(use flat combining lock)
- Michael-Scott queue

### Linked List
- TODO: implement Harris linked list

### AVL Tree
- SeqLockAVLTree, RwLockAVLTree(use crossbeam_utils::sync::ShardedLock)

### HashTable
- TODO: ?

## Reference
### General
- The Art of Multiprocessor Programming
- https://github.com/kaist-cp/cs431
- https://github.com/khizmax/libcds
- https://www.cs.cmu.edu/~yihans/papers/tutorial.pdf

### Lock
- flat combining lock: https://people.csail.mit.edu/shanir/publications/Flat%20Combining%20SPAA%2010.pdf

### Stack
- Treiber's Stack: https://dominoweb.draco.res.ibm.com/58319a2ed2b1078985257003004617ef.html
- Elimination-Backoff Stack: https://people.csail.mit.edu/shanir/publications/Lock_Free.pdf

### Queue
- two lock queue, Michael-Scott Queue: https://www.cs.rochester.edu/~scott/papers/1996_PODC_queues.pdf

### Binary Search Tree
- AVL Tree: https://stanford-ppl.github.io/website/papers/ppopp207-bronson.pdf
- B+ Tree: http://www.vldb.org/pvldb/vol4/p795-sewall.pdf
- Red-Black Tree: https://www.cs.umanitoba.ca/~hacamero/Research/RBTreesKim.pdf
- BzTree(B Tree): http://www.vldb.org/pvldb/vol11/p553-arulraj.pdf
