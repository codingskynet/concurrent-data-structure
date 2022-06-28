use std::time::{Duration, Instant};

use cds::queue::{MSQueue, MutexQueue, Queue, SpinLockQueue, TwoMutexQueue, TwoSpinLockQueue};
use criterion::{black_box, criterion_group, Criterion};
use criterion::{criterion_main, SamplingMode, Throughput};

mod util;

use rand::{thread_rng, Rng};
use util::concurrent::{bench_mixed_concurrent_queue, get_test_thread_nums};

const QUEUE_PER_OPS: usize = 10_000;
const QUEUE_PUSH_RATE: usize = 50;
const QUEUE_POP_RATE: usize = 50;

fn bench_mixed_queue(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!(
        "Queue/Ops(push: {}%, pop: {}%, per: {:+e})",
        QUEUE_PUSH_RATE, QUEUE_POP_RATE, QUEUE_PER_OPS
    ));
    group.measurement_time(Duration::from_secs(5));
    group.sampling_mode(SamplingMode::Flat);
    group.throughput(Throughput::Elements(QUEUE_PER_OPS as u64));

    group.bench_function("sequential", |b| {
        b.iter_custom(|iters| {
            let mut queue = Queue::new();

            let mut duration = Duration::ZERO;

            for _ in 0..iters {
                let mut rng = thread_rng();

                let op_idx = rng.gen_range(0..QUEUE_PER_OPS);

                if op_idx < QUEUE_PER_OPS {
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
    });
}

fn bench_mixed_mutex_queue(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!(
        "MutexQueue/Ops(push: {}%, pop: {}%, per: {:+e})",
        QUEUE_PUSH_RATE, QUEUE_POP_RATE, QUEUE_PER_OPS
    ));
    group.measurement_time(Duration::from_secs(5));
    group.sampling_mode(SamplingMode::Flat);

    for num in get_test_thread_nums() {
        group.throughput(Throughput::Elements((QUEUE_PER_OPS * num) as u64));
        bench_mixed_concurrent_queue::<MutexQueue<_>>(
            QUEUE_PER_OPS * QUEUE_PUSH_RATE / 100,
            QUEUE_PER_OPS * QUEUE_POP_RATE / 100,
            num,
            &mut group,
        );
    }
}

fn bench_mixed_two_mutex_queue(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!(
        "TwoMutexQueue/Ops(push: {}%, pop: {}%, per: {:+e})",
        QUEUE_PUSH_RATE, QUEUE_POP_RATE, QUEUE_PER_OPS
    ));
    group.measurement_time(Duration::from_secs(5));
    group.sampling_mode(SamplingMode::Flat);

    for num in get_test_thread_nums() {
        group.throughput(Throughput::Elements((QUEUE_PER_OPS * num) as u64));
        bench_mixed_concurrent_queue::<TwoMutexQueue<_>>(
            QUEUE_PER_OPS * QUEUE_PUSH_RATE / 100,
            QUEUE_PER_OPS * QUEUE_POP_RATE / 100,
            num,
            &mut group,
        );
    }
}

fn bench_mixed_spin_lock_queue(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!(
        "SpinLockQueue/Ops(push: {}%, pop: {}%, per: {:+e})",
        QUEUE_PUSH_RATE, QUEUE_POP_RATE, QUEUE_PER_OPS
    ));
    group.measurement_time(Duration::from_secs(5));
    group.sampling_mode(SamplingMode::Flat);

    for num in get_test_thread_nums() {
        group.throughput(Throughput::Elements((QUEUE_PER_OPS * num) as u64));
        bench_mixed_concurrent_queue::<SpinLockQueue<_>>(
            QUEUE_PER_OPS * QUEUE_PUSH_RATE / 100,
            QUEUE_PER_OPS * QUEUE_POP_RATE / 100,
            num,
            &mut group,
        );
    }
}

fn bench_mixed_two_spin_lock_queue(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!(
        "TwoSpinLockQueue/Ops(push: {}%, pop: {}%, per: {:+e})",
        QUEUE_PUSH_RATE, QUEUE_POP_RATE, QUEUE_PER_OPS
    ));
    group.measurement_time(Duration::from_secs(5));
    group.sampling_mode(SamplingMode::Flat);

    for num in get_test_thread_nums() {
        group.throughput(Throughput::Elements((QUEUE_PER_OPS * num) as u64));
        bench_mixed_concurrent_queue::<TwoSpinLockQueue<_>>(
            QUEUE_PER_OPS * QUEUE_PUSH_RATE / 100,
            QUEUE_PER_OPS * QUEUE_POP_RATE / 100,
            num,
            &mut group,
        );
    }
}

fn bench_mixed_ms_queue(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!(
        "MsQueue/Ops(push: {}%, pop: {}%, per: {:+e})",
        QUEUE_PUSH_RATE, QUEUE_POP_RATE, QUEUE_PER_OPS
    ));
    group.measurement_time(Duration::from_secs(5));
    group.sampling_mode(SamplingMode::Flat);

    for num in get_test_thread_nums() {
        group.throughput(Throughput::Elements((QUEUE_PER_OPS * num) as u64));
        bench_mixed_concurrent_queue::<MSQueue<_>>(
            QUEUE_PER_OPS * QUEUE_PUSH_RATE / 100,
            QUEUE_PER_OPS * QUEUE_POP_RATE / 100,
            num,
            &mut group,
        );
    }
}

criterion_group!(
    bench,
    bench_mixed_queue,
    bench_mixed_mutex_queue,
    bench_mixed_spin_lock_queue,
    bench_mixed_two_mutex_queue,
    bench_mixed_two_spin_lock_queue,
    bench_mixed_ms_queue
);
criterion_main! {
    bench,
}
