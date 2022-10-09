mod util;

use std::time::{Duration, Instant};

use cds::stack::{EBStack, MutexStack, SpinLockStack, Stack, TreiberStack};
use criterion::{black_box, criterion_group, Criterion};
use criterion::{criterion_main, SamplingMode, Throughput};
use rand::{thread_rng, Rng};

use util::concurrent::{bench_mixed_concurrent_stack, get_test_thread_nums};

const STACK_PER_OPS: usize = 10_000;
const STACK_PUSH_RATE: usize = 50;
const STACK_POP_RATE: usize = 50;

fn bench_mixed_stack(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!(
        "Stack/Ops(push: {}%, pop: {}%, per: {:+e})",
        STACK_PUSH_RATE, STACK_POP_RATE, STACK_PER_OPS
    ));
    group.measurement_time(Duration::from_secs(1));
    group.sampling_mode(SamplingMode::Flat);
    group.throughput(Throughput::Elements(STACK_PER_OPS as u64));

    group.bench_function("sequential", |b| {
        b.iter_custom(|iters| {
            let mut stack = Stack::new();

            let mut duration = Duration::ZERO;

            for _ in 0..iters {
                let mut rng = thread_rng();

                let op_idx = rng.gen_range(0..STACK_PER_OPS);

                if op_idx < STACK_PER_OPS {
                    let value: u64 = rng.gen();

                    let start = Instant::now();
                    let _ = black_box(stack.push(value));
                    duration += start.elapsed();
                } else {
                    let start = Instant::now();
                    let _ = black_box(stack.pop());
                    duration += start.elapsed();
                }
            }

            duration
        });
    });
}

fn bench_mixed_mutex_stack(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!(
        "MutexStack/Ops(push: {}%, pop: {}%, per: {:+e})",
        STACK_PUSH_RATE, STACK_POP_RATE, STACK_PER_OPS
    ));
    group.sampling_mode(SamplingMode::Flat);

    for num in get_test_thread_nums() {
        group.measurement_time(Duration::from_secs(1 * num as u64));
        group.throughput(Throughput::Elements((STACK_PER_OPS * num) as u64));
        bench_mixed_concurrent_stack::<MutexStack<_>>(
            STACK_PER_OPS * STACK_PUSH_RATE / 100,
            STACK_PER_OPS * STACK_POP_RATE / 100,
            num,
            &mut group,
        );
    }
}

fn bench_mixed_spinlock_stack(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!(
        "SpinLockStack/Ops(push: {}%, pop: {}%, per: {:+e})",
        STACK_PUSH_RATE, STACK_POP_RATE, STACK_PER_OPS
    ));
    group.sampling_mode(SamplingMode::Flat);

    for num in get_test_thread_nums() {
        group.measurement_time(Duration::from_secs(1 * num as u64));
        group.throughput(Throughput::Elements((STACK_PER_OPS * num) as u64));
        bench_mixed_concurrent_stack::<SpinLockStack<_>>(
            STACK_PER_OPS * STACK_PUSH_RATE / 100,
            STACK_PER_OPS * STACK_POP_RATE / 100,
            num,
            &mut group,
        );
    }
}

fn bench_mixed_treiber_stack(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!(
        "TreiberStack/Ops(push: {}%, pop: {}%, per: {:+e})",
        STACK_PUSH_RATE, STACK_POP_RATE, STACK_PER_OPS
    ));
    group.sampling_mode(SamplingMode::Flat);

    for num in get_test_thread_nums() {
        group.measurement_time(Duration::from_secs(1 * num as u64));
        group.throughput(Throughput::Elements((STACK_PER_OPS * num) as u64));
        bench_mixed_concurrent_stack::<TreiberStack<_>>(
            STACK_PER_OPS * STACK_PUSH_RATE / 100,
            STACK_PER_OPS * STACK_POP_RATE / 100,
            num,
            &mut group,
        );
    }
}

fn bench_mixed_ebstack(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!(
        "EBStack/Ops(push: {}%, pop: {}%, per: {:+e})",
        STACK_PUSH_RATE, STACK_POP_RATE, STACK_PER_OPS
    ));
    group.sampling_mode(SamplingMode::Flat);

    for num in get_test_thread_nums() {
        group.measurement_time(Duration::from_secs(1 * num as u64));
        group.throughput(Throughput::Elements((STACK_PER_OPS * num) as u64));
        bench_mixed_concurrent_stack::<EBStack<_>>(
            STACK_PER_OPS * STACK_PUSH_RATE / 100,
            STACK_PER_OPS * STACK_POP_RATE / 100,
            num,
            &mut group,
        );
    }
}

criterion_group!(
    bench,
    bench_mixed_stack,
    bench_mixed_mutex_stack,
    bench_mixed_spinlock_stack,
    bench_mixed_treiber_stack,
    bench_mixed_ebstack,
);
criterion_main! {
    bench,
}
