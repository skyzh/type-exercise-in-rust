use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde::Deserialize;

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

#[derive(Debug, Deserialize)]
struct ApiManifest {
    version: u8,
    target: Vec<ApiTarget>,
}

#[derive(Debug, Deserialize)]
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

fn check_starter_api(root: &Path) -> Result<()> {
    let starter = root.join("type-exercise-starter");
    let manifest_path = starter.join("api-roadmap.toml");
    let manifest_text = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;
    let manifest: ApiManifest = toml::from_str(&manifest_text)
        .with_context(|| format!("failed to parse {}", manifest_path.display()))?;
    if manifest.version != 1 {
        bail!(
            "unsupported starter API manifest version {}",
            manifest.version
        );
    }

    let roadmap_path = starter.join("API_ROADMAP.md");
    let roadmap = fs::read_to_string(&roadmap_path)
        .with_context(|| format!("failed to read {}", roadmap_path.display()))?;
    let mut seen = std::collections::BTreeSet::new();
    let mut days = std::collections::BTreeSet::new();
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
        days.insert(target.day);

        let roadmap_marker = format!("| {} | `{}` | {} |", target.day, target.file, target.title);
        if !roadmap.contains(&roadmap_marker) {
            bail!("roadmap is missing manifest row: {roadmap_marker}");
        }

        let source_path = starter.join(&target.file);
        let source = fs::read_to_string(&source_path).ok();
        for (item, declaration) in target.items.iter().zip(&target.declarations) {
            let item_marker = format!("`{item}`");
            let declaration_marker = format!("`{declaration}`");
            if !roadmap.contains(&item_marker) || !roadmap.contains(&declaration_marker) {
                bail!("roadmap is missing `{item}` or its declaration shape `{declaration}`");
            }
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

    let expected_days = (1..=13).collect::<std::collections::BTreeSet<_>>();
    if days != expected_days {
        bail!("starter API roadmap must cover every Day 1-13 exactly as a complete set");
    }
    if manifest
        .target
        .iter()
        .any(|target| target.materialized && target.day > 2)
    {
        bail!("only cumulative Days 1-2 starter targets may be materialized on PR #45");
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

    use super::{check_starter_api, copy_test, workspace_root};

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
}
