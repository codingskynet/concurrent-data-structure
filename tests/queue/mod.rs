mod fclock;
mod lockfree;
mod mutex;
mod spinlock;

use cds::queue::{FatNodeQueue, Queue};

use crate::util::queue::*;

#[test]
fn test_simple_queue() {
    test_simple_sequential_queue::<Queue<_>>();
}

#[test]
fn test_deep_queue() {
    test_deep_sequential_queue::<Queue<_>>();
}

#[test]
fn test_fat_node_queue() {
    test_simple_sequential_queue::<FatNodeQueue<_>>();
}

#[test]
fn test_deep_fat_node_queue() {
    test_deep_sequential_queue::<FatNodeQueue<_>>();
}
