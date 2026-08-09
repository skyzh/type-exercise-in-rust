# Starter API Roadmap

This roadmap names the declaration surface each day asks you to implement. Days 1–2 are the only
Rust scaffolds present in this starter snapshot. Later entries are documentation, not hidden
implementations: their files and declarations appear only when that cumulative course day lands.
Every listed body remains learner work.

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
| 3 | `src/column.rs` | Array, constant, null, and indexed column views | `ColumnViewImpl` → `pub struct ColumnViewImpl`; `ColumnViewImpl::array` → `pub fn array(`; `ColumnViewImpl::constant` → `pub fn constant(`; `ColumnViewImpl::null` → `pub fn null(`; `ColumnViewImpl::dictionary` → `pub fn dictionary(`; `ColumnView::get` → `pub fn get(`; `ColumnView::len` → `pub fn len(` |
| 4 | `src/expression.rs` | Generic scalar functions and expression adapters | `BinaryScalarFunction` → `pub trait BinaryScalarFunction`; `I32Add` → `pub struct I32Add`; `evaluate_binary` → `pub fn evaluate_binary(`; `UnaryExpression` → `pub struct UnaryExpression`; `BinaryExpression` → `pub struct BinaryExpression`; `Expression` → `pub trait Expression` |
| 5 | `src/promotion.rs` | Lossless numeric promotion | `NumericPromotion` → `pub struct NumericPromotion`; `NUMERIC_PROMOTIONS` → `pub const NUMERIC_PROMOTIONS`; `promote_numeric` → `pub fn promote_numeric(` |
| 5 | `src/operators.rs` | Arithmetic and numeric comparison kernels | `ArithmeticOperator` → `pub enum ArithmeticOperator`; `ComparisonOperator` → `pub enum ComparisonOperator`; `build_numeric_binary_expression` → `fn build_numeric_binary_expression(`; `build_numeric_comparison_expression` → `fn build_numeric_comparison_expression(` |
| 6 | `src/operators.rs` | Systematic unary, binary, and ternary arity | `CheckedUnaryScalarFunction` → `pub trait CheckedUnaryScalarFunction`; `CheckedBinaryScalarFunction` → `pub trait CheckedBinaryScalarFunction`; `CheckedTernaryScalarFunction` → `pub trait CheckedTernaryScalarFunction`; `TernaryExpression` → `pub struct TernaryExpression`; `ExpressionError` → `pub enum ExpressionError` |
| 7 | `src/boolean_logic.rs` | Three-valued Boolean logic | `NullEvaluationPolicy` → `pub enum NullEvaluationPolicy`; `BooleanOperator` → `pub enum BooleanOperator`; `build_boolean_expression` → `pub fn build_boolean_expression(`; `BOOLEAN_TRUTH_TABLE` → `pub const BOOLEAN_TRUTH_TABLE` |
| 8 | `src/expression.rs` | Runtime expression erasure and builtin catalog | `Expression` → `pub trait Expression`; `UnaryExpression` → `pub struct UnaryExpression`; `CheckedBinaryExpression` → `pub struct CheckedBinaryExpression`; `TernaryExpression` → `pub struct TernaryExpression`; `builtin_function_catalog` → `macro_rules! builtin_function_catalog` |
| 9 | `src/binder.rs` | Binder and runtime function registry | `BindError` → `pub enum BindError`; `BoundExpression` → `pub struct BoundExpression`; `FunctionRegistry` → `pub struct FunctionRegistry`; `FunctionRegistry::register` → `pub fn register(`; `FunctionRegistry::bind` → `pub fn bind(` |
| 10 | `src/expression.rs` | Representative primitive fast loops | `PrimitiveLoop` → `pub enum PrimitiveLoop`; `PrimitiveBinaryExpression::evaluate_with_loop` → `pub fn evaluate_with_loop(`; `BoundExpression::evaluate_with_loop` → `pub fn evaluate_with_loop(` |
| 11 | `src/array/list_array.rs` | One-level List scalars and arrays | `ListScalar` → `pub struct ListScalar`; `ListScalarRef` → `pub struct ListScalarRef`; `ListArray` → `pub struct ListArray`; `ListArray::try_from_raw_parts` → `pub fn try_from_raw_parts(`; `ListArray::try_from_rows` → `pub fn try_from_rows`; `ListArrayBuilder` → `pub struct ListArrayBuilder` |
| 11 | `src/column.rs` | List column downcast | `ListColumnView` → `pub struct ListColumnView`; `ColumnViewImpl::try_as_list` → `pub fn try_as_list(` |
| 12 | `src/array/iterator.rs` | Iterator and Rust trait boundaries | `ArrayIterator` → `pub struct ArrayIterator` |
| 12 | `src/expression.rs` | Thread-safe erased expression boundary | `Expression: Any + Send + Sync` → `pub trait Expression: Any + Send + Sync` |
| 13 | `src/expression.rs` | Batch-level asynchronous adapter | `evaluate_static` → `pub fn evaluate_static(`; `BatchFuture` → `pub type BatchFuture`; `AsyncExpression` → `pub trait AsyncExpression`; `AsyncExpressionAdapter` → `pub struct AsyncExpressionAdapter`; `BoundExpression::evaluate_async` → `pub fn evaluate_async(` |

The Day 5 roadmap already includes numeric comparisons beside arithmetic. Day 7 is the approved
three-valued Boolean checkpoint; the former Days 7–12 therefore appear here as Days 8–13.
