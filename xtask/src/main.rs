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
    /// Copy cumulative supplied tests through one chapter into the starter.
    CopyTest {
        #[arg(long)]
        chapter: usize,
        /// Copy only the cumulative tests through one checkpoint of a staged chapter.
        #[arg(long)]
        checkpoint: Option<usize>,
    },
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

fn chapter_checkpoint_count(chapter: usize) -> Option<usize> {
    match chapter {
        1 => Some(4),
        6 => Some(3),
        7 => Some(3),
        8 => Some(2),
        9 => Some(3),
        10 => Some(3),
        _ => None,
    }
}

fn chapter_checkpoint_path(root: &Path, chapter: usize, checkpoint: usize) -> Result<PathBuf> {
    let checkpoint_count = chapter_checkpoint_count(chapter)
        .with_context(|| format!("--checkpoint is not available with --chapter {chapter}"))?;
    if !(1..=checkpoint_count).contains(&checkpoint) {
        bail!("no tests are available for Chapter {chapter} checkpoint {checkpoint}");
    }
    let path = if checkpoint == checkpoint_count {
        root.join(format!("type-exercise/src/tests/chapter_{chapter}.rs"))
    } else {
        root.join(format!("supplied-tests/chapter_{chapter}"))
            .join(format!("checkpoint_{checkpoint}.rs"))
    };
    Ok(path)
}

fn copy_test(root: &Path, chapter: usize, checkpoint: Option<usize>) -> Result<CopyReport> {
    let source_dir = root.join("type-exercise/src/tests");
    let available = available_chapters(&source_dir)?;
    let last_chapter = available
        .last()
        .copied()
        .context("no chapter tests are available")?;
    if chapter == 0 || chapter > last_chapter {
        bail!("no tests are available for chapter {chapter}");
    }
    if let Some(checkpoint) = checkpoint {
        chapter_checkpoint_path(root, chapter, checkpoint)?;
    }

    // Read the complete cumulative source set before writing any supplied tests.
    let sources = (1..=chapter)
        .map(|number| {
            let name = format!("chapter_{number}.rs");
            let path = if number == chapter {
                checkpoint
                    .map(|checkpoint| chapter_checkpoint_path(root, chapter, checkpoint))
                    .transpose()?
                    .unwrap_or_else(|| source_dir.join(&name))
            } else {
                source_dir.join(&name)
            };
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
            fs::remove_file(&path).with_context(|| {
                format!("failed to remove stale supplied test {}", path.display())
            })?;
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

    if let Some(checkpoint) = checkpoint {
        println!(
            "copied cumulative Chapter {chapter} checkpoint {checkpoint} tests into type-exercise-starter-expr"
        );
    } else {
        println!("copied cumulative Chapters 1-{chapter} tests into type-exercise-starter-expr");
    }
    Ok(CopyReport { changed_files })
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.action {
        Action::CopyTest {
            chapter,
            checkpoint,
        } => {
            copy_test(&workspace_root()?, chapter, checkpoint)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::{copy_test, workspace_root};

    fn fixture() -> TempDir {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("type-exercise/src/tests");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(root.path().join("supplied-tests/chapter_1")).unwrap();
        fs::create_dir_all(root.path().join("type-exercise-starter/src")).unwrap();
        fs::write(source.join("chapter_1.rs"), b"fn four() {}\n").unwrap();
        for checkpoint in 1..=3 {
            fs::write(
                root.path()
                    .join("supplied-tests/chapter_1")
                    .join(format!("checkpoint_{checkpoint}.rs")),
                format!("fn checkpoint_{checkpoint}() {{}}\n"),
            )
            .unwrap();
        }
        fs::write(source.join("chapter_2.rs"), b"// two\n").unwrap();
        fs::write(source.join("chapter_3.rs"), b"// three\n").unwrap();
        root
    }

    #[test]
    fn rejects_invalid_chapters_before_mutating_the_starter() {
        let root = fixture();
        for chapter in [0, 4, usize::MAX] {
            let error = copy_test(root.path(), chapter, None).unwrap_err();
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
    fn copies_an_exact_cumulative_prefix_and_removes_later_supplied_tests() {
        let root = fixture();
        copy_test(root.path(), 3, None).unwrap();

        for chapter in 1..=3 {
            let source = root
                .path()
                .join(format!("type-exercise/src/tests/chapter_{chapter}.rs"));
            let target = root.path().join(format!(
                "type-exercise-starter/src/tests/chapter_{chapter}.rs"
            ));
            assert_eq!(fs::read(target).unwrap(), fs::read(source).unwrap());
        }

        copy_test(root.path(), 2, None).unwrap();
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
    fn repeated_copy_is_byte_identical_and_does_not_rewrite_files() {
        let root = fixture();
        assert_eq!(copy_test(root.path(), 3, None).unwrap().changed_files, 4);
        assert_eq!(copy_test(root.path(), 3, None).unwrap().changed_files, 0);
        assert_eq!(
            fs::read(
                root.path()
                    .join("type-exercise-starter/src/tests/chapter_1.rs")
            )
            .unwrap(),
            fs::read(root.path().join("type-exercise/src/tests/chapter_1.rs")).unwrap()
        );
    }

    #[test]
    fn repairs_only_supplied_test_drift() {
        let root = fixture();
        let target = root.path().join("type-exercise-starter/src/tests");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("chapter_1.rs"), b"// drifted\n").unwrap();
        fs::write(
            root.path().join("type-exercise-starter/src/tests.rs"),
            b"mod stale;\n",
        )
        .unwrap();
        fs::write(
            root.path().join("type-exercise-starter/src/learner.rs"),
            b"// learner-owned\n",
        )
        .unwrap();

        let report = copy_test(root.path(), 2, None).unwrap();
        assert_eq!(report.changed_files, 3);
        assert_eq!(
            fs::read(target.join("chapter_1.rs")).unwrap(),
            fs::read(root.path().join("type-exercise/src/tests/chapter_1.rs")).unwrap()
        );
        assert_eq!(
            fs::read(target.join("chapter_2.rs")).unwrap(),
            fs::read(root.path().join("type-exercise/src/tests/chapter_2.rs")).unwrap()
        );
        assert_eq!(
            fs::read(root.path().join("type-exercise-starter/src/learner.rs")).unwrap(),
            b"// learner-owned\n"
        );
    }

    #[test]
    fn workspace_copy_target_is_available() {
        let root = workspace_root().unwrap();
        assert!(root.join("type-exercise/src/tests/chapter_1.rs").is_file());
    }
}
