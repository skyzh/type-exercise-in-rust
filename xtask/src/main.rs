use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Debug, Subcommand)]
enum Action {
    /// Copy cumulative tests through one chapter into the starter.
    CopyTest {
        #[arg(long)]
        chapter: usize,
    },
    /// Verify that course chapters, supplied tests, navigation, and CI stay synchronized.
    CheckCourse,
}

const COURSE_CHAPTERS: usize = 12;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Fingerprint {
    bytes: usize,
    fnv1a64: u64,
}

#[derive(Clone, Copy, Debug)]
struct ChapterContract {
    number: usize,
    page: &'static str,
    title: &'static str,
    expected_red: &'static str,
    checkpoint_targets: &'static [&'static str],
    boundary: &'static str,
    transition: &'static str,
    page_fingerprint: Fingerprint,
    test_fingerprint: Fingerprint,
}

const README_FINGERPRINT: Fingerprint = Fingerprint {
    bytes: 3197,
    fnv1a64: 0x7a0a_ce1c_6744_2d89,
};
const SVG_FINGERPRINT: Fingerprint = Fingerprint {
    bytes: 5982,
    fnv1a64: 0x8dfc_2ae6_1fea_6033,
};
const CI_FINGERPRINT: Fingerprint = Fingerprint {
    bytes: 3549,
    fnv1a64: 0x0a44_7a46_db96_d593,
};
const CARGO_CONFIG_FINGERPRINT: Fingerprint = Fingerprint {
    bytes: 98,
    fnv1a64: 0xae57_ed5a_ab2b_148a,
};
const XTASK_MANIFEST_FINGERPRINT: Fingerprint = Fingerprint {
    bytes: 336,
    fnv1a64: 0x9bb5_84b9_23f6_3f8b,
};
const TEST_REGISTRY_FINGERPRINT: Fingerprint = Fingerprint {
    bytes: 183,
    fnv1a64: 0xc950_20cd_3bc0_2a4c,
};
const PREFACE_FINGERPRINT: Fingerprint = Fingerprint {
    bytes: 2146,
    fnv1a64: 0x785d_ceee_0904_15cc,
};
const SETUP_FINGERPRINT: Fingerprint = Fingerprint {
    bytes: 1857,
    fnv1a64: 0x7ac4_8706_c71f_d7bf,
};
const FOOTER_FINGERPRINT: Fingerprint = Fingerprint {
    bytes: 316,
    fnv1a64: 0xf365_baba_94a8_0018,
};
const SUMMARY_FINGERPRINT: Fingerprint = Fingerprint {
    bytes: 869,
    fnv1a64: 0x436f_0bca_5e08_0197,
};
const SITEMAP_TEXT_FINGERPRINT: Fingerprint = Fingerprint {
    bytes: 1001,
    fnv1a64: 0xa541_8a74_77b9_48bd,
};
const SITEMAP_XML_FINGERPRINT: Fingerprint = Fingerprint {
    bytes: 1711,
    fnv1a64: 0x3c6a_aa25_2d72_83b8,
};
const SOCIAL_IMAGE_FINGERPRINT: Fingerprint = Fingerprint {
    bytes: 1_043_209,
    fnv1a64: 0x87c2_ac72_f34f_d91d,
};
const BOOK_CONFIG_FINGERPRINT: Fingerprint = Fingerprint {
    bytes: 305,
    fnv1a64: 0x114c_8219_be8f_6b55,
};
const THEME_HEAD_FINGERPRINT: Fingerprint = Fingerprint {
    bytes: 1537,
    fnv1a64: 0x52a0_5948_1649_ad25,
};
const ROOT_MANIFEST_FINGERPRINT: Fingerprint = Fingerprint {
    bytes: 251,
    fnv1a64: 0x37ba_3635_943d_1f35,
};
const REFERENCE_MANIFEST_FINGERPRINT: Fingerprint = Fingerprint {
    bytes: 295,
    fnv1a64: 0xea4d_c3c4_3be5_5080,
};

const SUPPLIED_TEST_COUNTS: [usize; COURSE_CHAPTERS] = [4, 3, 5, 8, 5, 2, 7, 12, 7, 9, 6, 6];

const COURSE_SOURCE_FILES: [&str; 20] = [
    "SUMMARY.md",
    "assets/map-of-types.svg",
    "chapter-1-type-family.md",
    "chapter-10-list.md",
    "chapter-11-rust-boundaries.md",
    "chapter-12-async-boundary.md",
    "chapter-2-type-catalog.md",
    "chapter-3-column-views.md",
    "chapter-4-concrete-loops.md",
    "chapter-5-generic-arithmetic.md",
    "chapter-6-systematic-arity.md",
    "chapter-7-runtime-erasure.md",
    "chapter-8-binding-coercion.md",
    "chapter-9-primitive-loops.md",
    "copyright.md",
    "preface.md",
    "setup.md",
    "sitemap.txt",
    "sitemap.xml",
    "type-exercise-social.png",
];

const SUPPLIED_TEST_FILES: [&str; COURSE_CHAPTERS] = [
    "chapter_1.rs",
    "chapter_10.rs",
    "chapter_11.rs",
    "chapter_12.rs",
    "chapter_2.rs",
    "chapter_3.rs",
    "chapter_4.rs",
    "chapter_5.rs",
    "chapter_6.rs",
    "chapter_7.rs",
    "chapter_8.rs",
    "chapter_9.rs",
];

const THEME_FILES: [&str; 1] = ["head.hbs"];
const INTEGRATION_TEST_FILES: [&str; 0] = [];

const TEST_REGISTRY_LINES: [&str; COURSE_CHAPTERS] = [
    "mod chapter_1;",
    "mod chapter_10;",
    "mod chapter_11;",
    "mod chapter_12;",
    "mod chapter_2;",
    "mod chapter_3;",
    "mod chapter_4;",
    "mod chapter_5;",
    "mod chapter_6;",
    "mod chapter_7;",
    "mod chapter_8;",
    "mod chapter_9;",
];

const SITEMAP_URLS: [&str; 15] = [
    "https://skyzh.github.io/type-exercise-in-rust",
    "https://skyzh.github.io/type-exercise-in-rust/chapter-1-type-family",
    "https://skyzh.github.io/type-exercise-in-rust/chapter-10-list",
    "https://skyzh.github.io/type-exercise-in-rust/chapter-11-rust-boundaries",
    "https://skyzh.github.io/type-exercise-in-rust/chapter-12-async-boundary",
    "https://skyzh.github.io/type-exercise-in-rust/chapter-2-type-catalog",
    "https://skyzh.github.io/type-exercise-in-rust/chapter-3-column-views",
    "https://skyzh.github.io/type-exercise-in-rust/chapter-4-concrete-loops",
    "https://skyzh.github.io/type-exercise-in-rust/chapter-5-generic-arithmetic",
    "https://skyzh.github.io/type-exercise-in-rust/chapter-6-systematic-arity",
    "https://skyzh.github.io/type-exercise-in-rust/chapter-7-runtime-erasure",
    "https://skyzh.github.io/type-exercise-in-rust/chapter-8-binding-coercion",
    "https://skyzh.github.io/type-exercise-in-rust/chapter-9-primitive-loops",
    "https://skyzh.github.io/type-exercise-in-rust/preface",
    "https://skyzh.github.io/type-exercise-in-rust/setup",
];

const CHAPTERS: [ChapterContract; COURSE_CHAPTERS] = [
    ChapterContract {
        number: 1,
        page: "chapter-1-type-family.md",
        title: "Connect One Type Family by Hand",
        expected_red: "The untouched starter should fail because it has only `ScalarImpl`. Do not edit the copied test.",
        checkpoint_targets: &[
            "**Target:** `type-exercise-starter/src/physical_type.rs::{PhysicalType, TypeMismatch}` and `type-exercise-starter/src/scalar.rs::{Scalar, ScalarRef, ScalarImpl, ScalarRefImpl}`.",
            "**Target:** `type-exercise-starter/src/array.rs::{Array, ArrayBuilder, ArrayImpl}`, `type-exercise-starter/src/array/primitive_array.rs::{PrimitiveArray, PrimitiveArrayBuilder}`, and `type-exercise-starter/src/array/string_array.rs::{StringArray, StringArrayBuilder}`.",
            "**Target:** `From`/`TryFrom` implementations in `type-exercise-starter/src/scalar.rs` and `type-exercise-starter/src/array.rs`, plus exports in `type-exercise-starter/src/lib.rs`.",
        ],
        boundary: "Required work is exactly the explicit `i32` and `String` rows. Additional physical types, macros, columns, and expressions belong to later chapters. As an extension, sketch a third family on paper and mark every enum arm and conversion it would require; do not implement it yet.",
        transition: "Next: [Chapter 2 scales the family without copying every connection](./chapter-2-type-catalog.md).",
        page_fingerprint: Fingerprint {
            bytes: 4196,
            fnv1a64: 0x8bb8_885b_7ca1_a4b2,
        },
        test_fingerprint: Fingerprint {
            bytes: 3362,
            fnv1a64: 0x6129_d79a_91d5_45cb,
        },
    },
    ChapterContract {
        number: 2,
        page: "chapter-2-type-catalog.md",
        title: "Scale the Physical Type Family",
        expected_red: "The first run should fail on the new families and catalog.",
        checkpoint_targets: &[
            "**Target:** `type-exercise-starter/src/array/primitive_array.rs::{PrimitiveArray, PrimitiveArrayBuilder}`.",
            "**Target:** `type-exercise-starter/src/variant_catalog.rs::for_each_physical_family`, plus generated arms in `type-exercise-starter/src/physical_type.rs`, `type-exercise-starter/src/scalar.rs`, and `type-exercise-starter/src/array.rs`.",
            "**Target:** `type-exercise-starter/src/data_type.rs::{DataType, DataType::physical_type}`.",
        ],
        boundary: "All table rows are required. Decimal arithmetic and precision enforcement are not. Extending the catalog with another physical family is useful practice, but it must bring the complete scalar, array, erasure, and mismatch surface rather than one enum variant.",
        transition: "Next: [Chapter 3 reads several nullable column encodings](./chapter-3-column-views.md).",
        page_fingerprint: Fingerprint {
            bytes: 3819,
            fnv1a64: 0xd4a1_5f9c_7513_e168,
        },
        test_fingerprint: Fingerprint {
            bytes: 3786,
            fnv1a64: 0x49ac_2da9_0521_a8d7,
        },
    },
    ChapterContract {
        number: 3,
        page: "chapter-3-column-views.md",
        title: "Read Nullable Columns Without Materializing Them",
        expected_red: "The first run should fail on the missing column constructors and typed view.",
        checkpoint_targets: &[
            "**Target:** `type-exercise-starter/src/column.rs::ColumnViewImpl::{array, constant, null, dictionary}`.",
            "**Target:** `type-exercise-starter/src/column.rs::{ColumnView, ColumnView::get, ColumnView::len}` and `TryFrom<ColumnViewImpl>`.",
        ],
        boundary: "The four representations and fail-closed dictionary constructor are required. Run-length encoding and nested columns are extensions. List will reuse the same representation boundary in Chapter 10.",
        transition: "Next: [Chapter 4 exposes what unary and binary loops repeat](./chapter-4-concrete-loops.md).",
        page_fingerprint: Fingerprint {
            bytes: 3378,
            fnv1a64: 0xf92e_3658_f689_c25b,
        },
        test_fingerprint: Fingerprint {
            bytes: 4720,
            fnv1a64: 0x3df2_9b70_2959_1a40,
        },
    },
    ChapterContract {
        number: 4,
        page: "chapter-4-concrete-loops.md",
        title: "Expose the Cost of Concrete Loops",
        expected_red: "The first run should fail on the missing scalar-function traits, adapters, and batch evaluator.",
        checkpoint_targets: &[
            "**Target:** `type-exercise-starter/src/expression.rs::{BinaryScalarFunction, I32Add, evaluate_binary}` for the initial binary path; `type-exercise-starter/src/operators.rs::{CheckedUnaryScalarFunction, CheckedBinaryScalarFunction}` for the two checked arities.",
            "**Target:** `type-exercise-starter/src/operators.rs::{UnaryExpression, CheckedBinaryExpression}` and their `Expression::evaluate` implementations.",
        ],
        boundary: "One checked unary shell and one checked binary shell are required. More operations, runtime catalogs, and ternary evaluation are later work. Do not create a generic N-ary vector of erased values as an extension; it would throw away the typed family you just built.",
        transition: "Next: [Chapter 5 makes numeric operation selection generic](./chapter-5-generic-arithmetic.md).",
        page_fingerprint: Fingerprint {
            bytes: 3131,
            fnv1a64: 0x0256_148e_9392_b936,
        },
        test_fingerprint: Fingerprint {
            bytes: 6233,
            fnv1a64: 0x3afa_fdbe_9188_6468,
        },
    },
    ChapterContract {
        number: 5,
        page: "chapter-5-generic-arithmetic.md",
        title: "Make Numeric Evaluation Generic",
        expected_red: "The first run should fail on the promotion table and generic arithmetic catalog.",
        checkpoint_targets: &[
            "**Target:** `type-exercise-starter/src/promotion.rs::{NumericPromotion, NUMERIC_PROMOTIONS, promote_numeric}`.",
            "**Target:** `type-exercise-starter/src/operators.rs::{ArithmeticOperator, CheckedBinaryExpression, build_numeric_binary_expression}`.",
        ],
        boundary: "The pinned matrix and all four operations are required. Decimal arithmetic, narrowing casts, and precision-losing implicit casts are extensions only after their semantics are specified. Do not broaden the table merely to make more Rust conversions compile.",
        transition: "Next: [Chapter 6 proves that arity is systematic with a real ternary function](./chapter-6-systematic-arity.md).",
        page_fingerprint: Fingerprint {
            bytes: 3325,
            fnv1a64: 0xf2ac_b76f_711b_454b,
        },
        test_fingerprint: Fingerprint {
            bytes: 11604,
            fnv1a64: 0x4ac3_d2ce_345d_4eab,
        },
    },
    ChapterContract {
        number: 6,
        page: "chapter-6-systematic-arity.md",
        title: "Make Arity Systematic",
        expected_red: "The first run should fail on shared validation or the unary and ternary physical paths.",
        checkpoint_targets: &[
            "**Target:** `type-exercise-starter/src/operators.rs::validate_expression_inputs` and `type-exercise-starter/src/expression.rs::ExpressionError`.",
            "**Target:** `type-exercise-starter/src/operators.rs::{CheckedTernaryScalarFunction, TernaryExpression}`.",
            "**Target:** `type-exercise-starter/src/operators.rs::build_numeric_clamp_expression`.",
        ],
        boundary: "Unary, binary, real ternary, and generic validation for longer slices are required. Logical registration waits until Chapter 8. Concrete four- and five-input builtins are extensions. The archived source-tree-writing generator is not a course goal; if you generate boilerplate, use a source-controlled declarative macro.",
        transition: "Next: [Chapter 7 hides the typed shells behind one runtime interface](./chapter-7-runtime-erasure.md).",
        page_fingerprint: Fingerprint {
            bytes: 3265,
            fnv1a64: 0x0ffb_4d2e_df29_7e4a,
        },
        test_fingerprint: Fingerprint {
            bytes: 3716,
            fnv1a64: 0xf6ec_5379_22c1_1098,
        },
    },
    ChapterContract {
        number: 7,
        page: "chapter-7-runtime-erasure.md",
        title: "Erase Typed Expressions at Runtime",
        expected_red: "The first run should fail on the object-safe expression boundary or physical catalog.",
        checkpoint_targets: &[
            "**Target:** `type-exercise-starter/src/expression.rs::{Expression, ExpressionError}`.",
            "**Target:** `Expression` implementations for `UnaryExpression`, `CheckedBinaryExpression`, and `TernaryExpression` in `type-exercise-starter/src/operators.rs`, plus the original `BinaryExpression` compatibility implementation in `type-exercise-starter/src/expression.rs`.",
            "**Target:** `type-exercise-starter/src/expression.rs::{define_builtin_expressions, build_builtin_expression, BUILTIN_EXPRESSION_NAMES}`.",
        ],
        boundary: "Checked runtime erasure and a complete physical catalog are required. The concrete shells from Chapters 4–6 keep the same batch behavior; this chapter changes how the engine selects them. Dynamic plugin loading and per-row erased dispatch are extensions outside this course.",
        transition: "Next: [Chapter 8 binds logical calls and applies the promotion contract](./chapter-8-binding-coercion.md).",
        page_fingerprint: Fingerprint {
            bytes: 3310,
            fnv1a64: 0x29cd_3084_6798_179f,
        },
        test_fingerprint: Fingerprint {
            bytes: 6250,
            fnv1a64: 0x85e1_eb01_982b_8198,
        },
    },
    ChapterContract {
        number: 8,
        page: "chapter-8-binding-coercion.md",
        title: "Bind and Coerce Logical Calls",
        expected_red: "The first run should fail on slice-based binding, comparison semantics, or `contains`.",
        checkpoint_targets: &[
            "**Target:** `type-exercise-starter/src/binder.rs::{BindError, BoundExpression, FunctionRegistry::register, register_unary, register_binary, register_ternary, bind}`.",
            "**Target:** `type-exercise-starter/src/operators.rs::{ComparisonOperator}` and binder factories registered by `FunctionRegistry::with_builtins`.",
            "**Target:** `type-exercise-starter/src/promotion.rs::promote_numeric` and its callers in `type-exercise-starter/src/binder.rs::{bind_arithmetic, bind_comparison}`.",
        ],
        boundary: "Slice-based binding, lossless promotion, six comparisons, `contains`, and `concat` are required. Narrowing casts, parsing casts, SQL-complete coercion, Decimal arithmetic, and overload selection from untyped `NULL` are extensions that need separate semantics.",
        transition: "Next: [Chapter 9 specializes one representative dense loop](./chapter-9-primitive-loops.md).",
        page_fingerprint: Fingerprint {
            bytes: 4004,
            fnv1a64: 0x8341_989b_0995_1f16,
        },
        test_fingerprint: Fingerprint {
            bytes: 12440,
            fnv1a64: 0xb437_2325_b934_8dba,
        },
    },
    ChapterContract {
        number: 9,
        page: "chapter-9-primitive-loops.md",
        title: "Specialize One Primitive Loop",
        expected_red: "The first run should fail on fast-path selection while the earlier general evaluator stays green.",
        checkpoint_targets: &[
            "**Target:** `type-exercise-starter/src/expression.rs::{PrimitiveLoop, PrimitiveBinaryExpression::evaluate_with_loop}`.",
            "**Target:** `type-exercise-starter/src/binder.rs::BoundExpression::evaluate_with_loop`.",
        ],
        boundary: "Representative `i32` specialization and semantic fallbacks are required. Fast paths for every numeric family and operator are extensions. Do not duplicate the full evaluator to chase a benchmark.",
        transition: "Next: [Chapter 10 adds one-level List storage](./chapter-10-list.md).",
        page_fingerprint: Fingerprint {
            bytes: 2894,
            fnv1a64: 0x52ab_ded8_924c_d850,
        },
        test_fingerprint: Fingerprint {
            bytes: 7108,
            fnv1a64: 0x34c2_6c50_444e_5985,
        },
    },
    ChapterContract {
        number: 10,
        page: "chapter-10-list.md",
        title: "Build a One-Level List Column",
        expected_red: "The first run should fail on the missing List types, invariants, and column integration.",
        checkpoint_targets: &[
            "**Target:** `type-exercise-starter/src/array/list_array.rs::{ListScalar, ListScalarRef, ListError}` and List variants in `type-exercise-starter/src/{data_type,physical_type,scalar}.rs`.",
            "**Target:** `type-exercise-starter/src/array/list_array.rs::{ListArray, ListArrayBuilder, try_from_rows, try_from_raw_parts}`.",
            "**Target:** `type-exercise-starter/src/column.rs::{ListColumnView, ColumnViewImpl::try_as_list}` and List erasure in `type-exercise-starter/src/array.rs`.",
        ],
        boundary: "One-level storage and List inputs are required. Nested Lists, List equality as a scalar builtin, list-producing functions, and arbitrary List casts are extensions. The public type descriptor can represent a nested shape, but construction must reject it until those contracts exist.",
        transition: "Next: [Chapter 11 turns Rust boundary claims into executable checks](./chapter-11-rust-boundaries.md).",
        page_fingerprint: Fingerprint {
            bytes: 3310,
            fnv1a64: 0x5fd8_e07e_bafb_6c10,
        },
        test_fingerprint: Fingerprint {
            bytes: 8227,
            fnv1a64: 0xddd6_33e4_662f_8063,
        },
    },
    ChapterContract {
        number: 11,
        page: "chapter-11-rust-boundaries.md",
        title: "Strengthen Rust Type Boundaries",
        expected_red: "The first run should fail on one or more iterator, trait-object, thread, or lifetime guarantees.",
        checkpoint_targets: &[
            "**Target:** `type-exercise-starter/src/array.rs::Array::iter` and `type-exercise-starter/src/array/iterator.rs::ArrayIterator`.",
            "**Target:** `type-exercise-starter/src/expression.rs::Expression: Any + Send + Sync`.",
            "**Target:** `type-exercise-starter/src/binder.rs::FunctionRegistry::{register, register_unary, register_binary, register_ternary}` and `type-exercise-starter/src/column.rs::ColumnViewImpl<'a>`.",
        ],
        boundary: "Opaque iteration, checked recovery, thread-safety, and covariance are required. Unsafe downcasts, custom executors, and arbitrary lifetime conversion helpers are not.",
        transition: "Next: [Chapter 12 adds one future around a whole batch](./chapter-12-async-boundary.md).",
        page_fingerprint: Fingerprint {
            bytes: 2765,
            fnv1a64: 0x62c9_c19a_7961_9dfd,
        },
        test_fingerprint: Fingerprint {
            bytes: 3826,
            fnv1a64: 0xff1d_aeb7_b023_70a4,
        },
    },
    ChapterContract {
        number: 12,
        page: "chapter-12-async-boundary.md",
        title: "Add a Batch Async Boundary",
        expected_red: "The first run should fail on the missing static or erased batch future boundary.",
        checkpoint_targets: &[
            "**Target:** `type-exercise-starter/src/expression.rs::evaluate_static`.",
            "**Target:** `type-exercise-starter/src/expression.rs::{BatchFuture, AsyncExpression, AsyncExpressionAdapter}`.",
            "**Target:** `type-exercise-starter/src/binder.rs::BoundExpression::evaluate_async`.",
        ],
        boundary: "One ready future per batch is required. I/O, timers, retries, background threads, cancellation protocols, custom runtimes, and per-row futures are outside this course.",
        transition: "You have now moved type selection, representation dispatch, validation, promotion, and runtime selection out of the row loop while keeping each failure boundary explicit.",
        page_fingerprint: Fingerprint {
            bytes: 2664,
            fnv1a64: 0x8884_b734_0692_09e7,
        },
        test_fingerprint: Fingerprint {
            bytes: 6719,
            fnv1a64: 0x9229_1311_4714_0cb2,
        },
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CopyReport {
    changed_files: usize,
}

fn workspace_root() -> Result<PathBuf> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .context("failed to find the workspace root")
}

fn chapter_number(path: &Path) -> Option<usize> {
    let name = path.file_name()?.to_str()?;
    let number = name.strip_prefix("chapter_")?.strip_suffix(".rs")?;
    if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    number.parse().ok()
}

fn available_chapters(source_dir: &Path) -> Result<Vec<usize>> {
    let mut chapters = fs::read_dir(source_dir)
        .with_context(|| format!("failed to list {}", source_dir.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .filter_map(|path| chapter_number(&path))
        .collect::<Vec<_>>();
    chapters.sort_unstable();

    if chapters.is_empty() {
        bail!("no chapter tests are available");
    }
    for (index, chapter) in chapters.iter().copied().enumerate() {
        let expected = index + 1;
        if chapter != expected {
            bail!(
                "chapter test sequence is not contiguous: expected chapter {expected}, found chapter {chapter}"
            );
        }
    }
    Ok(chapters)
}

fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<bool> {
    if fs::read(path).ok().as_deref() == Some(bytes) {
        return Ok(false);
    }
    fs::write(path, bytes).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(true)
}

fn copy_test(root: &Path, chapter: usize) -> Result<CopyReport> {
    let source_dir = root.join("type-exercise/src/tests");
    let available = available_chapters(&source_dir)?;
    let Some(last_chapter) = available.last().copied() else {
        bail!("no chapter tests are available");
    };
    if chapter == 0 || chapter > last_chapter {
        bail!("no tests are available for chapter {chapter}");
    }

    // Read and validate the complete cumulative source set before mutating the starter.
    let sources = (1..=chapter)
        .map(|number| {
            let name = format!("chapter_{number}.rs");
            let path = source_dir.join(&name);
            let bytes = fs::read(&path)
                .with_context(|| format!("failed to read cumulative source {}", path.display()))?;
            Ok((name, bytes))
        })
        .collect::<Result<Vec<_>>>()?;

    let target_dir = root.join("type-exercise-starter/src/tests");
    fs::create_dir_all(&target_dir).context("failed to create the starter test directory")?;
    let mut changed_files = 0;
    for (name, bytes) in &sources {
        changed_files += usize::from(write_if_changed(&target_dir.join(name), bytes)?);
    }

    for entry in fs::read_dir(&target_dir).context("failed to list copied starter tests")? {
        let path = entry?.path();
        let Some(number) = chapter_number(&path) else {
            continue;
        };
        if number > chapter {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove stale {}", path.display()))?;
            changed_files += 1;
        }
    }

    let mut test_module = String::new();
    writeln!(
        test_module,
        "//! DO NOT MODIFY -- copied course test modules"
    )?;
    writeln!(
        test_module,
        "//! This file is rewritten by `cargo x copy-test`."
    )?;
    for number in 1..=chapter {
        writeln!(test_module, "mod chapter_{number};")?;
    }
    changed_files += usize::from(write_if_changed(
        &root.join("type-exercise-starter/src/tests.rs"),
        test_module.as_bytes(),
    )?);

    for (name, source_bytes) in sources {
        let target = target_dir.join(&name);
        let target_bytes = fs::read(&target)
            .with_context(|| format!("failed to verify copied target {}", target.display()))?;
        if target_bytes != source_bytes {
            bail!("copied test is not byte-identical: {name}");
        }
    }

    println!("copied cumulative Chapters 1-{chapter} tests into type-exercise-starter");
    Ok(CopyReport { changed_files })
}

fn course_chapter_number(path: &Path) -> Option<usize> {
    let name = path.file_name()?.to_str()?;
    let suffix = name.strip_prefix("chapter-")?;
    let (number, _) = suffix.split_once('-')?;
    number.parse().ok()
}

fn fingerprint(bytes: &[u8]) -> Fingerprint {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    Fingerprint {
        bytes: bytes.len(),
        fnv1a64: hash,
    }
}

fn normalized(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn require_once(haystack: &str, marker: &str, label: &str) -> Result<()> {
    let count = haystack.matches(marker).count();
    if count != 1 {
        bail!("{label} must appear exactly once, found {count}");
    }
    Ok(())
}

fn require_once_normalized(haystack: &str, marker: &str, label: &str) -> Result<()> {
    require_once(&normalized(haystack), &normalized(marker), label)
}

fn require_fingerprint(bytes: &[u8], expected: Fingerprint, label: &str) -> Result<()> {
    let actual = fingerprint(bytes);
    if actual != expected {
        bail!(
            "{label} changed outside the structured course contract: expected {expected:?}, found {actual:?}"
        );
    }
    Ok(())
}

fn require_ordered_markers(haystack: &str, markers: &[String], label: &str) -> Result<()> {
    let mut offset = 0;
    for marker in markers {
        let Some(relative) = haystack[offset..].find(marker) else {
            bail!("{label} is missing ordered marker {marker:?}");
        };
        offset += relative + marker.len();
    }
    Ok(())
}

fn relative_regular_files(root: &Path) -> Result<Vec<String>> {
    fn walk(root: &Path, directory: &Path, files: &mut Vec<String>) -> Result<()> {
        for entry in fs::read_dir(directory)
            .with_context(|| format!("failed to list {}", directory.display()))?
        {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let path = entry.path();
            if file_type.is_symlink() {
                bail!("publishable source cannot be a symlink: {}", path.display());
            }
            if file_type.is_dir() {
                walk(root, &path, files)?;
            } else if file_type.is_file() {
                files.push(
                    path.strip_prefix(root)
                        .context("failed to make publication path relative")?
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    walk(root, root, &mut files)?;
    files.sort();
    Ok(files)
}

fn relative_regular_files_if_present(root: &Path) -> Result<Vec<String>> {
    if !root
        .try_exists()
        .with_context(|| format!("failed to inspect {}", root.display()))?
    {
        return Ok(Vec::new());
    }
    relative_regular_files(root)
}

fn sitemap_xml_locations(xml: &str) -> Result<Vec<&str>> {
    let mut remaining = xml;
    let mut locations = Vec::new();
    while let Some((_, after_open)) = remaining.split_once("<loc>") {
        let (location, after_close) = after_open
            .split_once("</loc>")
            .context("sitemap.xml contains an unclosed <loc>")?;
        locations.push(location.trim());
        remaining = after_close;
    }
    Ok(locations)
}

fn check_course_contract(root: &Path) -> Result<()> {
    let cargo_config_bytes =
        fs::read(root.join(".cargo/config.toml")).context("failed to read Cargo configuration")?;
    let cargo_config = std::str::from_utf8(&cargo_config_bytes)
        .context("Cargo configuration is not valid UTF-8")?;
    for fact in [
        "[alias]",
        "x = \"run --package type-exercise-xtask --\"",
        "xtask = \"run --package type-exercise-xtask --\"",
    ] {
        require_once_normalized(cargo_config, fact, "Cargo command bootstrap")?;
    }
    require_fingerprint(
        &cargo_config_bytes,
        CARGO_CONFIG_FINGERPRINT,
        "complete Cargo command bootstrap",
    )?;

    let xtask_manifest_bytes =
        fs::read(root.join("xtask/Cargo.toml")).context("failed to read the xtask manifest")?;
    let xtask_manifest = std::str::from_utf8(&xtask_manifest_bytes)
        .context("xtask Cargo manifest is not valid UTF-8")?;
    for fact in [
        "[[bin]] name = \"type-exercise-xtask\" path = \"src/main.rs\" test = true bench = false",
        "anyhow = \"1\"",
        "clap = { version = \"4\", features = [\"derive\"] }",
        "tempfile = \"3\"",
    ] {
        require_once_normalized(xtask_manifest, fact, "xtask execution bootstrap")?;
    }
    require_fingerprint(
        &xtask_manifest_bytes,
        XTASK_MANIFEST_FINGERPRINT,
        "complete xtask manifest",
    )?;

    let book_config_path = root.join("course/book.toml");
    let book_config_bytes =
        fs::read(&book_config_path).context("failed to read course/book.toml")?;
    let book_config =
        std::str::from_utf8(&book_config_bytes).context("course/book.toml is not valid UTF-8")?;
    for fact in [
        "authors = [\"Alex Chi\"]",
        "description = \"A twelve-chapter Rust course that builds a typed vectorized database expression engine\"",
        "src = \"src\"",
        "title = \"Build a Typed Database Expression Engine in Rust\"",
        "git-repository-url = \"https://github.com/skyzh/type-exercise-in-rust\"",
    ] {
        require_once_normalized(book_config, fact, "mdBook publication configuration")?;
    }
    require_fingerprint(
        &book_config_bytes,
        BOOK_CONFIG_FINGERPRINT,
        "mdBook publication configuration",
    )?;

    let theme_dir = root.join("course/theme");
    let theme_files = relative_regular_files(&theme_dir)?;
    if theme_files != THEME_FILES {
        bail!(
            "mdBook theme file set must match the exact allowlist: expected {:?}, found {theme_files:?}",
            THEME_FILES
        );
    }
    let theme_head_bytes =
        fs::read(theme_dir.join("head.hbs")).context("failed to read course/theme/head.hbs")?;
    let theme_head =
        std::str::from_utf8(&theme_head_bytes).context("theme head is not valid UTF-8")?;
    for fact in [
        "<meta name=\"author\" content=\"Alex Chi Z\">",
        "<meta property=\"og:site_name\" content=\"Type Exercise in Rust\">",
        "<meta property=\"og:image:width\" content=\"1200\">",
        "<meta property=\"og:image:height\" content=\"628\">",
        "<meta name=\"twitter:creator\" content=\"@iskyzh\">",
    ] {
        require_once_normalized(theme_head, fact, "mdBook theme metadata")?;
    }
    require_fingerprint(
        &theme_head_bytes,
        THEME_HEAD_FINGERPRINT,
        "mdBook theme head",
    )?;

    let root_manifest_bytes =
        fs::read(root.join("Cargo.toml")).context("failed to read the root Cargo manifest")?;
    let root_manifest = std::str::from_utf8(&root_manifest_bytes)
        .context("root Cargo manifest is not valid UTF-8")?;
    for fact in [
        "members = [\"type-exercise\", \"type-exercise-starter\", \"xtask\"]",
        "exclude = [\"archived/type-exercise-ref\", \"archived/legacy-days/*\"]",
    ] {
        require_once_normalized(root_manifest, fact, "root workspace selector")?;
    }
    require_fingerprint(
        &root_manifest_bytes,
        ROOT_MANIFEST_FINGERPRINT,
        "root Cargo manifest",
    )?;

    let reference_manifest_path = root.join("type-exercise/Cargo.toml");
    let reference_manifest_bytes = fs::read(&reference_manifest_path)
        .context("failed to read the reference Cargo manifest")?;
    let reference_manifest = std::str::from_utf8(&reference_manifest_bytes)
        .context("reference Cargo manifest is not valid UTF-8")?;
    for fact in [
        "name = \"type-exercise\"",
        "rust_decimal = { version = \"1\", default-features = false }",
        "criterion = \"0.8.2\"",
        "[[bench]] name = \"expression\" harness = false",
    ] {
        require_once_normalized(
            reference_manifest,
            fact,
            "reference test-target configuration",
        )?;
    }
    require_fingerprint(
        &reference_manifest_bytes,
        REFERENCE_MANIFEST_FINGERPRINT,
        "reference Cargo manifest",
    )?;

    let integration_test_files =
        relative_regular_files_if_present(&root.join("type-exercise/tests"))?;
    if integration_test_files != INTEGRATION_TEST_FILES {
        bail!(
            "reference integration-test file set must match the exact allowlist: expected {:?}, found {integration_test_files:?}",
            INTEGRATION_TEST_FILES
        );
    }

    let supplied_test_dir = root.join("type-exercise/src/tests");
    let supplied_test_files = relative_regular_files(&supplied_test_dir)?;
    if supplied_test_files != SUPPLIED_TEST_FILES {
        bail!(
            "supplied-test file set must match the exact allowlist: expected {:?}, found {supplied_test_files:?}",
            SUPPLIED_TEST_FILES
        );
    }

    let available = available_chapters(&supplied_test_dir)?;
    let expected = (1..=COURSE_CHAPTERS).collect::<Vec<_>>();
    if available != expected {
        bail!("course test sequence must be Chapters 1-{COURSE_CHAPTERS}, found {available:?}");
    }

    let registry_path = root.join("type-exercise/src/tests.rs");
    let registry_bytes =
        fs::read(&registry_path).context("failed to read the reference test module list")?;
    let modules = std::str::from_utf8(&registry_bytes)
        .context("reference test registry is not valid UTF-8")?;
    let registry_lines = modules
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if registry_lines != TEST_REGISTRY_LINES {
        bail!(
            "reference test registry must contain only the exact unconditional chapter modules: expected {:?}, found {registry_lines:?}",
            TEST_REGISTRY_LINES
        );
    }
    require_fingerprint(
        &registry_bytes,
        TEST_REGISTRY_FINGERPRINT,
        "reference test registry",
    )?;
    for chapter in &expected {
        let declaration = format!("mod chapter_{chapter};");
        if modules
            .lines()
            .filter(|line| line.trim() == declaration)
            .count()
            != 1
        {
            bail!("reference test module list must contain exactly one {declaration}");
        }
    }
    if modules
        .lines()
        .filter(|line| line.trim().starts_with("mod chapter_"))
        .count()
        != COURSE_CHAPTERS
    {
        bail!("reference test module list contains an unexpected chapter module");
    }

    let course_dir = root.join("course/src");
    let course_source_files = relative_regular_files(&course_dir)?;
    if course_source_files != COURSE_SOURCE_FILES {
        bail!(
            "course source file set must match the exact allowlist: expected {:?}, found {course_source_files:?}",
            COURSE_SOURCE_FILES
        );
    }
    let mut pages = BTreeMap::new();
    for entry in fs::read_dir(&course_dir).context("failed to list course pages")? {
        let path = entry?.path();
        let Some(chapter) = course_chapter_number(&path) else {
            continue;
        };
        if pages.insert(chapter, path).is_some() {
            bail!("course contains more than one page for Chapter {chapter}");
        }
    }
    if pages.keys().copied().collect::<Vec<_>>() != expected {
        bail!("course page sequence must be Chapters 1-{COURSE_CHAPTERS}");
    }

    let summary = fs::read_to_string(course_dir.join("SUMMARY.md"))
        .context("failed to read the course summary")?;
    let workflow_path = root.join(".github/workflows/ci.yml");
    let workflow = fs::read_to_string(&workflow_path).context("failed to read the CI workflow")?;
    let summary_pages = summary
        .lines()
        .filter_map(|line| line.split_once("](./chapter-").map(|(_, suffix)| suffix))
        .filter_map(|suffix| suffix.strip_suffix(')'))
        .map(|suffix| format!("chapter-{suffix}"))
        .collect::<Vec<_>>();
    let expected_pages = pages
        .values()
        .map(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
                .context("course page name is not valid UTF-8")
        })
        .collect::<Result<Vec<_>>>()?;
    if summary_pages != expected_pages {
        bail!("SUMMARY.md chapter links must match the ordered Chapter 1-{COURSE_CHAPTERS} pages");
    }

    let summary_entries = CHAPTERS
        .iter()
        .map(|chapter| format!("- [{}](./{})", chapter.title, chapter.page))
        .collect::<Vec<_>>();
    require_ordered_markers(&summary, &summary_entries, "SUMMARY.md")?;
    for entry in &summary_entries {
        require_once(&summary, entry, "SUMMARY.md chapter entry")?;
    }
    let expected_summary_members = [
        vec![
            "[Preface](./preface.md)".to_string(),
            "[Environment Setup](./setup.md)".to_string(),
        ],
        summary_entries.clone(),
    ]
    .concat();
    let actual_summary_members = summary
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && *line != "# Summary")
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if actual_summary_members != expected_summary_members {
        bail!(
            "SUMMARY.md members must match the exact allowlist: expected {expected_summary_members:?}, found {actual_summary_members:?}"
        );
    }
    require_fingerprint(summary.as_bytes(), SUMMARY_FINGERPRINT, "SUMMARY.md")?;

    let sitemap_text =
        fs::read_to_string(course_dir.join("sitemap.txt")).context("failed to read sitemap.txt")?;
    let sitemap_text_urls = sitemap_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if sitemap_text_urls != SITEMAP_URLS {
        bail!(
            "sitemap.txt members must match the exact allowlist: expected {:?}, found {sitemap_text_urls:?}",
            SITEMAP_URLS
        );
    }
    require_fingerprint(
        sitemap_text.as_bytes(),
        SITEMAP_TEXT_FINGERPRINT,
        "sitemap.txt",
    )?;

    let sitemap_xml =
        fs::read_to_string(course_dir.join("sitemap.xml")).context("failed to read sitemap.xml")?;
    let sitemap_xml_urls = sitemap_xml_locations(&sitemap_xml)?;
    if sitemap_xml_urls != SITEMAP_URLS {
        bail!(
            "sitemap.xml members must match the exact allowlist: expected {:?}, found {sitemap_xml_urls:?}",
            SITEMAP_URLS
        );
    }
    require_fingerprint(
        sitemap_xml.as_bytes(),
        SITEMAP_XML_FINGERPRINT,
        "sitemap.xml",
    )?;

    for chapter in &CHAPTERS {
        let page = &pages[&chapter.number];
        let body_bytes =
            fs::read(page).with_context(|| format!("failed to read {}", page.display()))?;
        let body = std::str::from_utf8(&body_bytes)
            .with_context(|| format!("{} is not valid UTF-8", page.display()))?;
        let heading = format!("# Chapter {}: {}", chapter.number, chapter.title);
        require_once(
            body,
            &heading,
            &format!("Chapter {} heading", chapter.number),
        )?;
        require_once(
            body,
            "{{#include copyright.md}}",
            &format!("Chapter {} shared feedback footer", chapter.number),
        )?;

        let copy_command = format!("cargo x copy-test --chapter {}", chapter.number);
        let focused_command = format!(
            "cargo test -p type-exercise-starter chapter_{} --locked",
            chapter.number
        );
        require_once(
            body,
            &copy_command,
            &format!("Chapter {} copy command", chapter.number),
        )?;
        if body.matches(&focused_command).count() != 2 {
            bail!(
                "Chapter {} must contain its focused command exactly twice (start and completion)",
                chapter.number
            );
        }

        require_once_normalized(
            body,
            chapter.expected_red,
            &format!("Chapter {} expected-red contract", chapter.number),
        )?;
        for target in chapter.checkpoint_targets {
            require_once_normalized(
                body,
                target,
                &format!("Chapter {} checkpoint target", chapter.number),
            )?;
        }
        require_once_normalized(
            body,
            chapter.boundary,
            &format!("Chapter {} required/extension boundary", chapter.number),
        )?;
        require_once_normalized(
            body,
            chapter.transition,
            &format!("Chapter {} transition", chapter.number),
        )?;
        require_fingerprint(
            &body_bytes,
            chapter.page_fingerprint,
            &format!("Chapter {} page", chapter.number),
        )?;

        if workflow
            .lines()
            .filter(|line| line.trim() == copy_command)
            .count()
            != 1
        {
            bail!("CI must copy Chapter {} exactly once", chapter.number);
        }

        let test_path = root.join(format!(
            "type-exercise/src/tests/chapter_{}.rs",
            chapter.number
        ));
        let test_bytes = fs::read(&test_path)
            .with_context(|| format!("failed to read supplied tests {}", test_path.display()))?;
        let supplied_test_count = std::str::from_utf8(&test_bytes)
            .context("supplied test is not valid UTF-8")?
            .lines()
            .filter(|line| line.trim() == "#[test]")
            .count();
        let expected_test_count = SUPPLIED_TEST_COUNTS[chapter.number - 1];
        if supplied_test_count != expected_test_count {
            bail!(
                "Chapter {} must expose exactly {expected_test_count} supplied tests, found {supplied_test_count}",
                chapter.number
            );
        }
        require_fingerprint(
            &test_bytes,
            chapter.test_fingerprint,
            &format!("Chapter {} supplied-test identity", chapter.number),
        )?;
    }

    let readme_path = root.join("README.md");
    let readme_bytes = fs::read(&readme_path).context("failed to read README.md")?;
    let readme = std::str::from_utf8(&readme_bytes).context("README.md is not valid UTF-8")?;
    for fact in [
        "The book's \u{60}SUMMARY.md\u{60} is the sole ordered chapter list.",
        "Represent one-level Lists with checked offsets and independent outer and child nullability.",
        "The course deliberately stops short of Decimal arithmetic and precision enforcement, implicit narrowing or lossy casts, nested or list-producing functions, concrete four- and five-input builtins, exhaustive fast paths, an aggregate engine, and per-row futures.",
        "Never edit copied tests or \u{60}src/tests.rs\u{60}; keep all earlier copied chapters green.",
    ] {
        require_once_normalized(readme, fact, "README course fact")?;
    }
    require_fingerprint(&readme_bytes, README_FINGERPRINT, "README.md")?;

    let preface_bytes =
        fs::read(course_dir.join("preface.md")).context("failed to read course preface")?;
    let preface = std::str::from_utf8(&preface_bytes).context("preface is not valid UTF-8")?;
    for fact in [
        "A hand-written loop for \u{60}i32 + i32\u{60} is easy:",
        "Repeating those decisions in every loop makes each new function a new place for type drift, null bugs, and inconsistent errors.",
        "Only after their duplication is visible will you introduce the catalogs and generic adapters that remove it.",
        "{{#include copyright.md}}",
    ] {
        require_once_normalized(preface, fact, "preface learner contract")?;
    }
    require_fingerprint(&preface_bytes, PREFACE_FINGERPRINT, "course preface")?;

    let setup_bytes =
        fs::read(course_dir.join("setup.md")).context("failed to read environment setup")?;
    let setup = std::str::from_utf8(&setup_bytes).context("setup is not valid UTF-8")?;
    for fact in [
        "Do not edit \u{60}src/tests.rs\u{60} or copied files under \u{60}src/tests/\u{60}.",
        "The only permitted reference-to-starter operation is:",
        "Its first focused run should be red until you implement the chapter.",
        "{{#include copyright.md}}",
    ] {
        require_once_normalized(setup, fact, "setup learner boundary")?;
    }
    require_fingerprint(&setup_bytes, SETUP_FINGERPRINT, "environment setup")?;

    let footer_bytes =
        fs::read(course_dir.join("copyright.md")).context("failed to read shared course footer")?;
    let footer =
        std::str::from_utf8(&footer_bytes).context("shared course footer is not valid UTF-8")?;
    for fact in [
        "Join our [Discord Community](https://skyzh.dev/join/discord).",
        "Found an issue? Create an [issue or pull request](https://github.com/skyzh/type-exercise-in-rust).",
    ] {
        require_once_normalized(footer, fact, "shared feedback footer")?;
    }
    require_fingerprint(&footer_bytes, FOOTER_FINGERPRINT, "shared course footer")?;

    let svg_path = course_dir.join("assets/map-of-types.svg");
    let svg_bytes = fs::read(&svg_path).context("failed to read the editable type map")?;
    let svg = std::str::from_utf8(&svg_bytes).context("type map is not valid UTF-8")?;
    for fact in [
        "meaning: Integer, Double,",
        "Varchar, Char, Decimal, List(T)",
        "one optional dense i32 fast path; same observable contract",
        "One-level List: outer validity + offsets + child ArrayImpl + independent child validity",
        "Nullability is Option/validity state, not DataType::Nullable. Aggregate execution is outside this course.",
    ] {
        require_once_normalized(svg, fact, "type-map fact")?;
    }
    require_fingerprint(&svg_bytes, SVG_FINGERPRINT, "editable type map")?;

    let social_image = fs::read(course_dir.join("type-exercise-social.png"))
        .context("failed to read the course social image")?;
    require_fingerprint(
        &social_image,
        SOCIAL_IMAGE_FINGERPRINT,
        "course social image",
    )?;

    let workflow_markers = [
        "sha256sum --check .github/course-bootstrap.sha256",
        "bash .github/test-course-bootstrap.sh",
        "cargo fmt --all -- --check",
        "cargo fmt --manifest-path archived/type-exercise-ref/Cargo.toml --all -- --check",
        "cargo clippy --workspace --all-targets --locked -- -D warnings",
        "cargo clippy --manifest-path archived/type-exercise-ref/Cargo.toml --workspace --all-targets --all-features --locked -- -D warnings",
        "cargo test --workspace --locked",
        "cargo test --color never -p type-exercise-xtask --locked -- --test-threads=1",
        "cargo test --manifest-path archived/type-exercise-ref/Cargo.toml --no-fail-fast --workspace --all-features --locked",
        "cargo check -p type-exercise-starter --lib --locked",
        "cargo x copy-test --chapter 12",
        "cargo x check-course",
        "mdbook test course",
        "npm install --global static-sitemap-cli@2.2.8",
        "course/sitemap.sh --check",
    ];
    let normalized_workflow = normalized(&workflow);
    require_ordered_markers(
        &normalized_workflow,
        &workflow_markers
            .iter()
            .map(|marker| marker.to_string())
            .collect::<Vec<_>>(),
        "CI command set",
    )?;
    for marker in workflow_markers {
        require_once(&normalized_workflow, marker, "CI command")?;
    }
    require_fingerprint(workflow.as_bytes(), CI_FINGERPRINT, "complete CI workflow")?;

    println!(
        "verified the exact Chapters 1-{COURSE_CHAPTERS} course source, supplied tests, learner dependencies, README, SVG, SUMMARY, sitemaps, and CI"
    );
    Ok(())
}

fn verify_real_reference_inventory(root: &Path) -> Result<()> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .current_dir(root)
        .env("CARGO_TERM_COLOR", "never")
        .args([
            "test",
            "-p",
            "type-exercise",
            "--lib",
            "--locked",
            "--",
            "--test-threads=1",
        ])
        .output()
        .context("failed to execute the real reference test inventory")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        bail!("real reference test inventory failed:\n{stdout}\n{stderr}");
    }
    let summary = "test result: ok. 76 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;";
    require_once(&stdout, summary, "real reference 76-test inventory")?;
    println!("executed the exact real reference inventory: 76/76 tests");
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let root = workspace_root()?;
    match args.action {
        Action::CopyTest { chapter } => {
            copy_test(&root, chapter)?;
        }
        Action::CheckCourse => {
            check_course_contract(&root)?;
            verify_real_reference_inventory(&root)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use tempfile::TempDir;

    use super::{CHAPTERS, COURSE_SOURCE_FILES, check_course_contract, copy_test};

    fn fixture() -> TempDir {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("type-exercise/src/tests");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(root.path().join("type-exercise-starter/src")).unwrap();
        fs::write(source.join("chapter_1.rs"), b"// one\r\n#[test]\n").unwrap();
        fs::write(source.join("chapter_2.rs"), b"// two\n").unwrap();
        fs::write(source.join("chapter_3.rs"), b"// three\n").unwrap();
        root
    }

    fn repository_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf()
    }

    fn copy_file(source_root: &Path, target_root: &Path, relative: &str) {
        let source = source_root.join(relative);
        let target = target_root.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(source, target).unwrap();
    }

    fn contract_fixture() -> TempDir {
        let fixture = tempfile::tempdir().unwrap();
        let source = repository_root();
        let target = fixture.path();

        for relative in [
            ".cargo/config.toml",
            "Cargo.toml",
            "README.md",
            ".github/workflows/ci.yml",
            "course/book.toml",
            "course/theme/head.hbs",
            "type-exercise/Cargo.toml",
            "type-exercise/src/tests.rs",
            "xtask/Cargo.toml",
        ] {
            copy_file(&source, target, relative);
        }
        for relative in COURSE_SOURCE_FILES {
            copy_file(&source, target, &format!("course/src/{relative}"));
        }
        for chapter in CHAPTERS {
            copy_file(
                &source,
                target,
                &format!("type-exercise/src/tests/chapter_{}.rs", chapter.number),
            );
        }
        fixture
    }

    fn replace_exactly_once(path: &Path, from: &str, to: &str) {
        let body = fs::read_to_string(path).unwrap();
        assert_eq!(
            body.matches(from).count(),
            1,
            "mutation source must be unique in {}",
            path.display()
        );
        fs::write(path, body.replacen(from, to, 1)).unwrap();
    }

    fn assert_contract_mutation_rejected(name: &str, relative: &str, from: &str, to: &str) {
        let fixture = contract_fixture();
        check_course_contract(fixture.path()).unwrap();
        replace_exactly_once(&fixture.path().join(relative), from, to);
        let error = check_course_contract(fixture.path()).unwrap_err();
        assert!(
            !error.to_string().is_empty(),
            "{name} must produce an actionable contract error"
        );
    }

    #[test]
    fn rejects_every_reported_publication_contract_survivor() {
        let mutations = [
            (
                "swapped SUMMARY order",
                "course/src/SUMMARY.md",
                "- [Connect One Type Family by Hand](./chapter-1-type-family.md)\n- [Scale the Physical Type Family](./chapter-2-type-catalog.md)",
                "- [Scale the Physical Type Family](./chapter-2-type-catalog.md)\n- [Connect One Type Family by Hand](./chapter-1-type-family.md)",
            ),
            (
                "nonexistent checkpoint target",
                "course/src/chapter-5-generic-arithmetic.md",
                "promote_numeric",
                "promote_numbers",
            ),
            (
                "inverted expected-red contract",
                "course/src/chapter-4-concrete-loops.md",
                "The first run should fail on the missing scalar-function traits, adapters, and batch evaluator.",
                "The untouched checkpoint should pass before the learner implements anything.",
            ),
            (
                "learner edits supplied tests",
                "README.md",
                "Never edit copied tests or \u{60}src/tests.rs\u{60}; keep all earlier copied\nchapters green.",
                "Edit copied tests or remove their assertions when they block progress.",
            ),
            (
                "skipped Chapter 10 and 11 transition",
                "course/src/chapter-9-primitive-loops.md",
                "Next: [Chapter 10 adds one-level List storage](./chapter-10-list.md).",
                "Next: [Chapter 12 adds one future around a whole batch](./chapter-12-async-boundary.md).",
            ),
            (
                "required nested List construction",
                "course/src/chapter-10-list.md",
                "construction must reject it until those contracts exist",
                "construction must accept it as required work",
            ),
            (
                "false SVG nullability and aggregate scope",
                "course/src/assets/map-of-types.svg",
                "Nullability is Option/validity state, not DataType::Nullable. Aggregate execution is outside this course.",
                "Nullability uses DataType::Nullable. Aggregate execution is required in this course.",
            ),
            (
                "false README aggregate scope",
                "README.md",
                "an aggregate engine, and per-row futures",
                "an aggregate engine is required, and per-row futures",
            ),
            (
                "false README Decimal scope",
                "README.md",
                "stops short of Decimal arithmetic and precision enforcement",
                "requires Decimal arithmetic and precision enforcement",
            ),
            (
                "false README exhaustive fast-path scope",
                "README.md",
                "exhaustive fast paths, an aggregate engine",
                "exhaustive fast paths, including an aggregate engine",
            ),
            (
                "weakened current-workspace CI",
                ".github/workflows/ci.yml",
                "run: cargo test --workspace --locked",
                "run: cargo test -p type-exercise-xtask --locked",
            ),
            (
                "duplicate Chapter 1 copy command",
                "course/src/chapter-1-type-family.md",
                "cargo x copy-test --chapter 1\ncargo test -p type-exercise-starter chapter_1 --locked",
                "cargo x copy-test --chapter 1\ncargo x copy-test --chapter 1\ncargo test -p type-exercise-starter chapter_1 --locked",
            ),
            (
                "changed supplied-test identity",
                "type-exercise/src/tests/chapter_1.rs",
                "fn connects_the_explicit_integer_and_string_families()",
                "fn connects_only_part_of_the_explicit_family()",
            ),
            (
                "conditionally disabled real Chapter 12 registry",
                "type-exercise/src/tests.rs",
                "mod chapter_12;",
                "#[cfg(any())]\nmod chapter_12;",
            ),
            (
                "missing real Chapter 12 registry",
                "type-exercise/src/tests.rs",
                "mod chapter_12;\n",
                "",
            ),
            (
                "removed hand-written-loop motivation",
                "course/src/preface.md",
                "Repeating those decisions in every loop",
                "Loop duplication is not part of the course motivation",
            ),
            (
                "reversed setup supplied-test ownership",
                "course/src/setup.md",
                "- Do not edit \u{60}src/tests.rs\u{60} or copied files under \u{60}src/tests/\u{60}.",
                "- Edit \u{60}src/tests.rs\u{60} or copied files whenever an assertion blocks progress.",
            ),
            (
                "removed shared feedback footer",
                "course/src/copyright.md",
                "Your feedback is greatly appreciated. Join our [Discord Community](https://skyzh.dev/join/discord).",
                "No learner feedback channel is available.",
            ),
            (
                "redirected mdBook source",
                "course/book.toml",
                "src = \"src\"",
                "src = \"alternate\"",
            ),
            (
                "corrupted mdBook title",
                "course/book.toml",
                "title = \"Build a Typed Database Expression Engine in Rust\"",
                "title = \"Unrelated Generic Rust Notes\"",
            ),
            (
                "corrupted rendered author",
                "course/theme/head.hbs",
                "<meta name=\"author\" content=\"Alex Chi Z\">",
                "<meta name=\"author\" content=\"Anonymous Course Bot\">",
            ),
            (
                "excluded reference package from workspace",
                "Cargo.toml",
                "exclude = [\"archived/type-exercise-ref\", \"archived/legacy-days/*\"]",
                "exclude = [\"type-exercise\", \"archived/type-exercise-ref\", \"archived/legacy-days/*\"]",
            ),
            (
                "disabled real reference lib tests",
                "type-exercise/Cargo.toml",
                "[dependencies]",
                "[lib]\ntest = false\n\n[dependencies]",
            ),
        ];

        for (name, relative, from, to) in mutations {
            assert_contract_mutation_rejected(name, relative, from, to);
        }
    }

    #[test]
    fn rejects_unmanifested_course_and_supplied_test_members() {
        let aggregate = contract_fixture();
        fs::write(
            aggregate.path().join("course/src/aggregate.md"),
            "# Required Aggregate Engine\n",
        )
        .unwrap();
        let summary_path = aggregate.path().join("course/src/SUMMARY.md");
        let mut summary = fs::read_to_string(&summary_path).unwrap();
        summary.push_str("\n- [Required Aggregate Engine](./aggregate.md)\n");
        fs::write(summary_path, summary).unwrap();
        let sitemap_text_path = aggregate.path().join("course/src/sitemap.txt");
        let mut sitemap_text = fs::read_to_string(&sitemap_text_path).unwrap();
        sitemap_text.push_str("https://skyzh.github.io/type-exercise-in-rust/aggregate\n");
        fs::write(sitemap_text_path, sitemap_text).unwrap();
        let sitemap_xml_path = aggregate.path().join("course/src/sitemap.xml");
        let sitemap_xml = fs::read_to_string(&sitemap_xml_path)
            .unwrap()
            .replace(
                "</urlset>",
                "    <url>\n        <loc>https://skyzh.github.io/type-exercise-in-rust/aggregate</loc>\n    </url>\n</urlset>",
            );
        fs::write(sitemap_xml_path, sitemap_xml).unwrap();
        assert!(check_course_contract(aggregate.path()).is_err());

        let bonus = contract_fixture();
        fs::write(
            bonus.path().join("type-exercise/src/tests/bonus.rs"),
            "#[test]\nfn unmanifested_bonus() {}\n",
        )
        .unwrap();
        let registry_path = bonus.path().join("type-exercise/src/tests.rs");
        let mut registry = fs::read_to_string(&registry_path).unwrap();
        registry.push_str("mod bonus;\n");
        fs::write(registry_path, registry).unwrap();
        assert!(check_course_contract(bonus.path()).is_err());
    }

    #[test]
    fn rejects_unmanifested_theme_and_integration_test_targets() {
        let theme = contract_fixture();
        fs::write(
            theme.path().join("course/theme/index.hbs"),
            "<main>course unavailable</main>\n",
        )
        .unwrap();
        assert!(check_course_contract(theme.path()).is_err());

        let integration = contract_fixture();
        let integration_test = integration.path().join("type-exercise/tests/bonus.rs");
        fs::create_dir_all(integration_test.parent().unwrap()).unwrap();
        fs::write(
            integration_test,
            "#[test]\nfn unmanifested_integration() {}\n",
        )
        .unwrap();
        assert!(check_course_contract(integration.path()).is_err());
    }

    #[test]
    fn chapter_12_copy_is_complete_and_compiles_every_declared_module() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("type-exercise/src/tests");
        let starter = root.path().join("type-exercise-starter/src");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&starter).unwrap();
        fs::write(starter.join("lib.rs"), "#[cfg(test)]\nmod tests;\n").unwrap();

        for chapter in 1..=12 {
            fs::write(
                source.join(format!("chapter_{chapter}.rs")),
                format!(
                    "#[test]\nfn chapter_{chapter}_contract_compiles() {{ assert_eq!({chapter}, {chapter}); }}\n"
                ),
            )
            .unwrap();
        }

        copy_test(root.path(), 12).unwrap();
        for chapter in 1..=12 {
            assert_eq!(
                fs::read(starter.join(format!("tests/chapter_{chapter}.rs"))).unwrap(),
                fs::read(source.join(format!("chapter_{chapter}.rs"))).unwrap()
            );
        }
        assert_eq!(
            fs::read_to_string(starter.join("tests.rs")).unwrap(),
            "//! DO NOT MODIFY -- copied course test modules\n\
             //! This file is rewritten by \u{60}cargo x copy-test\u{60}.\n\
             mod chapter_1;\n\
             mod chapter_2;\n\
             mod chapter_3;\n\
             mod chapter_4;\n\
             mod chapter_5;\n\
             mod chapter_6;\n\
             mod chapter_7;\n\
             mod chapter_8;\n\
             mod chapter_9;\n\
             mod chapter_10;\n\
             mod chapter_11;\n\
             mod chapter_12;\n"
        );

        let executable = root.path().join("copied-course-tests");
        let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
        let compile = Command::new(rustc)
            .arg("--edition=2024")
            .arg("--test")
            .arg(starter.join("lib.rs"))
            .arg("-o")
            .arg(&executable)
            .status()
            .unwrap();
        assert!(
            compile.success(),
            "all declared Chapter 1-12 modules must compile"
        );
        assert!(
            Command::new(executable).status().unwrap().success(),
            "all copied Chapter 1-12 test modules must execute"
        );
    }

    #[test]
    fn rejects_invalid_chapters_before_mutating_the_starter() {
        let root = fixture();
        for chapter in [0, 4, usize::MAX] {
            let error = copy_test(root.path(), chapter).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains(&format!("no tests are available for chapter {chapter}"))
            );
        }
        assert!(!root.path().join("type-exercise-starter/src/tests").exists());
        assert!(
            !root
                .path()
                .join("type-exercise-starter/src/tests.rs")
                .exists()
        );
    }

    #[test]
    fn copies_an_exact_cumulative_prefix_and_removes_later_managed_tests() {
        let root = fixture();
        copy_test(root.path(), 3).unwrap();

        for chapter in 1..=3 {
            let source = root
                .path()
                .join(format!("type-exercise/src/tests/chapter_{chapter}.rs"));
            let target = root.path().join(format!(
                "type-exercise-starter/src/tests/chapter_{chapter}.rs"
            ));
            assert_eq!(fs::read(target).unwrap(), fs::read(source).unwrap());
        }

        copy_test(root.path(), 2).unwrap();
        assert!(
            !root
                .path()
                .join("type-exercise-starter/src/tests/chapter_3.rs")
                .exists()
        );
        assert_eq!(
            fs::read_to_string(root.path().join("type-exercise-starter/src/tests.rs")).unwrap(),
            "//! DO NOT MODIFY -- copied course test modules\n\
             //! This file is rewritten by `cargo x copy-test`.\n\
             mod chapter_1;\n\
             mod chapter_2;\n"
        );
    }

    #[test]
    fn repeated_sync_is_byte_identical_and_does_not_rewrite_files() {
        let root = fixture();
        assert_eq!(copy_test(root.path(), 3).unwrap().changed_files, 4);
        assert_eq!(copy_test(root.path(), 3).unwrap().changed_files, 0);
        assert_eq!(
            fs::read(
                root.path()
                    .join("type-exercise-starter/src/tests/chapter_1.rs")
            )
            .unwrap(),
            b"// one\r\n#[test]\n"
        );
    }

    #[test]
    fn repairs_existing_starter_drift_from_reference_bytes() {
        let root = fixture();
        let target = root.path().join("type-exercise-starter/src/tests");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("chapter_1.rs"), b"// drifted\n").unwrap();
        fs::write(
            root.path().join("type-exercise-starter/src/tests.rs"),
            b"mod stale;\n",
        )
        .unwrap();

        let report = copy_test(root.path(), 2).unwrap();
        assert_eq!(report.changed_files, 3);
        assert_eq!(
            fs::read(target.join("chapter_1.rs")).unwrap(),
            fs::read(root.path().join("type-exercise/src/tests/chapter_1.rs")).unwrap()
        );
        assert_eq!(
            fs::read(target.join("chapter_2.rs")).unwrap(),
            fs::read(root.path().join("type-exercise/src/tests/chapter_2.rs")).unwrap()
        );
    }
}
