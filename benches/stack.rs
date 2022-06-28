use std::time::Duration;

use cds::stack::{EBStack, MutexStack, SpinLockStack, TreiberStack};
use criterion::{criterion_group, Criterion};
use criterion::{criterion_main, SamplingMode, Throughput};

mod util;

use util::concurrent::{bench_mixed_concurrent_stack, get_test_thread_nums};

const STACK_PER_OPS: usize = 10_000;
const STACK_PUSH_RATE: usize = 50;
const STACK_POP_RATE: usize = 50;

fn bench_mixed_mutex_stack(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!(
        "MutexStack/Ops(push: {}%, pop: {}%, per: {:+e})",
        STACK_PUSH_RATE, STACK_POP_RATE, STACK_PER_OPS
    ));
    group.measurement_time(Duration::from_secs(5));
    group.sampling_mode(SamplingMode::Flat);

    for num in get_test_thread_nums() {
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
    group.measurement_time(Duration::from_secs(5));
    group.sampling_mode(SamplingMode::Flat);

    for num in get_test_thread_nums() {
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
    group.measurement_time(Duration::from_secs(5));
    group.sampling_mode(SamplingMode::Flat);

    for num in get_test_thread_nums() {
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
    group.measurement_time(Duration::from_secs(5));
    group.sampling_mode(SamplingMode::Flat);

    for num in get_test_thread_nums() {
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
    bench_mixed_mutex_stack,
    bench_mixed_spinlock_stack,
    bench_mixed_treiber_stack,
    bench_mixed_ebstack,
);
criterion_main! {
    bench,
}
