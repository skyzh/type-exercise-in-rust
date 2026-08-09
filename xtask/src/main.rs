use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

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

fn check_course_contract(root: &Path) -> Result<()> {
    let available = available_chapters(&root.join("type-exercise/src/tests"))?;
    let expected = (1..=COURSE_CHAPTERS).collect::<Vec<_>>();
    if available != expected {
        bail!("course test sequence must be Chapters 1-{COURSE_CHAPTERS}, found {available:?}");
    }

    let modules = fs::read_to_string(root.join("type-exercise/src/tests.rs"))
        .context("failed to read the reference test module list")?;
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
    let workflow = fs::read_to_string(root.join(".github/workflows/ci.yml"))
        .context("failed to read the CI workflow")?;
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
    for chapter in expected {
        let page = &pages[&chapter];
        let body = fs::read_to_string(page)
            .with_context(|| format!("failed to read {}", page.display()))?;
        let copy_command = format!("cargo x copy-test --chapter {chapter}");
        let focused_command =
            format!("cargo test -p type-exercise-starter chapter_{chapter} --locked");
        if !body.contains(&copy_command) || !body.contains(&focused_command) {
            bail!("Chapter {chapter} must contain its copy command and focused starter test");
        }

        if workflow
            .lines()
            .filter(|line| line.trim() == copy_command)
            .count()
            != 1
        {
            bail!("CI must copy Chapter {chapter} exactly once");
        }
    }

    println!(
        "verified Chapters 1-{COURSE_CHAPTERS} across course pages, supplied tests, SUMMARY, and CI"
    );
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let root = workspace_root()?;
    match args.action {
        Action::CopyTest { chapter } => {
            copy_test(&root, chapter)?;
        }
        Action::CheckCourse => check_course_contract(&root)?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::copy_test;

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
}
