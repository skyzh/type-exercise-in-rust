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

## Day 2: `Scalar` and `ScalarRef`

`Scalar` and `ScalarRef` are reciprocal types. We can get a reference `ScalarRef` of a `Scalar`, and convert
`ScalarRef` back to `Scalar`. By adding these two traits, we can write more generic functions with zero runtime
overhead on type matching and conversion. Meanwhile, we associate `Scalar` with `Array`, so as to write functions
more easily.

## Day 3: `ArrayImpl`, `ArrayBuilderImpl`, `ScalarImpl` and `ScalarRefImpl`

It could be possible that some information is not available until runtime. Therefore, we use `XXXImpl` enums to
cover all variants of a single type. At the same time, we also add `TryFrom<ArrayImpl>` and `Into<ArrayImpl>`
bound for `Array`.

## Day 4: More Types and Methods with Macro

`ArrayImpl` should supports common functions in traits, but `Array` trait doesn't have a unified interface for
all types -- `I32Array` accepts `get(&self, idx: usize) -> Option<i32>` while `StringArray` accepts
`get(&self, idx: usize) -> &str`. We need a `get(&self, idx:usize) -> ScalarRefImpl<'_>` on `ArrayImpl`. Therefore,
we have to write the match arms to dispatch the methods.

Also, we have written so many boilerplate code for `From` and `TryFrom`. We need to eliminate such duplicated code.

As we are having more and more data types, we need to write the same code multiple times within a match arm. In
day 4, we use declarative macros (instead of procedural macros or other kinds of code generator) to generate such
code and avoid writing boilerplate code.

# TBD Lectures

## Day 5: Binary Expressions

Now that we have `Array`, `ArrayBuilder`, `Scalar` and `ScalarRef`, we can convert every function we wrote to a
vectorized one using generics.

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
