use std::collections::BTreeMap;
use rand::Rng;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kvs_rs::btree;

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("my btree: insert seq 500K", |b| b.iter(|| btree_insert_seq(black_box(500_000))));
    c.bench_function("reference btree: insert seq 500K", |b| b.iter(|| reference_btreemap_insert_seq(black_box(500_000))));
    c.bench_function("my btree: insert rand 500K", |b| b.iter(|| btree_insert_rand(black_box(500_000))));
    c.bench_function("reference btree: insert rand 500K", |b| b.iter(|| reference_btreemap_insert_rand(black_box(500_000))));
    c.bench_function("my btree: get rand 500K", |b| b.iter(|| btree_get_rand(black_box(500_000))));
    c.bench_function("reference btree: get rand 500K", |b| b.iter(|| reference_btreemap_get_rand(black_box(500_000))));
}

fn btree_insert_seq(n: usize) {
    let mut t = btree::BTree::new();
    for i in 0..n {
        t.insert([i as u128; 1], 0);
    }
}

fn reference_btreemap_insert_seq(n: usize) {
    let mut t = BTreeMap::<[u128; 1], u8>::new();
    for i in 0..n {
        t.insert([i as u128; 1], 0);
    }
}

fn btree_insert_rand(n: usize) {
    let mut rng = rand::thread_rng();
    let mut t = btree::BTree::new();
    for _ in 0..n {
        t.insert([rng.gen(); 1], 0);
    }
}

fn reference_btreemap_insert_rand(n: usize) {
    let mut rng = rand::thread_rng();
    let mut t = BTreeMap::<[u128; 1], u8>::new();
    for _ in 0..n {
        t.insert([rng.gen(); 1], 0);
    }
}

fn btree_get_rand(n: usize) {
    let mut rng = rand::thread_rng();
    let mut t = btree::BTree::new();
    for _ in 0..n {
        t.insert([rng.gen(); 1], 0);
    }
    for _ in 0..n {
        t.get(&[rng.gen(); 1]);
    }
}

fn reference_btreemap_get_rand(n: usize) {
    let mut rng = rand::thread_rng();
    let mut t = BTreeMap::<[u128; 1], u8>::new();
    for _ in 0..n {
        t.insert([rng.gen(); 1], 0);
    }
    for _ in 0..n {
        t.get(&[rng.gen(); 1]);
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);