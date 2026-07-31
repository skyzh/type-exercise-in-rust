# Build a Database Expression Framework in Rust

A database expression framework turns a logical operation such as `smallint + integer` into a
kernel that can evaluate nullable batches. The logical types are known while planning a query, but
the physical representation of each input may not be known until execution. One batch may contain
a regular array, a repeated constant, or dictionary-encoded values.

Over five days, you will extend a supplied Arrow-like array and expression workspace until it can
bind and run those expressions through one typed execution path:

```text
logical function and input types
              |
              v
       bound expression
              |
              v
array / constant / dictionary views
              |
              v
 generated or custom typed kernel
              |
              v
      materialized array
```

## Course Roadmap

| Day | Capability you add |
| --- | --- |
| 1 | Read arrays, constants, and dictionaries without materializing a common representation. |
| 2 | Reject invalid signatures during planning and select one concrete kernel. |
| 3 | Specialize all-valid primitive batches and measure the result against hand-written loops. |
| 4 | Strengthen the framework with opaque iterators, trait upcasting, variance checks, and explicit thread-safety contracts. |
| 5 | Add a batch-level asynchronous boundary without making row evaluation asynchronous. |

The repository before Day 1 is the full starter. It already provides arrays, owned and borrowed
scalars, runtime type-erasing enums, generated expression templates, comparison kernels, and
historical snapshots under `archive/`. Each day adds one implementation slice and the chapter that
explains its contract. If you are reviewing the Git history, the parent of the Day 1 change is the
starter state and each completed day is the starting state for the next.

## What You Need to Know

You should be comfortable with Rust enums, traits, associated types, lifetimes, iterators, and
declarative macros. Familiarity with SQL null semantics and columnar execution helps, but the
chapters introduce the database-specific behavior before the Rust machinery that implements it.

Each day ends with focused tests and a short reflection. Passing tests is required, but you should
also be able to explain the data flow, the central invariant, one failure case, and why the next day
can build on the resulting state.

## Design Rule

Use generic expansion for dense expression families such as numeric promotion and comparison.
Keep data-type-specific functions, such as string containment, as explicit kernels. The common
framework should connect both kinds of function without forcing every database operation through
one universal abstraction.

Continue to [Environment Setup](./setup.md).

{{#include copyright.md}}
