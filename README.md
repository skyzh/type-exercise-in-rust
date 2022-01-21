# Type Exercise in Rust

*(In Chinese) Rust 语言中的类型体操 - 以数据库系统为例*

This is a short lecture on how to use the Rust type system to build necessary components in a database system.

The lecture evolves around how Rust programmers (like me) build database systems in the Rust programming language. We leverage the Rust type system to **minimize** runtime cost and make our development process easier with **safe**, **nightly** Rust.

![Map of Types](map-of-types.png)

## Day 1: `Array` and `ArrayBuilder`

`ArrayBuilder` and `Array` are reciprocal traits. `ArrayBuilder` creates an `Array`, while we can create a new array
using `ArrayBuilder` with existing `Array`. In day 1, we implement arrays for primitive types (like `i32`, `f32`)
and for variable-length types (like `String`). We use associated types in traits to deduce the right type in generic
functions and use GAT to unify the `Array` interfaces for both fixed-length and variable-length types. This framework
is also very similar to libraries like Apache Arrow, but with much stronger type constraints and much lower runtime
overhead.

The special thing is that, we use blanket implementation for `i32` and `f32` arrays -- `PrimitiveArray<T>`. This would
make our journey much more challenging, as we need to carefully evaluate the trait bounds needed for them in the
following days.

### Goals

Developers can create generic functions over all types of arrays -- no matter fixed-length primitive array like
`I32Array`, or variable-length array like `StringArray`.

Without our `Array` trait, developers might to implement...

```rust
fn build_i32_array_from_vec(items: &[Option<i32>]) -> Vec<i32> { /* .. */ }
fn build_str_array_from_vec(items: &[Option<&str>]) -> Vec<String> { /* .. */ }
```

Note that the function takes different parameter -- one `i32` without lifetime, one `&str`. Our `Array` trait
can unify their behavior:

```rust
fn build_array_from_vec<A: Array>(items: &[Option<A::RefItem<'_>>]) -> A {
    let mut builder = A::Builder::with_capacity(items.len());
    for item in items {
        builder.push(*item);
    }
    builder.finish()
}

#[test]
fn test_build_int32_array() {
    let data = vec![Some(1), Some(2), Some(3), None, Some(5)];
    let array = build_array_from_vec::<I32Array>(&data[..]);
}

#[test]
fn test_build_string_array() {
    let data = vec![Some("1"), Some("2"), Some("3"), None, Some("5"), Some("")];
    let array = build_array_from_vec::<StringArray>(&data[..]);
}
```

## Day 2: `Scalar` and `ScalarRef`

`Scalar` and `ScalarRef` are reciprocal types. We can get a reference `ScalarRef` of a `Scalar`, and convert
`ScalarRef` back to `Scalar`. By adding these two traits, we can write more generic functions with zero runtime
overhead on type matching and conversion. Meanwhile, we associate `Scalar` with `Array`, so as to write functions
more easily.

### Goals

Without our `Scalar` implement, there could be problems:

```rust
fn build_array_repeated_owned<A: Array>(item: A::OwnedItem, len: usize) -> A {
    let mut builder = A::Builder::with_capacity(len);
    for _ in 0..len {
        builder.push(Some(item /* How to convert `item` to `RefItem`? */));
    }
    builder.finish()
}
```

With `Scalar` trait and corresponding implements,

```rust
fn build_array_repeated_owned<A: Array>(item: A::OwnedItem, len: usize) -> A {
    let mut builder = A::Builder::with_capacity(len);
    for _ in 0..len {
        builder.push(Some(item.as_scalar_ref())); // Now we have `as_scalar_ref` on `Scalar`!
    }
    builder.finish()
}
```

## Day 3: `ArrayImpl`, `ArrayBuilderImpl`, `ScalarImpl` and `ScalarRefImpl`

It could be possible that some information is not available until runtime. Therefore, we use `XXXImpl` enums to
cover all variants of a single type. At the same time, we also add `TryFrom<ArrayImpl>` and `Into<ArrayImpl>`
bound for `Array`.

### Goals

This is hard -- imagine we simply require `TryFrom<ArrayImpl>` and `Into<ArrayImpl>` bound on `Array`:

```rust
pub trait Array:
    Send + Sync + Sized + 'static + TryFrom<ArrayImpl> + Into<ArrayImpl>
```

Compiler will complain:

```
43 | impl<T> Array for PrimitiveArray<T>
   |         ^^^^^ the trait `From<PrimitiveArray<T>>` is not implemented for `array::ArrayImpl`
   |
   = note: required because of the requirements on the impl of `Into<array::ArrayImpl>` for `PrimitiveArray<T>`
```

This is because we use blanket implementation for `PrimitiveArray` to cover all primitive types. In day 3,
we learn how to correctly add bounds to `PrimitiveArray`.

## Day 4: More Types and Methods with Macro

`ArrayImpl` should supports common functions in traits, but `Array` trait doesn't have a unified interface for
all types -- `I32Array` accepts `get(&self, idx: usize) -> Option<i32>` while `StringArray` accepts
`get(&self, idx: usize) -> &str`. We need a `get(&self, idx:usize) -> ScalarRefImpl<'_>` on `ArrayImpl`. Therefore,
we have to write the match arms to dispatch the methods.

Also, we have written so many boilerplate code for `From` and `TryFrom`. We need to eliminate such duplicated code.

As we are having more and more data types, we need to write the same code multiple times within a match arm. In
day 4, we use declarative macros (instead of procedural macros or other kinds of code generator) to generate such
code and avoid writing boilerplate code.

### Goals

Before that, we need to implement every `TryFrom` or `Scalar` by ourselves:

```rust
impl<'a> ScalarRef<'a> for i32 {
    type ArrayType = I32Array;
    type ScalarType = i32;

    fn to_owned_scalar(&self) -> i32 {
        *self
    }
}

// repeat the same code fore i64, f64, ...
```

```rust
impl ArrayImpl {
    /// Get the value at the given index.
    pub fn get(&self, idx: usize) -> Option<ScalarRefImpl<'_>> {
        match self {
            Self::Int32(array) => array.get(idx).map(ScalarRefImpl::Int32),
            Self::Flaot64(array) => array.get(idx).map(ScalarRefImpl::Int64),
            // ...
            // repeat the types for every functions we added on `Array`
        }
    }
```

With macros, we can easily add more and more types. In day 4, we will support:

```rust
pub enum ArrayImpl {
    Int16(I16Array),
    Int32(I32Array),
    Int64(I64Array),
    Float32(F32Array),
    Float64(F64Array),
    Bool(BoolArray),
    String(StringArray),
}
```

With little code changed and eliminating boilerplate code.

## Day 5: Binary Expressions

Now that we have `Array`, `ArrayBuilder`, `Scalar` and `ScalarRef`, we can convert every function we wrote to a
vectorized one using generics.

### Goals

Developers will only need to implement:

```rust
pub fn cmp_le<'a, I1: Array, I2: Array, C: Array + 'static>(
    i1: I1::RefItem<'a>,
    i2: I2::RefItem<'a>,
) -> bool
where
    I1::RefItem<'a>: Into<C::RefItem<'a>>,
    I2::RefItem<'a>: Into<C::RefItem<'a>>,
    C::RefItem<'a>: PartialOrd,
{
    i1.into().partial_cmp(&i2.into()).unwrap() == Ordering::Less
}
```

And they can create `BinaryExpression` around this function with any type:

```rust
// Vectorize `cmp_le` to accept an array instead of a single value.
let expr = BinaryExpression::<I32Array, I32Array, BoolArray, _>::new(
        cmp_le::<I32Array, I32Array, I64Array>,
    );
// We only need to pass `ArrayImpl` to the expression, and it will do everything for us,
// including type checks, loopping, etc.
let result: ArrayImpl = expr.eval(ArrayImpl, ArrayImpl).unwrap();

// `cmp_le` can also be used on `&str`.
let expr = BinaryExpression::<StringArray, StringArray, BoolArray, _>::new(
        cmp_le::<StringArray, StringArray, StringArray>,
    );
let result: ArrayImpl = expr.eval(ArrayImpl, ArrayImpl).unwrap();
```

# TBD Lectures

## Day 6: Aggregators

Aggregators are another kind of expressions. We learn how to implement them easily with our type system in day 6.

## Day 7: Expression Framework

Now we are having more and more expression kinds, and we need an expression framework to unify them -- including
unary, binary and expressions of more inputs. At the same time, we also need to automatically convert `ArrayImpl`
into their corresponding concrete types using `TryFrom` and `TryInto` traits.

At the same time, we will also experiment with return value optimizations in variable-size types.

## Day 8: Physical Data Type and Logical Data Type

`i32`, `i64` is simply physical types -- how types are stored in memory (or on disk). But in a database system,
we also have logical types (like `Char`, and `Varchar`). In day 8, we learn how to associate logical types with
physical types using macros.
