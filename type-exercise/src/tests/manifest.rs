//! Reference-only cumulative module-manifest guard.
//!
//! The active reference test manifest must list every chapter module from
//! Chapter 1 through the newest published chapter, contiguously and exactly
//! once. This file is not copied into the learner starter.

#[test]
fn reference_test_manifest_is_cumulative_and_contiguous() {
    let manifest = include_str!("../tests.rs");
    let mut chapters = Vec::new();
    for line in manifest.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("mod chapter_") {
            let number = rest
                .strip_suffix(';')
                .expect("chapter module line must end with ';'")
                .parse::<usize>()
                .expect("chapter module number must be an integer");
            chapters.push(number);
        }
    }
    let expected = (1..=9).collect::<Vec<_>>();
    assert_eq!(
        chapters, expected,
        "reference test manifest must be Chapters 1..=9 exactly"
    );
}

/// Detect learner-layout leaks in copied test source: direct variant paths,
/// whitespace-split paths, `use` renames, and `type` aliases that would pin
/// the reference's `BindError`/`ExpressionError` layouts.
fn loose_error_leak(source: &str) -> Option<&'static str> {
    let compact: String = source.chars().filter(|c| !c.is_whitespace()).collect();
    [
        "BindError::",
        "ExpressionError::",
        "BindErroras",
        "ExpressionErroras",
        "=BindError",
        "=ExpressionError",
    ]
    .into_iter()
    .find(|needle| compact.contains(needle))
}

#[test]
fn loose_error_guard_detects_all_bypass_forms() {
    // Direct path.
    assert_eq!(
        loose_error_leak("let _ = BindError::UnknownFunction { name };"),
        Some("BindError::")
    );
    // Whitespace-split path.
    assert_eq!(
        loose_error_leak("let _ = BindError :: UnknownFunction;"),
        Some("BindError::")
    );
    // use-rename form.
    assert_eq!(
        loose_error_leak("use crate::BindError as BE; let _ = BE::UnknownFunction;"),
        Some("BindErroras")
    );
    // type-alias form.
    assert_eq!(
        loose_error_leak("type BE = BindError; let _ = BE::UnknownFunction;"),
        Some("=BindError")
    );
    // Clean learner code stays allowed.
    assert_eq!(
        loose_error_leak("let _ = expression.evaluate(&inputs).is_err();"),
        None
    );
}

/// Reference-only guard: copied learner tests must never construct or match
/// the reference's `BindError`/`ExpressionError` variants, because their
/// layouts are the learner's readable choice (loose-Err boundary).
#[test]
fn copied_learner_tests_do_not_pin_error_layouts() {
    for source in [
        include_str!("chapter_1.rs"),
        include_str!("chapter_2.rs"),
        include_str!("chapter_3.rs"),
        include_str!("chapter_4.rs"),
        include_str!("chapter_5.rs"),
        include_str!("chapter_6.rs"),
        include_str!("chapter_7.rs"),
        include_str!("chapter_8.rs"),
        include_str!("chapter_9.rs"),
    ] {
        assert!(
            loose_error_leak(source).is_none(),
            "copied tests must not pin BindError/ExpressionError layouts"
        );
    }
}
