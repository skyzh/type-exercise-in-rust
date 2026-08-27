{{#include wip-banner.md}}

# Chapter 5: Make Numeric Evaluation Generic

Chapter 4 separated a scalar operation from the batch work around it. Its checked binary shell
already validates two physical input families and their lengths, skips the scalar call for strict
nulls, builds the associated output array, and stops on the first scalar error. Writing another
copy of that loop for every numeric type pair would throw away the boundary you just established.

The remaining problem has two parts. Given logical types such as `SmallInt` and `Double`, the
database must first decide whether an implicit conversion is lossless and what logical type the
result has. Only then can it choose one concrete Rust scalar type for the row operation. This
chapter keeps those decisions separate: an explicit promotion table owns the database policy, and
a small runtime match chooses one generic typed kernel before the existing batch loop begins.

## What is in the starter

Begin from your completed Chapter 4 workspace. The checked unary and binary shells in
`src/operators.rs` are working code; do not replace their validation, null, output, or error
behavior. The Day 5 surface is still deliberately small:

- `src/promotion.rs` contains comment shells for one promotion row, the promotion catalog, and its
  lookup function;
- `src/operators.rs` ends with comments for the arithmetic and comparison selectors;
- `src/array/primitive_array.rs` has the Arrow-style value and validity buffers but not the
  all-valid constructor used by this chapter's batch fixture; and
- `src/lib.rs` leaves the promotion module and the two operator enums unwired.

You own three connected additions: the logical promotion policy, generic arithmetic selection,
and generic numeric comparison. You will also add the small `PrimitiveArray::from_values` helper
needed to construct a non-null batch directly. Leave shared arity validation and ternary
evaluation for Chapter 6, runtime expression erasure for Chapter 8, and logical name binding for
Chapter 9.

Copy the cumulative supplied test before editing:

```console
cargo x copy-test --chapter 5
cargo test -p type-exercise-starter chapter_5 --locked
```

The focused run should fail on the missing promotion items, operator selectors, and
`PrimitiveArray::from_values`. Do not edit the copied test.

## Checkpoint 1: make widening a database policy

Open `src/promotion.rs` and define the public shape already named by the starter:

```rust,ignore
pub struct NumericPromotion {
    pub left: DataType,
    pub right: DataType,
    pub output: DataType,
}

pub const NUMERIC_PROMOTIONS: &[NumericPromotion] = /* every supported ordered pair */;

pub fn promote_numeric(
    left: impl Borrow<DataType>,
    right: impl Borrow<DataType>,
) -> Option<DataType>;
```

This is an ordered-pair catalog, not a request to let Rust choose an `as` cast. Enter both operand
orders whenever both are supported. The complete policy for the five non-Decimal numeric types is:

| left ↓ / right → | `SmallInt` | `Integer` | `BigInt` | `Real` | `Double` |
| --- | --- | --- | --- | --- | --- |
| `SmallInt` | `SmallInt` | `Integer` | `BigInt` | `Real` | `Double` |
| `Integer` | `Integer` | `Integer` | `BigInt` | `Double` | `Double` |
| `BigInt` | `BigInt` | `BigInt` | `BigInt` | reject | reject |
| `Real` | `Real` | `Double` | reject | `Real` | `Double` |
| `Double` | `Double` | `Double` | reject | `Double` | `Double` |

The unusual-looking rows state the rule. Every `i16` value is exact in `f32`, so `SmallInt` with
`Real` may stay `Real`. Every `i32` value is exact in `f64` but not in `f32`, so `Integer` with
`Real` widens to `Double`. Neither `f32` nor `f64` represents every `i64` value, so every
`BigInt`/floating-point pair is rejected even though Rust can spell the cast.

`Decimal` is also a numeric logical type, but it gets no row in this table. Precision, scale,
rounding, overflow, and division scale need a separate contract before an implicit Decimal
operation is meaningful. A physical representation alone does not supply those semantics.

Implement `promote_numeric` as a catalog lookup that returns the row's logical output or `None`.
Do not infer a fallback from enum order or substitute a duplicate row: the supplied test audits all
25 ordered input pairs and the exact 21 supported catalog keys.

Enable `promotion` in `src/lib.rs` and export `NumericPromotion`, `NUMERIC_PROMOTIONS`, and
`promote_numeric`. The final focused test also imports the later operator selectors, so it cannot
be green at this checkpoint. Use the library boundary instead:

```console
cargo check -p type-exercise-starter --lib --locked
```

Passing means the logical policy and its public lookup compile independently from physical
evaluation.

## Checkpoint 2: choose one arithmetic kernel before the rows

Start with the fixture helper in `src/array/primitive_array.rs`. It keeps the existing
representation and marks every supplied value valid:

```rust,ignore
impl<T> PrimitiveArray<T> {
    pub fn from_values(values: Vec<T>) -> Self {
        let validity = BitVec::repeat(true, values.len());
        Self { values, validity }
    }
}
```

This constructor is not a second array format and does not change null handling. It is simply the
direct counterpart to building a batch whose rows are all non-null.

Now extend `src/operators.rs` with the public `ArithmeticOperator` variants `Add`, `Subtract`,
`Multiply`, and `Divide`. The supported solution then uses a private `Numeric` trait for the
behavior shared by the five concrete output scalar types:

```rust,ignore
trait Numeric: Scalar + Copy + PartialOrd {
    fn add(self, rhs: Self) -> Self;
    fn subtract(self, rhs: Self) -> Self;
    fn multiply(self, rhs: Self) -> Self;
    fn checked_divide(self, rhs: Self) -> Result<Self, ScalarError>;
}
```

Implement it explicitly for `i16`, `i32`, `i64`, `f32`, and `f64`. Express addition,
subtraction, and multiplication through the standard `Add`, `Sub`, and `Mul` traits. For signed
integers, apply those traits to `std::num::Wrapping<T>` and recover `.0`; this keeps the course's
deterministic wrapping result in debug and release builds. Standard `Add` on a bare signed integer
does not itself choose one cross-profile overflow policy, so changing overflow into an error would
be a separate product-semantic decision rather than part of this generic refactor. Floating-point
implementations use the standard traits directly and retain ordinary IEEE results.

Division stays the one small custom fallible operation because stable `std` has no single checked
division trait covering both the course's integers and floats. Integer division reports
`DivisionByZero` for zero and `DivisionOverflow` for `MIN / -1`. Treat both `0.0` and `-0.0`
floating-point divisors as division by zero; other results such as infinity or NaN remain values.

The important Rust boundary is where all three generic types become concrete. A crate-private
`NumericBinary<L, R, O>` implements the Chapter 4 `CheckedBinaryScalarFunction` with typed
associated inputs `L` and `R`. A small `PromoteInto<O>` relationship performs only the lossless
conversions admitted by the promotion table. The physical builder matches the validated
`(left, right, output)` tuple once and stores the selected monomorphized whole-batch function
pointer in one concrete `NumericBinaryExpression`.

That function pointer enters the existing typed Chapter 4 batch adapter. It converts each erased
column to its typed view once, then the row loop receives `L` and `R` values directly, promotes
them to `O`, and applies the selected operation. Do not accept `ScalarRefImpl` in the checked hook,
re-run logical promotion, or match physical variants inside every row. The caller must obtain the
logical output from `promote_numeric` first; an unsupported pair never reaches the physical
builder.

Keep `build_numeric_binary_expression` and its returned shell crate-private. Export
`ArithmeticOperator` from the crate root, but do not turn the physical constructor into a public
user API: Chapter 9 will place logical name binding in front of it.

The copied test still imports numeric comparison, so use the library compile boundary again:

```console
cargo check -p type-exercise-starter --lib --locked
```

Passing means all four arithmetic choices can share one typed scalar implementation and the
existing checked batch shell without widening the public runtime boundary.

## Checkpoint 3: return Boolean through the same common type

Add the six public `ComparisonOperator` variants: `Less`, `LessOrEqual`, `Greater`,
`GreaterOrEqual`, `Equal`, and `NotEqual`. A crate-private `NumericCompare<L, R, O>` reuses the same
typed `PromoteInto<O>` conversions and tuple-selected batch kernel, but its associated output is
`bool`. Keep
`build_numeric_comparison_expression` crate-private.

This is why the associated output family from Chapter 4 matters. Both inputs may be promoted to
`f64` for the scalar comparison while the batch shell builds a `BoolArray`. There is no separate
comparison row loop.

Rust's floating-point comparisons supply the required NaN behavior: `<`, `<=`, `>`, `>=`, and
`=` are false when either relevant comparison is unordered, while `!=` is true. Do not turn NaN
into a batch error. Strict null handling remains different: if either input row is null, the
checked shell appends null and never calls `NumericCompare<O>`.

Export `ComparisonOperator` beside `ArithmeticOperator`, then run the completed contract:

```console
cargo test -p type-exercise-starter chapter_5 --locked
cargo test -p type-exercise-starter --lib --locked
```

The 9 focused cases and 42 cumulative learner tests prove the whole Day 5 boundary:

- the catalog contains exactly the approved ordered promotions and rejects every lossy pair;
- arithmetic works in both mixed operand orders and builds the promoted physical family;
- signed overflow wraps, while division by zero and signed division overflow stop the batch;
- a strict null prevents even a failing divide from being called;
- nonzero IEEE results and all six comparison operators retain their defined behavior; and
- comparison reuses the Chapter 4 arity, physical-type, length, null, and complete-output rules.

## Read the two decisions separately

The promotion table and the generic kernel solve different problems. The table answers a logical
question before evaluation: “Is this implicit conversion allowed, and what is the result type?”
The physical match answers a Rust question once: “Which concrete `Scalar` implements this
operation?” The checked shell then answers the batch question for every row. Collapsing those
three stages into `as f64`, a per-row type match, or another handwritten loop would make the code
shorter by hiding the policy you need to audit.

Before continuing, make sure you can explain these boundaries in your own words:

1. Why may `SmallInt + Real` produce `Real` while `Integer + Real` produces `Double`?
2. Why is every `BigInt`/floating-point pair absent even though Rust provides an `as` conversion?
3. Why does the physical builder select `(L, R, O)` once instead of matching scalar variants in
   each row?
4. Why is `null / 0` a null row rather than a division error?

You now have generic numeric operation selection without changing the batch contract that made
the concrete loops correct. Chapter 6 will reuse their validation rules across arities and add a
real ternary path.

Next: [Chapter 6 makes expression arity systematic](./chapter-6-systematic-arity.md).

{{#include copyright.md}}
