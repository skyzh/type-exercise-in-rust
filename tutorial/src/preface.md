# Preface

This course teaches you to build a database expression framework in Rust. The Rust type system is a
tool we will use along the way, not the product we are trying to ship.

A database expression engine sits between a query plan and batches of data. It must answer four
different questions:

1. Is `smallint + integer` a valid expression, and what type does it return?
2. How do we execute that expression over nullable columns efficiently?
3. How do we accept values whose physical representation is only known at runtime?
4. How can a new string, JSON, decimal, or list function join the framework without duplicating the
   entire executor?

The original version of this course approached those questions as a sequence of Rust type-system
exercises. That route exposed useful techniques, but it made the machinery feel universal. It is
not. Generic expansion is excellent for the dense families in a database system: numeric
promotion, comparisons, casts, and a few boolean operators. Most expressions are specific to one
or two types. A JSON path function, a decimal rounding function, and a string regular-expression
function do not become simpler when forced through one universal generic abstraction.

The revised framework uses this division of responsibility:

```text
SQL / plan
    |
    v
logical binder ---- rejects invalid signatures and selects promotions
    |
    v
bound expression -- concrete scalar types hidden behind a dyn-compatible trait
    |
    v
column views ------ array, constant, or dictionary without materialization
    |
    v
typed kernel ------ generated numeric family or explicit custom function
    |
    v
materialized output array
```

## What You Will Build

By the end of the course, the repository contains:

- Arrow-like fixed-width, variable-width, and nested arrays;
- an owned/borrowed scalar model expressed with generic associated types (GATs);
- a generic `ColumnView<'a, S>` over regular arrays, constants, and dictionaries;
- runtime enums that erase physical types only at framework boundaries;
- generated vectorizers for functions with one through five arguments;
- a planning-time registry that returns a `BoundExpression`;
- a promotion matrix for numeric and comparison families;
- ordinary custom kernels such as `str_contains`; and
- Criterion benchmarks against hand-written monomorphic loops.

## What Changed from the Original Articles

This book is a rewrite of the original three-part Chinese article series.
It preserves the explanations of arrays, scalars, GATs, type erasure, callback-style macros, and
logical/physical type mapping. It changes the order so every chapter answers a database-design
question raised by the previous chapter.

Generic associated types are stable, the course uses stable Rust with Edition 2024, and the
implementation states the required higher-ranked, cross-lifetime bounds directly. Historical
snapshots remain under `archive/`, but they are not the recommended design.

The binder and registry borrow a focused idea from
[`andylokandy/typed-type-exercise-in-rust`](https://github.com/andylokandy/typed-type-exercise-in-rust):
check logical types before evaluation so function authors do not hand-write unsafe or fallible
downcasts. This course keeps that idea compatible with its existing `Array`, `Scalar`, and
`ArrayImpl` model rather than adopting a separate value system.

## Prerequisites

You should be comfortable with Rust enums, traits, associated types, lifetimes, iterators, and
declarative macros. Familiarity with SQL null semantics and columnar execution is helpful but not
required.

Each implementation chapter includes a checkpoint and questions. Predict what the compiler or
runtime should do before reading the next section; the type relationships are easier to remember
when they solve a concrete failure you have already seen.

If you are using the course as an advanced Rust syllabus, the
[language-concept appendix](./appendix/rust_language.md) maps each concept to the exact framework
task where it matters. It is a navigation aid, not a second feature-driven course.

## Community

Join skyzh's Discord server to study with the database systems community.

[![Join skyzh's Discord Server](discord-badge.svg)](https://skyzh.dev/join/discord)

Continue to [Environment Setup](./getting_started.md).

{{#include copyright.md}}
