use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde::Deserialize;

mod starter_api_contract;

use starter_api_contract::{APPROVED_TARGETS, ApprovedTarget};

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
    /// Verify the learner-visible API roadmap against the cumulative starter.
    CheckStarterApi,
}

#[derive(Clone, Debug, Deserialize)]
struct ApiManifest {
    version: u8,
    target: Vec<ApiTarget>,
}

#[derive(Clone, Debug, Deserialize)]
struct ApiTarget {
    day: usize,
    title: String,
    file: String,
    items: Vec<String>,
    declarations: Vec<String>,
    materialized: bool,
}

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

fn validate_manifest_against_approved(manifest: &ApiManifest) -> Result<()> {
    if manifest.version != 1 {
        bail!(
            "unsupported starter API manifest version {}",
            manifest.version
        );
    }
    if manifest.target.len() != APPROVED_TARGETS.len() {
        bail!(
            "starter API manifest has {} targets; the approved source ledger has {}",
            manifest.target.len(),
            APPROVED_TARGETS.len()
        );
    }

    for (index, (actual, approved)) in manifest.target.iter().zip(APPROVED_TARGETS).enumerate() {
        if actual.day != approved.day
            || actual.title != approved.title
            || actual.file != approved.file
            || actual.items.iter().map(String::as_str).collect::<Vec<_>>() != approved.items
            || actual
                .declarations
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                != approved.declarations
            || actual.materialized != approved.materialized
        {
            bail!(
                "starter API target {} disagrees with the approved source ledger entry from {}: Day {} {} {}",
                index + 1,
                approved.source,
                approved.day,
                approved.file,
                approved.title
            );
        }
    }
    Ok(())
}

fn roadmap_row(target: &ApprovedTarget) -> String {
    let items = target
        .items
        .iter()
        .zip(target.declarations)
        .map(|(item, declaration)| format!("`{item}` → `{declaration}`"))
        .collect::<Vec<_>>()
        .join("; ");
    format!(
        "| {} | `{}` | {} | {} |",
        target.day, target.file, target.title, items
    )
}

fn validate_roadmap_against_approved(roadmap: &str) -> Result<()> {
    let actual = roadmap
        .lines()
        .filter(|line| {
            line.strip_prefix("| ")
                .and_then(|rest| rest.chars().next())
                .is_some_and(|character| character.is_ascii_digit())
        })
        .collect::<Vec<_>>();
    let expected = APPROVED_TARGETS.iter().map(roadmap_row).collect::<Vec<_>>();
    if actual.len() != expected.len() {
        bail!(
            "starter API roadmap has {} target rows; the approved source ledger has {}",
            actual.len(),
            expected.len()
        );
    }
    for (index, (actual, expected)) in actual.iter().zip(&expected).enumerate() {
        if *actual != expected {
            bail!(
                "starter API roadmap row {} disagrees with the approved source ledger\nexpected: {}\nactual:   {}",
                index + 1,
                expected,
                actual
            );
        }
    }
    Ok(())
}

fn struct_fields<'a>(source: &'a str, name: &str) -> Result<Vec<&'a str>> {
    let header = format!("pub struct {name} {{");
    let body = source
        .split_once(&header)
        .map(|(_, rest)| rest)
        .with_context(|| format!("Decimal storage source is missing `{header}`"))?;
    let body = body
        .split_once('}')
        .map(|(fields, _)| fields)
        .with_context(|| format!("Decimal storage source has no closing brace for `{name}`"))?;
    Ok(body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .map(|line| line.strip_suffix(',').unwrap_or(line))
        .collect())
}

fn validate_decimal_storage_source(source: &str) -> Result<()> {
    let expected = [
        "values: Vec<i128>",
        "validity: BitVec",
        "decimal_type: DecimalType",
        "null_count: usize",
    ];
    for name in ["DecimalArray", "DecimalArrayBuilder"] {
        let actual = struct_fields(source, name)?;
        if actual != expected {
            bail!(
                "{name} must contain exactly one flat i128 buffer, packed BitVec validity, one shared DecimalType, and the derived null count; found {actual:?}"
            );
        }
    }
    Ok(())
}

fn check_starter_api(root: &Path) -> Result<()> {
    let starter = root.join("type-exercise-starter");
    let manifest_path = starter.join("api-roadmap.toml");
    let manifest_text = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;
    let manifest: ApiManifest = toml::from_str(&manifest_text)
        .with_context(|| format!("failed to parse {}", manifest_path.display()))?;
    validate_manifest_against_approved(&manifest)?;

    let roadmap_path = starter.join("API_ROADMAP.md");
    let roadmap = fs::read_to_string(&roadmap_path)
        .with_context(|| format!("failed to read {}", roadmap_path.display()))?;
    validate_roadmap_against_approved(&roadmap)?;
    let mut seen = std::collections::BTreeSet::new();
    for target in &manifest.target {
        if !(1..=13).contains(&target.day) {
            bail!("starter API target has invalid Day {}", target.day);
        }
        if target.items.is_empty() || target.items.len() != target.declarations.len() {
            bail!(
                "starter API target Day {} {} must pair every item with one declaration",
                target.day,
                target.file
            );
        }
        if !seen.insert((target.day, target.file.clone(), target.title.clone())) {
            bail!(
                "duplicate starter API target: Day {} {} {}",
                target.day,
                target.file,
                target.title
            );
        }
        let source_path = starter.join(&target.file);
        let source = fs::read_to_string(&source_path).ok();
        for (item, declaration) in target.items.iter().zip(&target.declarations) {
            match (&source, target.materialized) {
                (Some(source), true) if !source.contains(declaration) => bail!(
                    "materialized starter target `{item}` is missing `{declaration}` in {}",
                    source_path.display()
                ),
                (Some(source), false) if source.contains(declaration) => bail!(
                    "future starter target `{item}` was materialized before Day {} in {}",
                    target.day,
                    source_path.display()
                ),
                (None, true) => bail!(
                    "materialized starter target file is missing: {}",
                    source_path.display()
                ),
                _ => {}
            }
        }
    }

    let reference_manifest = fs::read_to_string(root.join("type-exercise/Cargo.toml"))?;
    if reference_manifest.contains("rust_decimal") {
        bail!("reference production dependencies must not reintroduce rust_decimal");
    }
    let reference_catalog = fs::read_to_string(root.join("type-exercise/src/variant_catalog.rs"))?;
    if !reference_catalog.contains("{ decimal, Decimal")
        || reference_catalog.contains("{ copy, Decimal")
    {
        bail!("Decimal must remain a dedicated catalog row, never a copy primitive row");
    }
    let decimal_storage =
        fs::read_to_string(root.join("type-exercise/src/array/decimal_array.rs"))?;
    validate_decimal_storage_source(&decimal_storage)?;

    println!("starter API roadmap matches cumulative Days 1-2 and reserves Days 3-13");
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let root = workspace_root()?;
    match args.action {
        Action::CopyTest { chapter } => {
            copy_test(&root, chapter)?;
        }
        Action::CheckStarterApi => check_starter_api(&root)?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::{
        ApiManifest, check_starter_api, copy_test, validate_decimal_storage_source,
        validate_manifest_against_approved, workspace_root,
    };

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

    #[test]
    fn workspace_starter_api_manifest_matches_the_cumulative_scaffold() {
        check_starter_api(&workspace_root().unwrap()).unwrap();
    }

    fn workspace_manifest() -> ApiManifest {
        let root = workspace_root().unwrap();
        let source =
            fs::read_to_string(root.join("type-exercise-starter/api-roadmap.toml")).unwrap();
        toml::from_str(&source).unwrap()
    }

    #[test]
    fn approved_api_ledger_rejects_coordinated_semantic_drift() {
        let manifest = workspace_manifest();

        let mut missing = manifest.clone();
        missing.target.remove(
            missing
                .target
                .iter()
                .position(|target| target.title == "Thread-safe erased expression boundary")
                .unwrap(),
        );
        assert!(validate_manifest_against_approved(&missing).is_err());

        let mut swapped = manifest.clone();
        let day_12 = swapped
            .target
            .iter()
            .position(|target| target.title == "Private concrete array iterator")
            .unwrap();
        let day_13 = swapped
            .target
            .iter()
            .position(|target| target.title == "Static and erased batch futures")
            .unwrap();
        swapped.target[day_12].day = 13;
        swapped.target[day_13].day = 12;
        assert!(validate_manifest_against_approved(&swapped).is_err());

        let mut duplicated = manifest.clone();
        let duplicate_item = duplicated.target[0].items[1].clone();
        let duplicate_declaration = duplicated.target[0].declarations[1].clone();
        duplicated.target[0].items.push(duplicate_item);
        duplicated.target[0]
            .declarations
            .push(duplicate_declaration);
        assert!(validate_manifest_against_approved(&duplicated).is_err());

        let mut wrong_file = manifest.clone();
        wrong_file
            .target
            .iter_mut()
            .find(|target| {
                target
                    .items
                    .iter()
                    .any(|item| item == "BoundExpression::evaluate_async")
            })
            .unwrap()
            .file = "src/expression.rs".to_owned();
        assert!(validate_manifest_against_approved(&wrong_file).is_err());

        let mut leaked = manifest;
        leaked
            .target
            .iter_mut()
            .find(|target| target.day == 3)
            .unwrap()
            .materialized = true;
        assert!(validate_manifest_against_approved(&leaked).is_err());
    }

    #[test]
    fn decimal_storage_layout_rejects_shadow_or_widened_row_state() {
        let root = workspace_root().unwrap();
        let source =
            fs::read_to_string(root.join("type-exercise/src/array/decimal_array.rs")).unwrap();
        validate_decimal_storage_source(&source).unwrap();

        for forbidden in [
            "    _row_types: Vec<DecimalType>,\n",
            "    values: Vec<Option<Decimal>>,\n",
            "    validity: Vec<bool>,\n",
        ] {
            let mutated = if forbidden.contains("_row_types") {
                source.replacen(
                    "    validity: BitVec,\n",
                    &format!("    validity: BitVec,\n{forbidden}"),
                    1,
                )
            } else if forbidden.contains("values:") {
                source.replacen("    values: Vec<i128>,\n", forbidden, 1)
            } else {
                source.replacen("    validity: BitVec,\n", forbidden, 1)
            };
            assert!(validate_decimal_storage_source(&mutated).is_err());
        }
    }
}
