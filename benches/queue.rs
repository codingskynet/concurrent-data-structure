mod util;

use std::time::{Duration, Instant};

use cds::lock::{RawMutex, RawSpinLock};
use cds::queue::*;
use criterion::{black_box, criterion_group, Criterion};
use criterion::{criterion_main, SamplingMode, Throughput};
use crossbeam_queue::SegQueue;
use crossbeam_utils::thread;
use rand::{thread_rng, Rng};

use util::concurrent::{bench_mixed_concurrent_queue, get_test_thread_nums};
use util::sequential::bench_mixed_sequential_queue;

const QUEUE_PER_OPS: usize = 10_000;
const QUEUE_PUSH_RATE: usize = 50;
const QUEUE_POP_RATE: usize = 50;

fn bench_crossbeam_seg_queue(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!(
        "crossbeam_queue::SegQueue/Ops(push: {}%, pop: {}%, per: {:+e})",
        QUEUE_PUSH_RATE, QUEUE_POP_RATE, QUEUE_PER_OPS
    ));
    group.sampling_mode(SamplingMode::Flat);

    for num in get_test_thread_nums() {
        group.measurement_time(Duration::from_secs(1 * num as u64));
        group.throughput(Throughput::Elements((QUEUE_PER_OPS * num) as u64));
        group.bench_function(&format!("{} threads", num,), |b| {
            b.iter_custom(|iters| {
                let queue = SegQueue::new();

                let mut duration = Duration::ZERO;
                for _ in 0..iters {
                    let batched_time = thread::scope(|s| {
                        let mut threads = Vec::new();

                        for _ in 0..num {
                            let t = s.spawn(|_| {
                                let mut rng = thread_rng();
                                let mut duration = Duration::ZERO;

                                for _ in 0..QUEUE_PER_OPS {
                                    let op_idx = rng.gen_range(0..QUEUE_PER_OPS);

                                    if op_idx < QUEUE_PUSH_RATE * QUEUE_PER_OPS / 100 {
                                        let value: u64 = rng.gen();

                                        let start = Instant::now();
                                        let _ = black_box(queue.push(value));
                                        duration += start.elapsed();
                                    } else {
                                        let start = Instant::now();
                                        let _ = black_box(queue.pop());
                                        duration += start.elapsed();
                                    }
                                }

                                duration
                            });

                            threads.push(t);
                        }

                        threads
                            .into_iter()
                            .map(|h| h.join().unwrap())
                            .collect::<Vec<_>>()
                            .iter()
                            .sum::<Duration>()
                    })
                    .unwrap();

                    duration += batched_time
                }

                // avg thread time
                duration / (num as u32)
            });
        });
    }
}

fn bench_sequential<Q: SequentialQueue<u64>>(name: String, c: &mut Criterion) {
    let mut group = c.benchmark_group(name);
    group.measurement_time(Duration::from_secs(1));
    group.sampling_mode(SamplingMode::Flat);
    group.throughput(Throughput::Elements(QUEUE_PER_OPS as u64));
    bench_mixed_sequential_queue::<Queue<_>>(
        QUEUE_PER_OPS * QUEUE_PUSH_RATE / 100,
        QUEUE_PER_OPS * QUEUE_POP_RATE / 100,
        &mut group,
    );
}

fn bench_concurrent<Q: Sync + ConcurrentQueue<u64>>(name: String, c: &mut Criterion) {
    let mut group = c.benchmark_group(name);
    group.sampling_mode(SamplingMode::Flat);

    for num in get_test_thread_nums() {
        group.measurement_time(Duration::from_secs(1 * num as u64));
        group.throughput(Throughput::Elements((QUEUE_PER_OPS * num) as u64));
        bench_mixed_concurrent_queue::<FCQueue<_, RawSpinLock, Queue<_>>>(
            QUEUE_PER_OPS * QUEUE_PUSH_RATE / 100,
            QUEUE_PER_OPS * QUEUE_POP_RATE / 100,
            num,
            &mut group,
        );
    }
}

fn bench_mixed_queue(c: &mut Criterion) {
    bench_sequential::<Queue<_>>(
        format!(
            "Queue/Ops(push: {}%, pop: {}%, per: {:+e})",
            QUEUE_PUSH_RATE, QUEUE_POP_RATE, QUEUE_PER_OPS
        ),
        c,
    )
}

fn bench_mixed_fat_node_queue(c: &mut Criterion) {
    bench_sequential::<FatNodeQueue<_>>(
        format!(
            "FatQueueQueue/Ops(push: {}%, pop: {}%, per: {:+e})",
            QUEUE_PUSH_RATE, QUEUE_POP_RATE, QUEUE_PER_OPS
        ),
        c,
    )
}

fn bench_mixed_flat_combining_spinlock_queue(c: &mut Criterion) {
    bench_concurrent::<FCQueue<_, RawSpinLock, Queue<_>>>(
        format!(
            "FCQueue<RawSpinLock, Queue>/Ops(push: {}%, pop: {}%, per: {:+e})",
            QUEUE_PUSH_RATE, QUEUE_POP_RATE, QUEUE_PER_OPS
        ),
        c,
    );
}

fn bench_mixed_flat_combining_spinlock_fat_node_queue(c: &mut Criterion) {
    bench_concurrent::<FCQueue<_, RawSpinLock, FatNodeQueue<_>>>(
        format!(
            "FCQueue<RawSpinLock, FatNodeQueue>/Ops(push: {}%, pop: {}%, per: {:+e})",
            QUEUE_PUSH_RATE, QUEUE_POP_RATE, QUEUE_PER_OPS
        ),
        c,
    );
}

fn bench_mixed_flat_combining_mutex_queue(c: &mut Criterion) {
    bench_concurrent::<FCQueue<_, RawMutex, Queue<_>>>(
        format!(
            "FCQueue<RawMutex, Queue>/Ops(push: {}%, pop: {}%, per: {:+e})",
            QUEUE_PUSH_RATE, QUEUE_POP_RATE, QUEUE_PER_OPS
        ),
        c,
    );
}

fn bench_mixed_flat_combining_mutex_fat_node_queue(c: &mut Criterion) {
    bench_concurrent::<FCQueue<_, RawMutex, FatNodeQueue<_>>>(
        format!(
            "FCQueue<RawMutex, FatNodeQueue>/Ops(push: {}%, pop: {}%, per: {:+e})",
            QUEUE_PUSH_RATE, QUEUE_POP_RATE, QUEUE_PER_OPS
        ),
        c,
    );
}

fn bench_mixed_mutex_queue(c: &mut Criterion) {
    bench_concurrent::<MutexQueue<_>>(
        format!(
            "MutexQueue/Ops(push: {}%, pop: {}%, per: {:+e})",
            QUEUE_PUSH_RATE, QUEUE_POP_RATE, QUEUE_PER_OPS
        ),
        c,
    );
}

fn bench_mixed_two_mutex_queue(c: &mut Criterion) {
    bench_concurrent::<TwoMutexQueue<_>>(
        format!(
            "TwoMutexQueue/Ops(push: {}%, pop: {}%, per: {:+e})",
            QUEUE_PUSH_RATE, QUEUE_POP_RATE, QUEUE_PER_OPS
        ),
        c,
    );
}

fn bench_mixed_spin_lock_queue(c: &mut Criterion) {
    bench_concurrent::<SpinLockQueue<_>>(
        format!(
            "SpinLockQueue/Ops(push: {}%, pop: {}%, per: {:+e})",
            QUEUE_PUSH_RATE, QUEUE_POP_RATE, QUEUE_PER_OPS
        ),
        c,
    );
}

fn bench_mixed_two_spin_lock_queue(c: &mut Criterion) {
    bench_concurrent::<TwoSpinLockQueue<_>>(
        format!(
            "TwoSpinLockQueue/Ops(push: {}%, pop: {}%, per: {:+e})",
            QUEUE_PUSH_RATE, QUEUE_POP_RATE, QUEUE_PER_OPS
        ),
        c,
    );
}

fn bench_mixed_ms_queue(c: &mut Criterion) {
    bench_concurrent::<MSQueue<_>>(
        format!(
            "MSQueue/Ops(push: {}%, pop: {}%, per: {:+e})",
            QUEUE_PUSH_RATE, QUEUE_POP_RATE, QUEUE_PER_OPS
        ),
        c,
    );
}

criterion_group!(
    bench,
    bench_mixed_queue,
    bench_mixed_fat_node_queue,
    bench_crossbeam_seg_queue,
    bench_mixed_flat_combining_spinlock_queue,
    bench_mixed_flat_combining_spinlock_fat_node_queue,
    bench_mixed_flat_combining_mutex_queue,
    bench_mixed_flat_combining_mutex_fat_node_queue,
    bench_mixed_mutex_queue,
    bench_mixed_spin_lock_queue,
    bench_mixed_two_mutex_queue,
    bench_mixed_two_spin_lock_queue,
    bench_mixed_ms_queue
);

criterion_main! {
    bench,
}
