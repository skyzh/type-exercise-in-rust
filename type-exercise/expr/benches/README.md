# Chapter 7 benchmark method

This benchmark measures the maintained `type-exercise-expr` reference solution, not learner code in
`type-exercise-starter-expr`. Run it with:

```console
cargo bench -p type-exercise-expr --bench expression
```

Every case uses 65,536 deterministic rows and materializes an `I32Array`. The general and
primitive-specialized adapters receive the same borrowed column views. The dense handwritten
baseline receives preclassified slices or constant metadata outside timing, so it is a lower-bound
kernel rather than a peer adapter that performs representation selection. Fallback handwritten
cases use the general typed-view loop. The harness checks identical outputs before timing.

The four dense cases cover array/array, array/constant, constant/array, and constant/constant.
Three fallback cases cover a nullable array, a null constant, and an Indexed view. Its indices
are constructed and validated before the timed loop, so setup work is not mistaken for expression
evaluation. Criterion uses 20 samples, a 250 ms warm-up, and a 750 ms measurement period per
implementation. Results are machine-specific observations, not portable pass/fail thresholds.
