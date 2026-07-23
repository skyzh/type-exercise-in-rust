// Copyright 2022-2026 Alex Chi. Licensed under Apache-2.0.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use expr_common::array::{Array, ArrayBuilder, ArrayImpl, BoolArray, BoolArrayBuilder, I32Array};
use expr_common::column::ColumnViewImpl;
use expr_common::scalar::ScalarRefImpl;
use expr_template::BinaryExpression;

const ROWS: usize = 65_536;

fn input_arrays() -> (I32Array, I32Array) {
    let left = (0..ROWS)
        .map(|row| (row % 17 != 0).then_some(row as i32))
        .collect::<Vec<_>>();
    let right = (0..ROWS)
        .map(|row| (row % 19 != 0).then_some((ROWS - row) as i32))
        .collect::<Vec<_>>();
    (I32Array::from_slice(&left), I32Array::from_slice(&right))
}

fn handwritten_array(left: &I32Array, right: &I32Array) -> BoolArray {
    let mut output = BoolArrayBuilder::with_capacity(left.len());
    for (left, right) in left.iter().zip(right.iter()) {
        output.push(left.zip(right).map(|(left, right)| left <= right));
    }
    output.finish()
}

fn handwritten_constant(left: &I32Array, right: i32) -> BoolArray {
    let mut output = BoolArrayBuilder::with_capacity(left.len());
    for left in left.iter() {
        output.push(left.map(|left| left <= right));
    }
    output.finish()
}

fn handwritten_dictionary(
    indices: &[Option<usize>],
    values: &I32Array,
    right: &I32Array,
) -> BoolArray {
    let mut output = BoolArrayBuilder::with_capacity(indices.len());
    for (row, key) in indices.iter().enumerate() {
        let left = key.and_then(|key| values.get(key));
        output.push(left.zip(right.get(row)).map(|(left, right)| left <= right));
    }
    output.finish()
}

fn benchmark_expression(c: &mut Criterion) {
    let (left, right) = input_arrays();
    let left_impl: ArrayImpl = left.clone().into();
    let right_impl: ArrayImpl = right.clone().into();
    let expression = BinaryExpression::<i32, i32, bool, _>::new(|left, right| left <= right);

    let dictionary_values =
        I32Array::from_slice(&(0..256).map(|value| Some(value * 257)).collect::<Vec<_>>());
    let dictionary_values_impl: ArrayImpl = dictionary_values.clone().into();
    let dictionary_indices = (0..ROWS)
        .map(|row| (row % 17 != 0).then_some(row % 256))
        .collect::<Vec<_>>();

    let mut group = c.benchmark_group("i32_less_equal");
    group.throughput(Throughput::Elements(ROWS as u64));

    group.bench_function(BenchmarkId::new("generated", "array_array"), |b| {
        b.iter(|| {
            black_box(
                expression
                    .eval_batch(black_box(&left_impl), black_box(&right_impl))
                    .unwrap(),
            )
        })
    });
    group.bench_function(BenchmarkId::new("handwritten", "array_array"), |b| {
        b.iter(|| black_box(handwritten_array(black_box(&left), black_box(&right))))
    });

    group.bench_function(BenchmarkId::new("generated", "array_constant"), |b| {
        b.iter(|| {
            black_box(
                expression
                    .eval_views(
                        ColumnViewImpl::array(black_box(&left_impl)),
                        ColumnViewImpl::constant(ScalarRefImpl::Int32(32_768), ROWS),
                    )
                    .unwrap(),
            )
        })
    });
    group.bench_function(BenchmarkId::new("handwritten", "array_constant"), |b| {
        b.iter(|| black_box(handwritten_constant(black_box(&left), black_box(32_768))))
    });

    group.bench_function(BenchmarkId::new("generated", "dictionary_array"), |b| {
        b.iter(|| {
            let dictionary = ColumnViewImpl::dictionary(
                black_box(&dictionary_indices),
                black_box(&dictionary_values_impl),
            )
            .unwrap();
            black_box(
                expression
                    .eval_views(dictionary, ColumnViewImpl::array(black_box(&right_impl)))
                    .unwrap(),
            )
        })
    });
    group.bench_function(BenchmarkId::new("handwritten", "dictionary_array"), |b| {
        b.iter(|| {
            black_box(handwritten_dictionary(
                black_box(&dictionary_indices),
                black_box(&dictionary_values),
                black_box(&right),
            ))
        })
    });

    group.finish();
}

criterion_group!(benches, benchmark_expression);
criterion_main!(benches);
