# Starter API Roadmap

This roadmap names the declaration surface each day asks you to implement. Days 1–2 are the only
new Rust scaffolds present in this starter snapshot. Later entries are documentation, not hidden
implementations: their files and declarations appear only when that cumulative course day lands,
except for APIs such as `Array::iter` that an earlier checkpoint already requires and a later day
revisits. Every listed body remains learner work.

The compiled source ledger in `xtask/src/starter_api_contract.rs` is authoritative. This table and
`api-roadmap.toml` are checked views of that frozen chapter-to-source mapping; editing both views
cannot change day ownership or move an API to another file.

| Day | File | Checkpoint | Items and declaration shapes |
| ---: | --- | --- | --- |
| 1 | `src/physical_type.rs` | Physical rows and checked mismatch | `PhysicalType` → `pub enum PhysicalType`; `TypeMismatch` → `pub struct TypeMismatch` |
| 1 | `src/scalar.rs` | Owned and borrowed scalar contract | `Scalar` → `pub trait Scalar`; `ScalarRef` → `pub trait ScalarRef`; `ScalarImpl` → `pub enum ScalarImpl`; `ScalarRefImpl` → `pub enum ScalarRefImpl` |
| 1 | `src/array.rs` | Nullable array contract and erasure | `Array` → `pub trait Array`; `ArrayBuilder` → `pub trait ArrayBuilder`; `ArrayImpl` → `pub enum ArrayImpl` |
| 1 | `src/array/primitive_array.rs` | Flat fixed-width buffers | `PrimitiveArray` → `pub struct PrimitiveArray`; `PrimitiveArrayBuilder` → `pub struct PrimitiveArrayBuilder` |
| 1 | `src/array/string_array.rs` | Shared UTF-8 bytes and offsets | `StringArray` → `pub struct StringArray`; `StringArrayBuilder` → `pub struct StringArrayBuilder` |
| 2 | `src/variant_catalog.rs` | Single physical-family catalog | `for_each_physical_family` → `macro_rules! for_each_physical_family` |
| 2 | `src/data_type.rs` | Logical-to-physical mapping | `DataType` → `pub enum DataType`; `DataType::decimal` → `pub fn decimal(`; `DataType::physical_type` → `pub fn physical_type(` |
| 2 | `src/decimal.rs` | Checked Decimal descriptor and scalar | `DecimalType` → `pub struct DecimalType`; `DecimalType::try_new` → `pub fn try_new(`; `Decimal` → `pub struct Decimal`; `Decimal::try_new` → `pub fn try_new(`; `DecimalError` → `pub enum DecimalError` |
| 2 | `src/array/decimal_array.rs` | Metadata-aware Decimal storage | `DecimalArray` → `pub struct DecimalArray`; `DecimalArray::try_from_raw_parts` → `pub fn try_from_raw_parts(`; `DecimalArrayBuilder` → `pub struct DecimalArrayBuilder`; `DecimalArrayBuilder::try_with_type` → `pub fn try_with_type(`; `DecimalArrayBuilder::try_push` → `pub fn try_push(` |
| 2 | `src/scalar.rs` | Exact Decimal scalar erasure | `ScalarImpl::try_decimal` → `pub fn try_decimal(&self,`; `ScalarRefImpl::try_decimal` → `pub fn try_decimal(self,`; `From<Decimal> for ScalarRefImpl` → `impl<'a> From<Decimal> for ScalarRefImpl<'a>`; `TryFrom<ScalarRefImpl> for Decimal` → `impl TryFrom<ScalarRefImpl<'_>> for Decimal` |
| 3 | `src/column.rs` | Checked column representations and typed views | `ColumnViewImpl` → `pub struct ColumnViewImpl`; `ColumnViewImpl::array` → `pub fn array(`; `ColumnViewImpl::constant` → `pub fn constant(`; `ColumnViewImpl::null` → `pub fn null(`; `ColumnViewImpl::dictionary` → `pub fn dictionary(`; `ColumnView` → `pub struct ColumnView`; `ColumnView::get` → `pub fn get(`; `ColumnView::len` → `pub fn len(`; `TryFrom<ColumnViewImpl> for ColumnView` → `impl<'a, S> TryFrom<ColumnViewImpl<'a>> for ColumnView<'a, S>` |
| 4 | `src/expression.rs` | Initial binary scalar function | `BinaryScalarFunction` → `pub trait BinaryScalarFunction`; `I32Add` → `pub struct I32Add`; `evaluate_binary` → `pub fn evaluate_binary(` |
| 4 | `src/operators.rs` | Checked unary and binary expression shells | `CheckedUnaryScalarFunction` → `pub trait CheckedUnaryScalarFunction`; `CheckedBinaryScalarFunction` → `pub trait CheckedBinaryScalarFunction`; `UnaryExpression` → `pub struct UnaryExpression`; `CheckedBinaryExpression` → `pub struct CheckedBinaryExpression` |
| 5 | `src/promotion.rs` | Lossless numeric promotion | `NumericPromotion` → `pub struct NumericPromotion`; `NUMERIC_PROMOTIONS` → `pub const NUMERIC_PROMOTIONS`; `promote_numeric` → `pub fn promote_numeric(` |
| 5 | `src/operators.rs` | Arithmetic and numeric comparison kernels | `ArithmeticOperator` → `pub enum ArithmeticOperator`; `CheckedBinaryExpression` → `pub struct CheckedBinaryExpression`; `build_numeric_binary_expression` → `pub(crate) fn build_numeric_binary_expression(`; `ComparisonOperator` → `pub enum ComparisonOperator`; `build_numeric_comparison_expression` → `pub(crate) fn build_numeric_comparison_expression(` |
| 6 | `src/operators.rs` | Shared expression input validation | `validate_expression_inputs` → `pub fn validate_expression_inputs(` |
| 6 | `src/expression.rs` | Structured expression errors | `ExpressionError` → `pub enum ExpressionError` |
| 6 | `src/operators.rs` | Checked ternary expression and clamp | `CheckedTernaryScalarFunction` → `pub trait CheckedTernaryScalarFunction`; `TernaryExpression` → `pub struct TernaryExpression`; `build_numeric_clamp_expression` → `pub(crate) fn build_numeric_clamp_expression(` |
| 7 | `src/boolean_logic.rs` | Three-valued Boolean logic | `NullEvaluationPolicy` → `pub enum NullEvaluationPolicy`; `BooleanOperator` → `pub enum BooleanOperator`; `build_boolean_expression` → `pub fn build_boolean_expression(`; `BOOLEAN_TRUTH_TABLE` → `pub const BOOLEAN_TRUTH_TABLE` |
| 8 | `src/expression.rs` | Runtime expression erasure and builtin catalog | `Expression` → `pub trait Expression`; `ExpressionError` → `pub enum ExpressionError`; `BinaryExpression` → `pub struct BinaryExpression`; `define_builtin_expressions` → `macro_rules! define_builtin_expressions`; `build_builtin_expression` → `pub fn build_builtin_expression(`; `BUILTIN_EXPRESSION_NAMES` → `pub const BUILTIN_EXPRESSION_NAMES` |
| 8 | `src/operators.rs` | Erased checked operator shells | `Expression for UnaryExpression` → `Expression for UnaryExpression`; `Expression for CheckedBinaryExpression` → `Expression for CheckedBinaryExpression`; `Expression for TernaryExpression` → `Expression for TernaryExpression` |
| 9 | `src/binder.rs` | Binder and runtime function registry | `BindError` → `pub enum BindError`; `BoundExpression` → `pub struct BoundExpression`; `FunctionRegistry` → `pub struct FunctionRegistry`; `FunctionRegistry::register` → `pub fn register(`; `FunctionRegistry::register_unary` → `pub fn register_unary(`; `FunctionRegistry::register_binary` → `pub fn register_binary(`; `FunctionRegistry::register_ternary` → `pub fn register_ternary(`; `FunctionRegistry::bind` → `pub fn bind(`; `bind_arithmetic` → `fn bind_arithmetic(`; `bind_comparison` → `fn bind_comparison(` |
| 9 | `src/operators.rs` | Logical comparison operators | `ComparisonOperator` → `pub enum ComparisonOperator` |
| 10 | `src/expression.rs` | Representative primitive fast loops | `PrimitiveLoop` → `pub enum PrimitiveLoop`; `PrimitiveBinaryExpression::evaluate_with_loop` → `pub fn evaluate_with_loop(` |
| 10 | `src/binder.rs` | Bound fast-loop forwarding | `BoundExpression::evaluate_with_loop` → `pub fn evaluate_with_loop(` |
| 11 | `src/array/list_array.rs` | One-level List scalars and arrays | `ListError` → `pub enum ListError`; `ListScalar` → `pub struct ListScalar`; `ListScalarRef` → `pub struct ListScalarRef`; `ListArray` → `pub struct ListArray`; `ListArrayBuilder` → `pub(crate) struct ListArrayBuilder`; `ListArray::try_from_rows` → `pub fn try_from_rows`; `ListArray::try_from_raw_parts` → `pub fn try_from_raw_parts(` |
| 11 | `src/data_type.rs` | Logical List type | `DataType::List` → `List(Box<DataType>)` |
| 11 | `src/physical_type.rs` | Physical List type | `PhysicalType::List` → `List(Box<PhysicalType>)` |
| 11 | `src/scalar.rs` | Owned and borrowed List scalar variants | `ScalarImpl::List` → `List(ListScalar)`; `ScalarRefImpl::List` → `List(ListScalarRef<'a>)` |
| 11 | `src/array.rs` | List array erasure | `From<ListArray> for ArrayImpl` → `impl From<ListArray> for ArrayImpl`; `TryFrom<ArrayImpl> for ListArray` → `impl TryFrom<ArrayImpl> for ListArray` |
| 11 | `src/column.rs` | List column downcast | `ListColumnView` → `pub struct ListColumnView`; `ColumnViewImpl::try_as_list` → `pub fn try_as_list(` |
| 12 | `src/array.rs` | Opaque array iteration | `Array::iter` → `fn iter<'a>(&'a self) -> impl Iterator` |
| 12 | `src/array/iterator.rs` | Private concrete array iterator | `ArrayIterator` → `pub struct ArrayIterator` |
| 12 | `src/expression.rs` | Thread-safe erased expression boundary | `Expression: Any + Send + Sync` → `pub trait Expression: Any + Send + Sync` |
| 12 | `src/binder.rs` | Shareable registry factories | `FunctionRegistry::register` → `pub fn register(`; `FunctionRegistry::register_unary` → `pub fn register_unary(`; `FunctionRegistry::register_binary` → `pub fn register_binary(`; `FunctionRegistry::register_ternary` → `pub fn register_ternary(` |
| 12 | `src/column.rs` | Borrowed column-view lifetime | `ColumnViewImpl<'a>` → `pub struct ColumnViewImpl<'a>` |
| 13 | `src/expression.rs` | Static and erased batch futures | `evaluate_static` → `pub fn evaluate_static`; `BatchFuture` → `pub type BatchFuture`; `AsyncExpression` → `pub trait AsyncExpression`; `AsyncExpressionAdapter` → `pub struct AsyncExpressionAdapter` |
| 13 | `src/binder.rs` | Bound asynchronous forwarding | `BoundExpression::evaluate_async` → `fn evaluate_async(` |

Day 5 introduces numeric comparisons beside arithmetic. Day 7 is the approved three-valued
Boolean checkpoint; the former Chapters 7–12 therefore appear here as Days 8–13.
