// Copyright 2022-2026 Alex Chi. Licensed under Apache-2.0.

//! A batch-level asynchronous boundary for expressions.
//!
//! This example deliberately performs no I/O. It isolates the type-system choices required to
//! erase a future while keeping the main expression engine synchronous.

use std::future::Future;
use std::pin::Pin;

use anyhow::{Result, bail};
use expr_common::array::{ArrayImpl, I32Array};
use expr_common::column::ColumnViewImpl;
use expr_common::expr::Expression;

type BatchFuture<'a> = Pin<Box<dyn Future<Output = Result<ArrayImpl>> + Send + 'a>>;

trait AsyncExpression: Send + Sync {
    fn eval_async<'a>(&'a self, data: &'a [ColumnViewImpl<'a>]) -> BatchFuture<'a>;
}

struct AsyncAdapter<E>(E);

impl<E: Expression> AsyncExpression for AsyncAdapter<E> {
    fn eval_async<'a>(&'a self, data: &'a [ColumnViewImpl<'a>]) -> BatchFuture<'a> {
        Box::pin(async move { self.0.eval(data) })
    }
}

/// Static counterpart to the erased future below, with the RPIT spelling kept visible.
#[allow(clippy::manual_async_fn)]
fn eval_static<'a, E: Expression>(
    expression: &'a E,
    data: &'a [ColumnViewImpl<'a>],
) -> impl Future<Output = Result<ArrayImpl>> + Send + 'a {
    async move { expression.eval(data) }
}

struct IdentityExpression;

impl Expression for IdentityExpression {
    fn eval(&self, data: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl> {
        if data.len() != 1 {
            bail!("expected one input");
        }
        match data[0] {
            ColumnViewImpl::Array(array) => Ok(array.clone()),
            _ => bail!("expected one regular array input"),
        }
    }
}

fn main() {
    let input: ArrayImpl = I32Array::from_values(vec![1, 2, 3]).into();
    let views = [ColumnViewImpl::array(&input)];
    let expression = IdentityExpression;

    {
        let _static_future = eval_static(&expression, &views);
    }

    let expression: Box<dyn AsyncExpression> = Box::new(AsyncAdapter(expression));
    let _boxed_future = expression.eval_async(&views);
}

#[cfg(test)]
mod tests {
    use std::task::{Context, Poll, Waker};

    use expr_common::scalar::ScalarRefImpl;

    use super::*;

    fn poll_ready<F: Future + ?Sized>(mut future: Pin<&mut F>) -> F::Output {
        let mut context = Context::from_waker(Waker::noop());
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("the example future should complete without I/O"),
        }
    }

    #[test]
    fn evaluates_one_regular_array_through_static_and_erased_futures() {
        let input: ArrayImpl = I32Array::from_values(vec![1, 2, 3]).into();
        let views = [ColumnViewImpl::array(&input)];
        let expression = IdentityExpression;

        {
            let mut static_future = std::pin::pin!(eval_static(&expression, &views));
            let static_output = poll_ready(static_future.as_mut()).unwrap();
            assert_eq!(static_output.len(), 3);
            assert_eq!(static_output.get(0), Some(ScalarRefImpl::Int32(1)));
        }

        let expression: Box<dyn AsyncExpression> = Box::new(AsyncAdapter(expression));
        let mut boxed_future = expression.eval_async(&views);
        let erased_output = poll_ready(boxed_future.as_mut()).unwrap();
        assert_eq!(erased_output.len(), 3);
        assert_eq!(erased_output.get(2), Some(ScalarRefImpl::Int32(3)));
    }

    #[test]
    fn rejects_a_non_array_view_at_the_async_boundary() {
        let views = [ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 3)];
        let expression: Box<dyn AsyncExpression> = Box::new(AsyncAdapter(IdentityExpression));

        let mut future = expression.eval_async(&views);
        let error = poll_ready(future.as_mut()).unwrap_err();
        assert_eq!(error.to_string(), "expected one regular array input");
    }

    #[test]
    fn rejects_the_wrong_number_of_inputs_at_the_async_boundary() {
        let views = [];
        let expression: Box<dyn AsyncExpression> = Box::new(AsyncAdapter(IdentityExpression));

        let mut future = expression.eval_async(&views);
        let error = poll_ready(future.as_mut()).unwrap_err();
        assert_eq!(error.to_string(), "expected one input");
    }
}
