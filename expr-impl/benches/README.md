# Expression benchmarks

Run the release-mode benchmarks with:

```console
cargo bench -p expr-impl --bench expression
```

Each generated typed-kernel case has a hand-written, monomorphic loop over the same storage and
builder types. This is a more useful baseline than comparing different algorithms or memory
formats: the difference isolates the cost of type erasure and `ColumnView` dispatch. The array,
constant, and dictionary cases all materialize the same nullable Boolean output.

The `i32_non_null` group compares the general nullable template, primitive comparison and addition
fast paths, and hand-written loops over the same all-valid inputs. The fast and hand-written paths
do not read validity per row and initialize the all-valid output bitmap in bulk.

Treat a persistent regression greater than 15% for the regular array/array path as a reason to
inspect generated code before merging. Constant and dictionary views may have a larger gap because
their hand-written baselines know the encoding statically, but they should still avoid input
materialization and remain in the same order of magnitude.

Treat a persistent regression greater than 15% from either hand-written non-null baseline as a
reason to inspect whether the fast loop still vectorizes.
