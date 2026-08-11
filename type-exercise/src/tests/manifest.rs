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
    chapters.sort_unstable();
    let expected = (1..=12).collect::<Vec<_>>();
    assert_eq!(
        chapters, expected,
        "reference test manifest must be Chapters 1..=12 exactly"
    );
}
