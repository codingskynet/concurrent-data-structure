# Concurrent Data Structure for Rust

## Goal & Status
Implement sequential, lock-based and lock-free concurrent data structures below:

|            | Stack | Queue | Linked List | AVL Tree | Red-Black Tree |
|------------|-------|-------|-------------|----------|----------------|
| Sequential | Done  | Done  |    Done     |   Done   |                |
| Lock-based |       |       |             |   Done   |                |
| Lock-free  |       |       |             |          |                |

## Detail
### Stack
- Implement [Treiber Stack](https://dominoweb.draco.res.ibm.com/58319a2ed2b1078985257003004617ef.html)
- TODO: implement Treiber stack, elimination backoff stack

### Queue
TODO: implement Michael-Scott queue

### Linked List
TODO: implement Harris linked list

### AVL Tree
- Implement simple concurrent AVL Tree with RwLock(crossbeam_utils::sync::ShardedLock), SeqLock

### Red-Black Tree
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
