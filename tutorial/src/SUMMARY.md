# Build a Database Expression Framework in Rust

[Preface](./preface.md)
[Environment Setup](./getting_started.md)

- [Part I: Start with a Scalar Evaluator](./volcano/overview.md)
  - [Values, Chunks, and Nulls](./volcano/chunk.md)
  - [Evaluate Expressions One Row at a Time](./volcano/expressions.md)
  - [Bind Logical Data Types](./volcano/data_types.md)
  - [Draw the Framework Boundary](./volcano/framework.md)

- [Part II: Build the Vectorized Runtime](./vectorized/overview.md)
  - [Arrow-like Arrays and Builders](./vectorized/array.md)
  - [Owned and Borrowed Scalars](./vectorized/scalar.md)
  - [Column Views: Arrays, Constants, and Dictionaries](./vectorized/column_view.md)
  - [Erase Physical Types at the Boundary](./vectorized/impls.md)
  - [Generate Vectorized Function Templates](./vectorized/func.md)
  - [Expand Numeric Families, Customize Everything Else](./vectorized/data_types.md)
  - [Bind and Execute the Complete Framework](./vectorized/framework.md)

[Benchmarks and Next Steps](./benchmarks.md)
[Appendix: Rust Language Concepts by Task](./appendix/rust_language.md)
