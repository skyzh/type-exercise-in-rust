use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const CHAPTERS: &[(usize, &str)] = &[
    (1, "chapter-1-type-family.md"),
    (2, "chapter-2-type-catalog.md"),
    (3, "chapter-3-column-views.md"),
    (4, "chapter-4-concrete-loops.md"),
    (5, "chapter-5-generic-arithmetic.md"),
    (6, "chapter-6-systematic-arity.md"),
    (7, "chapter-7-boolean-logic.md"),
    (8, "chapter-8-runtime-erasure.md"),
    (9, "chapter-9-binding-coercion.md"),
    (10, "chapter-10-primitive-loops.md"),
    (11, "chapter-11-list.md"),
    (12, "chapter-12-rust-boundaries.md"),
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("reference crate must be inside the workspace")
        .to_path_buf()
}

fn read(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn count(haystack: &str, needle: &str) -> usize {
    haystack.match_indices(needle).count()
}

fn line_count(haystack: &str, needle: &str) -> usize {
    haystack
        .lines()
        .filter(|line| line.trim() == needle)
        .count()
}

fn validate_summary(summary: &str) -> Result<(), String> {
    let mut previous_summary_offset = 0;
    for &(_, file) in CHAPTERS {
        let summary_link = format!("](./{file})");
        if count(summary, &summary_link) != 1 {
            return Err(format!("SUMMARY must contain {file} exactly once"));
        }
        let offset = summary
            .find(&summary_link)
            .ok_or_else(|| format!("SUMMARY is missing {file}"))?;
        if offset < previous_summary_offset {
            return Err("SUMMARY chapter order is not cumulative".to_owned());
        }
        previous_summary_offset = offset;
    }
    Ok(())
}

fn validate_public_contract_text(
    chapter_2: &str,
    chapter_3: &str,
    starter_readme: &str,
    starter_agents: &str,
) -> Result<(), String> {
    let required = [
        (
            "course/src/chapter-2-type-catalog.md",
            chapter_2,
            [
                "published course currently requires Chapters 1–12",
                "Reserved Day 13 async scaffolds",
                "currently published or required prefix",
            ]
            .as_slice(),
        ),
        (
            "course/src/chapter-3-column-views.md",
            chapter_3,
            [
                "Primitive specialization remains Chapter 10.",
                "same representation boundary in Chapter 11.",
            ]
            .as_slice(),
        ),
        (
            "type-exercise-starter/README.md",
            starter_readme,
            [
                "published course currently requires Day 1–12 checkpoints",
                "Reserved Day 13 async scaffolds",
                "are not currently published",
            ]
            .as_slice(),
        ),
        (
            "type-exercise-starter/AGENTS.md",
            starter_agents,
            [
                "Published and required Day 1–12 ownership",
                "Reserved Day 13 async scaffolds",
                "not current requirements",
            ]
            .as_slice(),
        ),
    ];
    for (relative, source, anchors) in required {
        validate_scaffold_anchors(relative, source, anchors)?;
    }

    let stale = [
        (
            "course/src/chapter-3-column-views.md",
            chapter_3,
            "List will reuse the same representation boundary in Chapter 10",
        ),
        (
            "course/src/chapter-2-type-catalog.md",
            chapter_2,
            "every later target through Day 13",
        ),
        (
            "course/src/chapter-2-type-catalog.md",
            chapter_2,
            "Days 8–13",
        ),
        (
            "type-exercise-starter/README.md",
            starter_readme,
            "Day 1–13 checkpoints",
        ),
        (
            "type-exercise-starter/AGENTS.md",
            starter_agents,
            "Day 1–13 ownership",
        ),
    ];
    for (relative, source, forbidden) in stale {
        if source.contains(forbidden) {
            return Err(format!(
                "{relative} contains stale public contract {forbidden:?}"
            ));
        }
    }

    Ok(())
}

fn validate_publication_sync(root: &Path) -> Result<(), String> {
    let summary = read(root.join("course/src/SUMMARY.md"));
    let reference_manifest = read(root.join("type-exercise/src/tests.rs"));
    let workflow = read(root.join(".github/workflows/ci.yml"));
    let sitemap_text = read(root.join("course/src/sitemap.txt"));
    let sitemap_xml = read(root.join("course/src/sitemap.xml"));
    let chapter_2 = read(root.join("course/src/chapter-2-type-catalog.md"));
    let chapter_3 = read(root.join("course/src/chapter-3-column-views.md"));
    let starter_readme = read(root.join("type-exercise-starter/README.md"));
    let starter_agents = read(root.join("type-exercise-starter/AGENTS.md"));

    validate_summary(&summary)?;
    validate_public_contract_text(&chapter_2, &chapter_3, &starter_readme, &starter_agents)?;
    for &(chapter, file) in CHAPTERS {
        let chapter_path = root.join("course/src").join(file);
        let chapter_source = read(&chapter_path);
        if !chapter_source.contains(&format!("# Chapter {chapter}:")) {
            return Err(format!(
                "{} has the wrong chapter heading",
                chapter_path.display()
            ));
        }
        if count(&chapter_source, "{{#include wip-banner.md}}") != 1 {
            return Err(format!(
                "{} must include the shared banner once",
                chapter_path.display()
            ));
        }

        let test_module = format!("mod chapter_{chapter};");
        if count(&reference_manifest, &test_module) != 1 {
            return Err(format!(
                "reference manifest must contain {test_module} once"
            ));
        }
        if !root
            .join(format!("type-exercise/src/tests/chapter_{chapter}.rs"))
            .is_file()
        {
            return Err(format!("reference Chapter {chapter} test is missing"));
        }

        let copy_command = format!("cargo x copy-test --chapter {chapter}");
        if line_count(&workflow, &copy_command) != 1 {
            return Err(format!("CI must run {copy_command} exactly once"));
        }

        let slug = file
            .strip_suffix(".md")
            .expect("chapter file must end in .md");
        let url = format!("https://skyzh.github.io/type-exercise-in-rust/{slug}");
        if line_count(&sitemap_text, &url) != 1
            || count(&sitemap_xml, &format!("<loc>{url}</loc>")) != 1
        {
            return Err(format!("sitemaps must contain {url} exactly once"));
        }
    }

    for pair in CHAPTERS.windows(2) {
        let (_, current_file) = pair[0];
        let (_, next_file) = pair[1];
        let current = read(root.join("course/src").join(current_file));
        if count(&current, &format!("(./{next_file})")) != 1 {
            return Err(format!(
                "{current_file} must link to {next_file} exactly once"
            ));
        }
    }

    let future = 13;
    let test_module = format!("mod chapter_{future};");
    let copy_command = format!("cargo x copy-test --chapter {future}");
    if reference_manifest.contains(&test_module) || workflow.contains(&copy_command) {
        return Err(format!(
            "future Chapter {future} leaked into the published prefix"
        ));
    }

    for chapter in [10, 11, 12] {
        let copied_test = read(root.join(format!("type-exercise/src/tests/chapter_{chapter}.rs")));
        for reference_variant in ["ListError::", "ExpressionError::"] {
            if copied_test.contains(reference_variant) {
                return Err(format!(
                    "Day {chapter} copied tests must check behavioral Err, not {reference_variant}"
                ));
            }
        }
    }

    Ok(())
}

fn learner_rust_files(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("failed to list {}: {error}", directory.display()))
    {
        let path = entry
            .expect("starter directory entry must be readable")
            .path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "tests") {
                continue;
            }
            learner_rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs")
            && path.file_name().is_some_and(|name| name != "tests.rs")
        {
            files.push(path);
        }
    }
}

fn active_identifiers(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .flat_map(|line| {
            line.split_once("//")
                .map_or(line, |(code, _)| code)
                .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        })
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
        .collect()
}

fn validate_scaffold_anchors(relative: &str, source: &str, anchors: &[&str]) -> Result<(), String> {
    for anchor in anchors {
        if count(source, anchor) != 1 {
            return Err(format!("{relative} must contain {anchor:?} exactly once"));
        }
    }
    Ok(())
}

fn validate_solution_free_source(relative: &str, source: &str) -> Result<(), String> {
    let identifiers = active_identifiers(source);
    let forbidden = [
        "NonNullPrimitiveArray",
        "PrimitiveBinaryExpression",
        "PrimitiveLoop",
        "as_non_null",
        "evaluate_with_loop",
        "List",
        "ListError",
        "ListScalar",
        "ListScalarRef",
        "ListArray",
        "ListArrayBuilder",
        "ListColumnView",
        "try_as_list",
        "list_array",
        "ArrayIterator",
        "Any",
        "Send",
        "Sync",
        "downcast_ref",
    ];
    for identifier in forbidden {
        if identifiers.contains(identifier) {
            return Err(format!(
                "solution-owned identifier {identifier} is active in {relative}"
            ));
        }
    }
    Ok(())
}

fn validate_starter_scaffold(root: &Path) -> Result<(), String> {
    let required = [
        (
            "type-exercise-starter/src/array/primitive_array.rs",
            [
                "// pub struct NonNullPrimitiveArray<'a, T> {",
                "// pub fn null_count(&self) -> usize;",
                "// pub fn as_non_null(&self) -> Option<NonNullPrimitiveArray<'_, T>>;",
            ]
            .as_slice(),
        ),
        (
            "type-exercise-starter/src/expression.rs",
            [
                "// pub enum PrimitiveLoop {",
                "// pub struct PrimitiveBinaryExpression<F> {",
                "//     pub fn evaluate_with_loop(",
            ]
            .as_slice(),
        ),
        (
            "type-exercise-starter/src/binder.rs",
            ["//     pub fn evaluate_with_loop("].as_slice(),
        ),
        (
            "type-exercise-starter/src/array/list_array.rs",
            [
                "// pub enum ListError {",
                "// pub struct ListScalar {",
                "// pub struct ListScalarRef<'a> {",
                "// pub struct ListArray {",
                "// pub(crate) struct ListArrayBuilder {",
                "//     pub fn try_from_rows<'a>(",
                "//     pub fn try_from_raw_parts(",
            ]
            .as_slice(),
        ),
        (
            "type-exercise-starter/src/array.rs",
            [
                "// mod list_array;",
                "// pub use list_array::{ListArray, ListError, ListScalar, ListScalarRef};",
            ]
            .as_slice(),
        ),
        (
            "type-exercise-starter/src/column.rs",
            [
                "// pub struct ListColumnView<'a> {",
                "//     pub fn try_as_list(",
            ]
            .as_slice(),
        ),
        (
            "type-exercise-starter/src/lib.rs",
            [
                "the active `pub use array::*` below exports `ListArray`, `ListError`,",
                "// pub use column::{ColumnView, ColumnViewImpl, ListColumnView};",
            ]
            .as_slice(),
        ),
        (
            "type-exercise-starter/src/array.rs",
            [
                "// mod iterator;",
                "/// Day 12, checkpoint 1: replace this adapter with the private iterator from",
                "/// use type_exercise_starter::ArrayIterator;",
            ]
            .as_slice(),
        ),
        (
            "type-exercise-starter/src/array/iterator.rs",
            ["// pub(crate) struct ArrayIterator<'a, A: Array> {"].as_slice(),
        ),
        (
            "type-exercise-starter/src/expression.rs",
            [
                "// pub trait Expression: Any + Send + Sync {",
                "// impl dyn Expression { /* checked Any recovery */ }",
            ]
            .as_slice(),
        ),
        (
            "type-exercise-starter/src/binder.rs",
            [
                "//! Day 12, checkpoint 3: require registered factories to be Send + Sync + 'static here.",
                "// impl FunctionRegistry { /* strengthen register, register_unary, register_binary, register_ternary */ }",
            ]
            .as_slice(),
        ),
        (
            "type-exercise-starter/src/column.rs",
            ["// pub struct ColumnViewImpl<'a> {"].as_slice(),
        ),
    ];

    for (relative, anchors) in required {
        let source = read(root.join(relative));
        validate_scaffold_anchors(relative, &source, anchors)?;
    }

    let mut rust_files = Vec::new();
    learner_rust_files(&root.join("type-exercise-starter/src"), &mut rust_files);
    for path in rust_files {
        validate_solution_free_source(&path.display().to_string(), &read(&path))?;
    }

    Ok(())
}

#[test]
fn public_course_and_test_surfaces_are_synchronized_through_day_12() {
    validate_publication_sync(&workspace_root()).unwrap();
}

#[test]
fn day_10_through_12_starter_scaffolds_are_complete_but_solution_free() {
    validate_starter_scaffold(&workspace_root()).unwrap();
}

#[test]
fn publication_sync_guard_fails_on_real_surface_drift() {
    let root = workspace_root();
    let summary_path = root.join("course/src/SUMMARY.md");
    let summary = read(&summary_path);
    let drifted = summary.replace(
        "- [Strengthen Rust Type Boundaries](./chapter-12-rust-boundaries.md)\n",
        "",
    );
    assert_ne!(summary, drifted);
    let error = validate_summary(&drifted).unwrap_err();
    assert!(error.contains("chapter-12-rust-boundaries.md"));
}

#[test]
fn public_contract_guard_rejects_stale_list_destination_and_day_13_conflation() {
    let root = workspace_root();
    let chapter_2 = read(root.join("course/src/chapter-2-type-catalog.md"));
    let chapter_3 = read(root.join("course/src/chapter-3-column-views.md"));
    let starter_readme = read(root.join("type-exercise-starter/README.md"));
    let starter_agents = read(root.join("type-exercise-starter/AGENTS.md"));

    let stale_list = chapter_3.replace(
        "same representation boundary in Chapter 11.",
        "same representation boundary in Chapter 10.",
    );
    assert_ne!(chapter_3, stale_list);
    let list_error =
        validate_public_contract_text(&chapter_2, &stale_list, &starter_readme, &starter_agents)
            .unwrap_err();
    assert!(list_error.contains("Chapter 11"));

    let stale_chapter_2 = chapter_2.replace(
        "published course currently requires Chapters 1–12",
        "starter source names every later target through Day 13",
    );
    assert_ne!(chapter_2, stale_chapter_2);
    let chapter_2_error = validate_public_contract_text(
        &stale_chapter_2,
        &chapter_3,
        &starter_readme,
        &starter_agents,
    )
    .unwrap_err();
    assert!(chapter_2_error.contains("Chapters 1–12"));

    let stale_readme = starter_readme.replace(
        "published course currently requires Day 1–12 checkpoints",
        "actual files contain the Day 1–13 checkpoints",
    );
    assert_ne!(starter_readme, stale_readme);
    let readme_error =
        validate_public_contract_text(&chapter_2, &chapter_3, &stale_readme, &starter_agents)
            .unwrap_err();
    assert!(readme_error.contains("Day 1–12 checkpoints"));

    let stale_agents = starter_agents.replace(
        "Published and required Day 1–12 ownership",
        "Day 1–13 ownership",
    );
    assert_ne!(starter_agents, stale_agents);
    let agents_error =
        validate_public_contract_text(&chapter_2, &chapter_3, &starter_readme, &stale_agents)
            .unwrap_err();
    assert!(agents_error.contains("Day 1–12 ownership"));
}

#[test]
fn starter_guard_fails_on_missing_scaffold_and_active_solution_symbols() {
    let root = workspace_root();
    let list_path = root.join("type-exercise-starter/src/array/list_array.rs");
    let source = read(&list_path);

    let missing = source.replace("// pub struct ListArray", "// pub struct RenamedListArray");
    assert_ne!(source, missing);
    let missing_error = validate_scaffold_anchors(
        "type-exercise-starter/src/array/list_array.rs",
        &missing,
        &["// pub struct ListArray {"],
    )
    .unwrap_err();
    assert!(missing_error.contains("pub struct ListArray"));

    let leaked = source.replace("// pub struct ListArray", "pub struct ListArray");
    assert_ne!(source, leaked);
    let leakage_error =
        validate_solution_free_source("type-exercise-starter/src/array/list_array.rs", &leaked)
            .unwrap_err();
    assert!(leakage_error.contains("ListArray"));

    let iterator_path = root.join("type-exercise-starter/src/array/iterator.rs");
    let iterator_source = read(&iterator_path);
    let missing_iterator = iterator_source.replace("ArrayIterator", "RenamedArrayIterator");
    assert_ne!(iterator_source, missing_iterator);
    let missing_iterator_error = validate_scaffold_anchors(
        "type-exercise-starter/src/array/iterator.rs",
        &missing_iterator,
        &["// pub(crate) struct ArrayIterator<'a, A: Array> {"],
    )
    .unwrap_err();
    assert!(missing_iterator_error.contains("ArrayIterator"));

    let leaked_iterator = iterator_source.replace(
        "// pub(crate) struct ArrayIterator",
        "pub(crate) struct ArrayIterator",
    );
    assert_ne!(iterator_source, leaked_iterator);
    let iterator_leakage_error = validate_solution_free_source(
        "type-exercise-starter/src/array/iterator.rs",
        &leaked_iterator,
    )
    .unwrap_err();
    assert!(iterator_leakage_error.contains("ArrayIterator"));
}
