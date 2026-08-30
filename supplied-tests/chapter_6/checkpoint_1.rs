use std::cell::Cell;

use crate::{Array, ArrayImpl, ColumnViewImpl, I32Array, evaluate_unary};

#[test]
fn checkpoint_1_strict_vectorization_skips_null_rows() {
    let calls = Cell::new(0);
    let input: ArrayImpl = I32Array::from_slice(&[Some(1), None, Some(3)]).into();
    let output = evaluate_unary::<i32, i32, _>(ColumnViewImpl::array(&input), |value| {
        calls.set(calls.get() + 1);
        value + 10
    })
    .unwrap();

    assert_eq!(calls.get(), 2);
    assert_eq!(
        <&I32Array>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(11), None, Some(13)]
    );
}
