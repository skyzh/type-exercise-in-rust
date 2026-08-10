//! Frozen learner-owned API map for the cumulative starter.
//!
//! This compiled ledger is deliberately independent of the editable TOML and
//! Markdown views. Days 3-6 and 8-13 are traced to the reviewed downstream
//! chapter sources; Day 7 is the separately approved Boolean insertion.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ApprovedTarget {
    pub day: usize,
    pub source: &'static str,
    pub title: &'static str,
    pub file: &'static str,
    pub items: &'static [&'static str],
    pub declarations: &'static [&'static str],
    pub materialized: bool,
}

macro_rules! target {
    ($day:literal, $source:literal, $title:literal, $file:literal,
     [$($item:literal),+ $(,)?], [$($declaration:literal),+ $(,)?], $materialized:literal) => {
        ApprovedTarget {
            day: $day,
            source: $source,
            title: $title,
            file: $file,
            items: &[$($item),+],
            declarations: &[$($declaration),+],
            materialized: $materialized,
        }
    };
}

pub(crate) const APPROVED_TARGETS: &[ApprovedTarget] = &[
    target!(
        1,
        "course/src/chapter-1-type-family.md",
        "Physical rows and checked mismatch",
        "src/physical_type.rs",
        ["PhysicalType", "TypeMismatch"],
        ["pub enum PhysicalType", "pub struct TypeMismatch"],
        true
    ),
    target!(
        1,
        "course/src/chapter-1-type-family.md",
        "Owned and borrowed scalar contract",
        "src/scalar.rs",
        ["Scalar", "ScalarRef", "ScalarImpl", "ScalarRefImpl"],
        [
            "pub trait Scalar",
            "pub trait ScalarRef",
            "pub enum ScalarImpl",
            "pub enum ScalarRefImpl"
        ],
        true
    ),
    target!(
        1,
        "course/src/chapter-1-type-family.md",
        "Nullable array contract and erasure",
        "src/array.rs",
        ["Array", "ArrayBuilder", "ArrayImpl"],
        [
            "pub trait Array",
            "pub trait ArrayBuilder",
            "pub enum ArrayImpl"
        ],
        true
    ),
    target!(
        1,
        "course/src/chapter-1-type-family.md",
        "Flat fixed-width buffers",
        "src/array/primitive_array.rs",
        ["PrimitiveArray", "PrimitiveArrayBuilder"],
        [
            "pub struct PrimitiveArray",
            "pub struct PrimitiveArrayBuilder"
        ],
        true
    ),
    target!(
        1,
        "course/src/chapter-1-type-family.md",
        "Shared UTF-8 bytes and offsets",
        "src/array/string_array.rs",
        ["StringArray", "StringArrayBuilder"],
        ["pub struct StringArray", "pub struct StringArrayBuilder"],
        true
    ),
    target!(
        2,
        "course/src/chapter-2-type-catalog.md",
        "Single physical-family catalog",
        "src/variant_catalog.rs",
        ["for_each_physical_family"],
        ["macro_rules! for_each_physical_family"],
        true
    ),
    target!(
        2,
        "course/src/chapter-2-type-catalog.md",
        "Logical-to-physical mapping",
        "src/data_type.rs",
        ["DataType", "DataType::decimal", "DataType::physical_type"],
        [
            "pub enum DataType",
            "pub fn decimal(",
            "pub fn physical_type("
        ],
        true
    ),
    target!(
        2,
        "course/src/chapter-2-type-catalog.md",
        "Checked Decimal descriptor and scalar",
        "src/decimal.rs",
        [
            "DecimalType",
            "DecimalType::try_new",
            "Decimal",
            "Decimal::try_new",
            "DecimalError"
        ],
        [
            "pub struct DecimalType",
            "pub fn try_new(",
            "pub struct Decimal",
            "pub fn try_new(",
            "pub enum DecimalError"
        ],
        true
    ),
    target!(
        2,
        "course/src/chapter-2-type-catalog.md",
        "Metadata-aware Decimal storage",
        "src/array/decimal_array.rs",
        [
            "DecimalArray",
            "DecimalArray::try_from_raw_parts",
            "DecimalArrayBuilder",
            "DecimalArrayBuilder::try_with_type",
            "DecimalArrayBuilder::try_push"
        ],
        [
            "pub struct DecimalArray",
            "pub fn try_from_raw_parts(",
            "pub struct DecimalArrayBuilder",
            "pub fn try_with_type(",
            "pub fn try_push("
        ],
        true
    ),
    target!(
        2,
        "course/src/chapter-2-type-catalog.md",
        "Exact Decimal scalar erasure",
        "src/scalar.rs",
        [
            "ScalarImpl::try_decimal",
            "ScalarRefImpl::try_decimal",
            "From<Decimal> for ScalarRefImpl",
            "TryFrom<ScalarRefImpl> for Decimal"
        ],
        [
            "pub fn try_decimal(&self,",
            "pub fn try_decimal(self,",
            "impl<'a> From<Decimal> for ScalarRefImpl<'a>",
            "impl TryFrom<ScalarRefImpl<'_>> for Decimal"
        ],
        true
    ),
    target!(
        3,
        "course/src/chapter-3-column-views.md",
        "Checked column representations and typed views",
        "src/column.rs",
        [
            "ColumnViewImpl",
            "ColumnViewImpl::array",
            "ColumnViewImpl::constant",
            "ColumnViewImpl::null",
            "ColumnViewImpl::dictionary",
            "ColumnView",
            "ColumnView::get",
            "ColumnView::len",
            "TryFrom<ColumnViewImpl> for ColumnView"
        ],
        [
            "pub struct ColumnViewImpl",
            "pub fn array(",
            "pub fn constant(",
            "pub fn null(",
            "pub fn dictionary(",
            "pub struct ColumnView",
            "pub fn get(",
            "pub fn len(",
            "impl<'a, S> TryFrom<ColumnViewImpl<'a>> for ColumnView<'a, S>"
        ],
        false
    ),
    target!(
        4,
        "course/src/chapter-4-concrete-loops.md",
        "Initial binary scalar function",
        "src/expression.rs",
        ["BinaryScalarFunction", "I32Add", "evaluate_binary"],
        [
            "pub trait BinaryScalarFunction",
            "pub struct I32Add",
            "pub fn evaluate_binary("
        ],
        false
    ),
    target!(
        4,
        "course/src/chapter-4-concrete-loops.md",
        "Checked unary and binary expression shells",
        "src/operators.rs",
        [
            "CheckedUnaryScalarFunction",
            "CheckedBinaryScalarFunction",
            "UnaryExpression",
            "CheckedBinaryExpression"
        ],
        [
            "pub trait CheckedUnaryScalarFunction",
            "pub trait CheckedBinaryScalarFunction",
            "pub struct UnaryExpression",
            "pub struct CheckedBinaryExpression"
        ],
        false
    ),
    target!(
        5,
        "course/src/chapter-5-generic-arithmetic.md",
        "Lossless numeric promotion",
        "src/promotion.rs",
        ["NumericPromotion", "NUMERIC_PROMOTIONS", "promote_numeric"],
        [
            "pub struct NumericPromotion",
            "pub const NUMERIC_PROMOTIONS",
            "pub fn promote_numeric("
        ],
        false
    ),
    target!(
        5,
        "course/src/chapter-5-generic-arithmetic.md",
        "Arithmetic and numeric comparison kernels",
        "src/operators.rs",
        [
            "ArithmeticOperator",
            "CheckedBinaryExpression",
            "build_numeric_binary_expression",
            "ComparisonOperator",
            "build_numeric_comparison_expression"
        ],
        [
            "pub enum ArithmeticOperator",
            "pub struct CheckedBinaryExpression",
            "pub(crate) fn build_numeric_binary_expression(",
            "pub enum ComparisonOperator",
            "pub(crate) fn build_numeric_comparison_expression("
        ],
        false
    ),
    target!(
        6,
        "course/src/chapter-6-systematic-arity.md",
        "Shared expression input validation",
        "src/operators.rs",
        ["validate_expression_inputs"],
        ["pub fn validate_expression_inputs("],
        false
    ),
    target!(
        6,
        "course/src/chapter-6-systematic-arity.md",
        "Structured expression errors",
        "src/expression.rs",
        ["ExpressionError"],
        ["pub enum ExpressionError"],
        false
    ),
    target!(
        6,
        "course/src/chapter-6-systematic-arity.md",
        "Checked ternary expression and clamp",
        "src/operators.rs",
        [
            "CheckedTernaryScalarFunction",
            "TernaryExpression",
            "build_numeric_clamp_expression"
        ],
        [
            "pub trait CheckedTernaryScalarFunction",
            "pub struct TernaryExpression",
            "pub(crate) fn build_numeric_clamp_expression("
        ],
        false
    ),
    target!(
        7,
        "approved inserted Day 7 Boolean checkpoint",
        "Three-valued Boolean logic",
        "src/boolean_logic.rs",
        [
            "NullEvaluationPolicy",
            "BooleanOperator",
            "build_boolean_expression",
            "BOOLEAN_TRUTH_TABLE"
        ],
        [
            "pub enum NullEvaluationPolicy",
            "pub enum BooleanOperator",
            "pub fn build_boolean_expression(",
            "pub const BOOLEAN_TRUTH_TABLE"
        ],
        false
    ),
    target!(
        8,
        "course/src/chapter-7-runtime-erasure.md",
        "Runtime expression erasure and builtin catalog",
        "src/expression.rs",
        [
            "Expression",
            "ExpressionError",
            "BinaryExpression",
            "define_builtin_expressions",
            "build_builtin_expression",
            "BUILTIN_EXPRESSION_NAMES"
        ],
        [
            "pub trait Expression",
            "pub enum ExpressionError",
            "pub struct BinaryExpression",
            "macro_rules! define_builtin_expressions",
            "pub fn build_builtin_expression(",
            "pub const BUILTIN_EXPRESSION_NAMES"
        ],
        false
    ),
    target!(
        8,
        "course/src/chapter-7-runtime-erasure.md",
        "Erased checked operator shells",
        "src/operators.rs",
        [
            "Expression for UnaryExpression",
            "Expression for CheckedBinaryExpression",
            "Expression for TernaryExpression"
        ],
        [
            "Expression for UnaryExpression",
            "Expression for CheckedBinaryExpression",
            "Expression for TernaryExpression"
        ],
        false
    ),
    target!(
        9,
        "course/src/chapter-8-binding-coercion.md",
        "Binder and runtime function registry",
        "src/binder.rs",
        [
            "BindError",
            "BoundExpression",
            "FunctionRegistry",
            "FunctionRegistry::register",
            "FunctionRegistry::register_unary",
            "FunctionRegistry::register_binary",
            "FunctionRegistry::register_ternary",
            "FunctionRegistry::bind",
            "bind_arithmetic",
            "bind_comparison"
        ],
        [
            "pub enum BindError",
            "pub struct BoundExpression",
            "pub struct FunctionRegistry",
            "pub fn register(",
            "pub fn register_unary(",
            "pub fn register_binary(",
            "pub fn register_ternary(",
            "pub fn bind(",
            "fn bind_arithmetic(",
            "fn bind_comparison("
        ],
        false
    ),
    target!(
        9,
        "course/src/chapter-8-binding-coercion.md",
        "Logical comparison operators",
        "src/operators.rs",
        ["ComparisonOperator"],
        ["pub enum ComparisonOperator"],
        false
    ),
    target!(
        10,
        "course/src/chapter-9-primitive-loops.md",
        "Representative primitive fast loops",
        "src/expression.rs",
        [
            "PrimitiveLoop",
            "PrimitiveBinaryExpression::evaluate_with_loop"
        ],
        ["pub enum PrimitiveLoop", "pub fn evaluate_with_loop("],
        false
    ),
    target!(
        10,
        "course/src/chapter-9-primitive-loops.md",
        "Bound fast-loop forwarding",
        "src/binder.rs",
        ["BoundExpression::evaluate_with_loop"],
        ["pub fn evaluate_with_loop("],
        false
    ),
    target!(
        11,
        "course/src/chapter-10-list.md",
        "One-level List scalars and arrays",
        "src/array/list_array.rs",
        [
            "ListError",
            "ListScalar",
            "ListScalarRef",
            "ListArray",
            "ListArrayBuilder",
            "ListArray::try_from_rows",
            "ListArray::try_from_raw_parts"
        ],
        [
            "pub enum ListError",
            "pub struct ListScalar",
            "pub struct ListScalarRef",
            "pub struct ListArray",
            "pub struct ListArrayBuilder",
            "pub fn try_from_rows",
            "pub fn try_from_raw_parts("
        ],
        false
    ),
    target!(
        11,
        "course/src/chapter-10-list.md",
        "Logical List type",
        "src/data_type.rs",
        ["DataType::List"],
        ["List(Box<DataType>)"],
        false
    ),
    target!(
        11,
        "course/src/chapter-10-list.md",
        "Physical List type",
        "src/physical_type.rs",
        ["PhysicalType::List"],
        ["List(Box<PhysicalType>)"],
        false
    ),
    target!(
        11,
        "course/src/chapter-10-list.md",
        "Owned and borrowed List scalar variants",
        "src/scalar.rs",
        ["ScalarImpl::List", "ScalarRefImpl::List"],
        ["List(ListScalar)", "List(ListScalarRef<'a>)"],
        false
    ),
    target!(
        11,
        "course/src/chapter-10-list.md",
        "List array erasure",
        "src/array.rs",
        [
            "From<ListArray> for ArrayImpl",
            "TryFrom<ArrayImpl> for ListArray"
        ],
        [
            "impl From<ListArray> for ArrayImpl",
            "impl TryFrom<ArrayImpl> for ListArray"
        ],
        false
    ),
    target!(
        11,
        "course/src/chapter-10-list.md",
        "List column downcast",
        "src/column.rs",
        ["ListColumnView", "ColumnViewImpl::try_as_list"],
        ["pub struct ListColumnView", "pub fn try_as_list("],
        false
    ),
    target!(
        12,
        "course/src/chapter-11-rust-boundaries.md",
        "Opaque array iteration",
        "src/array.rs",
        ["Array::iter"],
        ["fn iter<'a>(&'a self) -> impl Iterator"],
        true
    ),
    target!(
        12,
        "course/src/chapter-11-rust-boundaries.md",
        "Private concrete array iterator",
        "src/array/iterator.rs",
        ["ArrayIterator"],
        ["pub struct ArrayIterator"],
        false
    ),
    target!(
        12,
        "course/src/chapter-11-rust-boundaries.md",
        "Thread-safe erased expression boundary",
        "src/expression.rs",
        ["Expression: Any + Send + Sync"],
        ["pub trait Expression: Any + Send + Sync"],
        false
    ),
    target!(
        12,
        "course/src/chapter-11-rust-boundaries.md",
        "Shareable registry factories",
        "src/binder.rs",
        [
            "FunctionRegistry::register",
            "FunctionRegistry::register_unary",
            "FunctionRegistry::register_binary",
            "FunctionRegistry::register_ternary"
        ],
        [
            "pub fn register(",
            "pub fn register_unary(",
            "pub fn register_binary(",
            "pub fn register_ternary("
        ],
        false
    ),
    target!(
        12,
        "course/src/chapter-11-rust-boundaries.md",
        "Borrowed column-view lifetime",
        "src/column.rs",
        ["ColumnViewImpl<'a>"],
        ["pub struct ColumnViewImpl<'a>"],
        false
    ),
    target!(
        13,
        "course/src/chapter-12-async-boundary.md",
        "Static and erased batch futures",
        "src/expression.rs",
        [
            "evaluate_static",
            "BatchFuture",
            "AsyncExpression",
            "AsyncExpressionAdapter"
        ],
        [
            "pub fn evaluate_static",
            "pub type BatchFuture",
            "pub trait AsyncExpression",
            "pub struct AsyncExpressionAdapter"
        ],
        false
    ),
    target!(
        13,
        "course/src/chapter-12-async-boundary.md",
        "Bound asynchronous forwarding",
        "src/binder.rs",
        ["BoundExpression::evaluate_async"],
        ["pub fn evaluate_async("],
        false
    ),
];
