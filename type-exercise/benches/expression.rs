use std::hint::black_box;
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
use type_exercise::{
    Array, ArrayBuilder, ArrayImpl, BinaryExpression, ColumnView, ColumnViewImpl, Expression,
    I32Add, I32Array, PhysicalType, PrimitiveBinaryExpression, ScalarRefImpl,
};

const ROWS: usize = 65_536;

#[derive(Clone, Copy)]
enum HandwrittenColumn<'a> {
    DenseArray(&'a [i32]),
    Constant { value: i32, len: usize },
    General,
}

fn handwritten_general_add(inputs: &[ColumnViewImpl<'_>]) -> ArrayImpl {
    assert_eq!(inputs.len(), 2);
    let left = ColumnView::<i32>::try_from(inputs[0].clone()).unwrap();
    let right = ColumnView::<i32>::try_from(inputs[1].clone()).unwrap();
    assert_eq!(left.len(), right.len());

    let mut output = <I32Array as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        output.push(match (left.get(row), right.get(row)) {
            (Some(left), Some(right)) => Some(left.wrapping_add(right)),
            _ => None,
        });
    }
    output.finish().into()
}

fn general_add_batch(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    Ok(handwritten_general_add(inputs))
}

fn handwritten_add(
    inputs: &[ColumnViewImpl<'_>],
    handwritten: [HandwrittenColumn<'_>; 2],
) -> ArrayImpl {
    let values = match handwritten {
        [
            HandwrittenColumn::DenseArray(left),
            HandwrittenColumn::DenseArray(right),
        ] => left
            .iter()
            .copied()
            .zip(right.iter().copied())
            .map(|(left, right)| left.wrapping_add(right))
            .collect(),
        [
            HandwrittenColumn::DenseArray(left),
            HandwrittenColumn::Constant { value: right, .. },
        ] => left
            .iter()
            .copied()
            .map(|left| left.wrapping_add(right))
            .collect(),
        [
            HandwrittenColumn::Constant { value: left, .. },
            HandwrittenColumn::DenseArray(right),
        ] => right
            .iter()
            .copied()
            .map(|right| left.wrapping_add(right))
            .collect(),
        [
            HandwrittenColumn::Constant { value: left, len },
            HandwrittenColumn::Constant { value: right, .. },
        ] => (0..len).map(|_| left.wrapping_add(right)).collect(),
        _ => return handwritten_general_add(inputs),
    };
    I32Array::from_values(values).into()
}

fn benchmark_case(
    criterion: &mut Criterion,
    name: &str,
    inputs: [ColumnViewImpl<'_>; 2],
    handwritten: [HandwrittenColumn<'_>; 2],
) {
    let general = BinaryExpression::new(
        "i32_add_general",
        [PhysicalType::Int32, PhysicalType::Int32],
        PhysicalType::Int32,
        general_add_batch,
    );
    let specialized = PrimitiveBinaryExpression::new("i32_add", I32Add);
    let expected = handwritten_add(&inputs, handwritten);
    assert_eq!(general.evaluate(&inputs).unwrap(), expected);
    assert_eq!(specialized.evaluate(&inputs).unwrap(), expected);

    let mut group = criterion.benchmark_group(name);
    group.bench_function("general", |bencher| {
        bencher.iter(|| general.evaluate(black_box(&inputs)).unwrap())
    });
    group.bench_function("specialized", |bencher| {
        bencher.iter(|| specialized.evaluate(black_box(&inputs)).unwrap())
    });
    group.bench_function("handwritten", |bencher| {
        bencher.iter(|| handwritten_add(black_box(&inputs), handwritten))
    });
    group.finish();
}

fn benchmark_expressions(criterion: &mut Criterion) {
    let dense_left: ArrayImpl =
        I32Array::from_values((0..ROWS).map(|row| row as i32).collect()).into();
    let dense_right: ArrayImpl =
        I32Array::from_values((0..ROWS).map(|row| (row as i32).wrapping_mul(3)).collect()).into();
    let dense_left_values = <&I32Array>::try_from(&dense_left).unwrap().values();
    let dense_right_values = <&I32Array>::try_from(&dense_right).unwrap().values();

    let dense_left_view = ColumnViewImpl::try_non_null_array(&dense_left).unwrap();
    let dense_right_view = ColumnViewImpl::try_non_null_array(&dense_right).unwrap();

    benchmark_case(
        criterion,
        "dense/array-array",
        [dense_left_view.clone(), dense_right_view.clone()],
        [
            HandwrittenColumn::DenseArray(dense_left_values),
            HandwrittenColumn::DenseArray(dense_right_values),
        ],
    );
    benchmark_case(
        criterion,
        "dense/array-constant",
        [
            dense_left_view,
            ColumnViewImpl::constant(ScalarRefImpl::Int32(7), ROWS),
        ],
        [
            HandwrittenColumn::DenseArray(dense_left_values),
            HandwrittenColumn::Constant {
                value: 7,
                len: ROWS,
            },
        ],
    );
    benchmark_case(
        criterion,
        "dense/constant-array",
        [
            ColumnViewImpl::constant(ScalarRefImpl::Int32(7), ROWS),
            dense_right_view,
        ],
        [
            HandwrittenColumn::Constant {
                value: 7,
                len: ROWS,
            },
            HandwrittenColumn::DenseArray(dense_right_values),
        ],
    );
    benchmark_case(
        criterion,
        "dense/constant-constant",
        [
            ColumnViewImpl::constant(ScalarRefImpl::Int32(7), ROWS),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(11), ROWS),
        ],
        [
            HandwrittenColumn::Constant {
                value: 7,
                len: ROWS,
            },
            HandwrittenColumn::Constant {
                value: 11,
                len: ROWS,
            },
        ],
    );

    let nullable_left: ArrayImpl = I32Array::from_slice(
        &(0..ROWS)
            .map(|row| (row % 17 != 0).then_some(row as i32))
            .collect::<Vec<_>>(),
    )
    .into();
    benchmark_case(
        criterion,
        "fallback/nullable-array-array",
        [
            ColumnViewImpl::array(&nullable_left),
            ColumnViewImpl::array(&dense_right),
        ],
        [HandwrittenColumn::General, HandwrittenColumn::General],
    );
    benchmark_case(
        criterion,
        "fallback/array-null",
        [
            ColumnViewImpl::array(&dense_left),
            ColumnViewImpl::null(PhysicalType::Int32, ROWS),
        ],
        [HandwrittenColumn::General, HandwrittenColumn::General],
    );

    let indexed_values = (0..256)
        .map(|value| (value % 29 != 0).then_some(value * 5))
        .collect::<Vec<_>>();
    let indexed_values: ArrayImpl = I32Array::from_slice(&indexed_values).into();
    let indexed_indices = (0..ROWS).map(|row| (row % 256) as u32).collect::<Vec<_>>();
    let indexed = ColumnViewImpl::indexed(&indexed_indices, &indexed_values).unwrap();
    benchmark_case(
        criterion,
        "fallback/indexed-array",
        [indexed, ColumnViewImpl::array(&dense_right)],
        [HandwrittenColumn::General, HandwrittenColumn::General],
    );
}

fn criterion_config() -> Criterion {
    Criterion::default()
        .sample_size(20)
        .warm_up_time(Duration::from_millis(250))
        .measurement_time(Duration::from_millis(750))
}

criterion_group! {
    name = benches;
    config = criterion_config();
    targets = benchmark_expressions
}
criterion_main!(benches);
