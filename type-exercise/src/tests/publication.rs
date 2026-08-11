use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

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

const FIXTURE_THREADS: usize = 16;
static FIXTURE_NONCE: AtomicU64 = AtomicU64::new(0);
const SOCIAL_DESCRIPTION: &str = "A twelve-chapter Rust course on type families, generic expressions, checked binding, one-level Lists, and stronger Rust type boundaries.";
const SOCIAL_DESCRIPTION_SUFFIX: &str = "one-level Lists, and stronger Rust type boundaries.";

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("reference crate must be inside the workspace")
        .to_path_buf()
}

struct PublicationFixture {
    root: PathBuf,
}

impl PublicationFixture {
    fn copy_from(source_root: &Path) -> Self {
        let root = allocate_fixture_root();

        for relative in [
            "README.md",
            ".github/workflows/ci.yml",
            "course/theme/head.hbs",
            "course/src/SUMMARY.md",
            "course/src/sitemap.txt",
            "course/src/sitemap.xml",
            "type-exercise/src/tests.rs",
            "type-exercise-starter/README.md",
            "type-exercise-starter/AGENTS.md",
        ] {
            copy_fixture_file(source_root, &root, relative);
        }
        for &(chapter, file) in CHAPTERS {
            copy_fixture_file(source_root, &root, format!("course/src/{file}"));
            copy_fixture_file(
                source_root,
                &root,
                format!("type-exercise/src/tests/chapter_{chapter}.rs"),
            );
        }

        Self { root }
    }
}

fn allocate_fixture_root() -> PathBuf {
    loop {
        let nonce = FIXTURE_NONCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "type-exercise-publication-{}-{nonce}",
            std::process::id()
        ));
        match fs::create_dir(&root) {
            Ok(()) => return root,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!(
                "failed to allocate publication fixture {}: {error}",
                root.display()
            ),
        }
    }
}

impl Drop for PublicationFixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap_or_else(|error| {
            panic!(
                "failed to remove publication fixture {}: {error}",
                self.root.display()
            )
        });
    }
}

fn copy_fixture_file(source_root: &Path, fixture_root: &Path, relative: impl AsRef<Path>) {
    let relative = relative.as_ref();
    let destination = fixture_root.join(relative);
    fs::create_dir_all(
        destination
            .parent()
            .expect("fixture file must have a parent directory"),
    )
    .unwrap_or_else(|error| {
        panic!(
            "failed to create fixture parent for {}: {error}",
            destination.display()
        )
    });
    fs::copy(source_root.join(relative), &destination).unwrap_or_else(|error| {
        panic!(
            "failed to copy fixture file {}: {error}",
            relative.display()
        )
    });
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
    root_readme: &str,
    chapter_2: &str,
    chapter_3: &str,
    starter_readme: &str,
    starter_agents: &str,
) -> Result<(), String> {
    let required = [
        (
            "README.md",
            root_readme,
            [
                "currently published Chapters 1–12 build these outcomes",
                "batch-level asynchronous adapter is reserved for future, non-required Day 13 work",
                "It is not part of the currently published course",
            ]
            .as_slice(),
        ),
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
            "README.md",
            root_readme,
            "fast path and a batch-level\n  asynchronous adapter",
        ),
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

fn validate_description_tag(theme_head: &str, key: &str, expected_tag: &str) -> Result<(), String> {
    if count(theme_head, key) != 1 {
        return Err(format!(
            "course/theme/head.hbs must contain exactly one {key} key"
        ));
    }
    let keyed_tag = theme_head
        .lines()
        .find(|line| line.contains(key))
        .expect("one description key must have a containing line");
    if keyed_tag.trim() != expected_tag {
        return Err(format!(
            "course/theme/head.hbs {key} must equal {expected_tag:?}"
        ));
    }

    Ok(())
}

fn validate_social_metadata(theme_head: &str) -> Result<(), String> {
    validate_description_tag(
        theme_head,
        r#"property="og:description""#,
        &format!(r#"<meta property="og:description" content="{SOCIAL_DESCRIPTION}">"#),
    )?;
    validate_description_tag(
        theme_head,
        r#"name="twitter:description""#,
        &format!(r#"<meta name="twitter:description" content="{SOCIAL_DESCRIPTION}">"#),
    )?;
    Ok(())
}

fn validate_publication_sync(root: &Path) -> Result<(), String> {
    let root_readme = read(root.join("README.md"));
    let summary = read(root.join("course/src/SUMMARY.md"));
    let reference_manifest = read(root.join("type-exercise/src/tests.rs"));
    let workflow = read(root.join(".github/workflows/ci.yml"));
    let sitemap_text = read(root.join("course/src/sitemap.txt"));
    let sitemap_xml = read(root.join("course/src/sitemap.xml"));
    let chapter_2 = read(root.join("course/src/chapter-2-type-catalog.md"));
    let chapter_3 = read(root.join("course/src/chapter-3-column-views.md"));
    let starter_readme = read(root.join("type-exercise-starter/README.md"));
    let starter_agents = read(root.join("type-exercise-starter/AGENTS.md"));
    let theme_head = read(root.join("course/theme/head.hbs"));

    validate_summary(&summary)?;
    validate_public_contract_text(
        &root_readme,
        &chapter_2,
        &chapter_3,
        &starter_readme,
        &starter_agents,
    )?;
    validate_social_metadata(&theme_head)?;
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
    let root_readme = read(root.join("README.md"));
    let chapter_2 = read(root.join("course/src/chapter-2-type-catalog.md"));
    let chapter_3 = read(root.join("course/src/chapter-3-column-views.md"));
    let starter_readme = read(root.join("type-exercise-starter/README.md"));
    let starter_agents = read(root.join("type-exercise-starter/AGENTS.md"));

    let stale_list = chapter_3.replace(
        "same representation boundary in Chapter 11.",
        "same representation boundary in Chapter 10.",
    );
    assert_ne!(chapter_3, stale_list);
    let list_error = validate_public_contract_text(
        &root_readme,
        &chapter_2,
        &stale_list,
        &starter_readme,
        &starter_agents,
    )
    .unwrap_err();
    assert!(list_error.contains("Chapter 11"));

    let stale_chapter_2 = chapter_2.replace(
        "published course currently requires Chapters 1–12",
        "starter source names every later target through Day 13",
    );
    assert_ne!(chapter_2, stale_chapter_2);
    let chapter_2_error = validate_public_contract_text(
        &root_readme,
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
    let readme_error = validate_public_contract_text(
        &root_readme,
        &chapter_2,
        &chapter_3,
        &stale_readme,
        &starter_agents,
    )
    .unwrap_err();
    assert!(readme_error.contains("Day 1–12 checkpoints"));

    let stale_agents = starter_agents.replace(
        "Published and required Day 1–12 ownership",
        "Day 1–13 ownership",
    );
    assert_ne!(starter_agents, stale_agents);
    let agents_error = validate_public_contract_text(
        &root_readme,
        &chapter_2,
        &chapter_3,
        &starter_readme,
        &stale_agents,
    )
    .unwrap_err();
    assert!(agents_error.contains("Day 1–12 ownership"));

    let stale_root_readme = root_readme.replace(
        "A batch-level asynchronous adapter is reserved for future, non-required Day 13 work",
        "Preserve the same results and errors through one representative fast path and a batch-level\n  asynchronous adapter",
    );
    assert_ne!(root_readme, stale_root_readme);
    let root_readme_error = validate_public_contract_text(
        &stale_root_readme,
        &chapter_2,
        &chapter_3,
        &starter_readme,
        &starter_agents,
    )
    .unwrap_err();
    assert!(root_readme_error.contains("README.md"));
    assert!(root_readme_error.contains("reserved for future"));
}

#[test]
fn publication_sync_top_level_guard_rejects_root_readme_drift() {
    let fixture = PublicationFixture::copy_from(&workspace_root());
    let readme_path = fixture.root.join("README.md");
    let readme = read(&readme_path);
    let stale = readme.replace(
        "is reserved for future, non-required Day 13 work.\nIt is not part of the currently published course",
        "is part of the currently published course",
    );
    assert_ne!(readme, stale);
    fs::write(&readme_path, stale)
        .unwrap_or_else(|error| panic!("failed to mutate {}: {error}", readme_path.display()));

    let error = validate_publication_sync(&fixture.root).unwrap_err();
    assert!(error.contains("README.md"));
    assert!(error.contains("reserved for future"));
}

fn mutate_fixture_theme(replace: impl FnOnce(&str) -> String) -> Result<(), String> {
    let fixture = PublicationFixture::copy_from(&workspace_root());
    let theme_path = fixture.root.join("course/theme/head.hbs");
    let theme = read(&theme_path);
    let drifted = replace(&theme);
    assert_ne!(theme, drifted);
    fs::write(&theme_path, drifted)
        .unwrap_or_else(|error| panic!("failed to mutate {}: {error}", theme_path.display()));

    validate_publication_sync(&fixture.root)
}

#[test]
fn publication_sync_top_level_guard_rejects_og_metadata_drift() {
    let error = mutate_fixture_theme(|theme| {
        theme.replacen(
            "one-level Lists, and stronger Rust type boundaries.",
            "one-level Lists, and batch futures.",
            1,
        )
    })
    .unwrap_err();
    assert!(error.contains(r#"property="og:description""#));
    assert!(error.contains("stronger Rust type boundaries"));
}

#[test]
fn publication_sync_top_level_guard_rejects_twitter_metadata_drift() {
    let error = mutate_fixture_theme(|theme| {
        let description_offset = theme
            .rfind("one-level Lists, and stronger Rust type boundaries.")
            .expect("Twitter description must exist");
        let mut drifted = theme.to_owned();
        drifted.replace_range(
            description_offset..description_offset + SOCIAL_DESCRIPTION_SUFFIX.len(),
            "one-level Lists, and batch futures.",
        );
        drifted
    })
    .unwrap_err();
    assert!(error.contains(r#"name="twitter:description""#));
    assert!(error.contains("stronger Rust type boundaries"));
}

#[test]
fn publication_sync_top_level_guard_rejects_duplicate_stale_og_tag() {
    let error = mutate_fixture_theme(|theme| {
        theme.replace(
            r#"<meta property="og:description" content="#,
            concat!(
                r#"<meta property="og:description" content="stale batch futures">"#,
                "\n",
                r#"<meta property="og:description" content="#,
            ),
        )
    })
    .unwrap_err();
    assert!(error.contains("exactly one"));
    assert!(error.contains(r#"property="og:description""#));
}

#[test]
fn publication_sync_top_level_guard_rejects_duplicate_stale_twitter_tag() {
    let error = mutate_fixture_theme(|theme| {
        theme.replace(
            r#"<meta name="twitter:description" content="#,
            concat!(
                r#"<meta name="twitter:description" content="stale batch futures">"#,
                "\n",
                r#"<meta name="twitter:description" content="#,
            ),
        )
    })
    .unwrap_err();
    assert!(error.contains("exactly one"));
    assert!(error.contains(r#"name="twitter:description""#));
}

#[test]
fn publication_fixture_allocation_is_collision_proof_under_concurrency() {
    let start = std::sync::Arc::new(std::sync::Barrier::new(FIXTURE_THREADS));
    let source_root = std::sync::Arc::new(workspace_root());
    let roots = std::thread::scope(|scope| {
        let handles = (0..FIXTURE_THREADS)
            .map(|_| {
                let start = std::sync::Arc::clone(&start);
                let source_root = std::sync::Arc::clone(&source_root);
                scope.spawn(move || {
                    start.wait();
                    let fixture = PublicationFixture::copy_from(&source_root);
                    validate_publication_sync(&fixture.root).unwrap();
                    fixture.root.clone()
                })
            })
            .collect::<Vec<_>>();

        handles
            .into_iter()
            .map(|handle| handle.join().expect("fixture worker must not panic"))
            .collect::<BTreeSet<_>>()
    });

    assert_eq!(roots.len(), FIXTURE_THREADS);
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
