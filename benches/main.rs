use criterion::criterion_main;

mod benchmark;
pub mod util;

criterion_main! {
    benchmark::bench,
}
