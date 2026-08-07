# Chapter 6 benchmark method

Run the maintained Criterion comparison with:

```console
cargo bench -p type-exercise --bench expression
```

Every case uses 65,536 deterministic rows and materializes an `I32Array`. The harness compares the
general adapter, the primitive-specialized adapter, and a direct handwritten loop over the same
borrowed column views. It checks identical outputs before timing.

The four dense cases cover array/array, array/constant, constant/array, and constant/constant.
Three fallback cases cover a nullable array, a null constant, and a dictionary. Dictionary keys
are constructed and validated before the timed loop, so setup work is not mistaken for expression
evaluation. Criterion uses 20 samples, a 250 ms warm-up, and a 750 ms measurement period per
implementation. Results are machine-specific observations, not portable pass/fail thresholds.
