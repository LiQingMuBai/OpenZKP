// TODO: Use u256h everywhere
#![allow(clippy::unreadable_literal)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hex_literal::*;
use primefield::{u256h, FieldElement, U256};
use starkdex::hash;

fn pedersen_hash(crit: &mut Criterion) {
    let elements = [
        u256h!("03d937c035c878245caf64531a5756109c53068da139362728feb561405371cb"),
        u256h!("0208a0a10250e382e1e4bbe2880906c2791bf6275695e02fbbc6aeff9cd8b31a"),
    ];
    crit.bench_function("Pedersen hash", move |bench| {
        bench.iter(|| black_box(hash(&elements)))
    });
}

fn criterion_benchmark(c: &mut Criterion) {
    pedersen_hash(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
