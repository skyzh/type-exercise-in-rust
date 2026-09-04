use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

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
    /// Copy cumulative supplied tests through one visible checkpoint.
    CopyTest {
        #[arg(long)]
        chapter: usize,
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
    let number = name
        .strip_prefix("chapter_")?
        .strip_suffix(".rs")
        .unwrap_or_else(|| name.strip_prefix("chapter_").unwrap());
    (!number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| number.parse().ok())
        .flatten()
}

fn checkpoint_number(path: &Path) -> Option<usize> {
    let name = path.file_name()?.to_str()?;
    let number = name.strip_prefix("chapter-")?;
    (!number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| number.parse().ok())
        .flatten()
}

fn available_chapters(root: &Path) -> Result<Vec<usize>> {
    let checkpoint_dir = root.join("checkpoint");
    let mut chapters = fs::read_dir(&checkpoint_dir)
        .with_context(|| format!("failed to list {}", checkpoint_dir.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .filter(|path| path.is_dir())
        .filter_map(|path| checkpoint_number(&path))
        .collect::<Vec<_>>();
    chapters.sort_unstable();

    if chapters.is_empty() {
        bail!("no checkpoint tests are available");
    }
    for (index, chapter) in chapters.iter().copied().enumerate() {
        let expected = index + 1;
        if chapter != expected {
            bail!(
                "checkpoint sequence is not contiguous: expected chapter {expected}, found chapter {chapter}"
            );
        }
    }
    Ok(chapters)
}

fn copy_file_if_changed(source: &Path, target: &Path) -> Result<bool> {
    let source_bytes = fs::read(source)
        .with_context(|| format!("failed to read copy source {}", source.display()))?;
    if fs::read(target).ok().as_deref() == Some(source_bytes.as_slice()) {
        return Ok(false);
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::copy(source, target).with_context(|| {
        format!(
            "failed to copy {} to {}",
            source.display(),
            target.display()
        )
    })?;
    fs::File::options()
        .write(true)
        .open(target)
        .with_context(|| format!("failed to open copied target {}", target.display()))?
        .set_times(fs::FileTimes::new().set_modified(SystemTime::now()))
        .with_context(|| format!("failed to refresh copied target {}", target.display()))?;
    Ok(true)
}

fn remove_path(path: &Path) -> Result<usize> {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return Ok(0);
    };
    if metadata.is_dir() {
        let removed = fs::read_dir(path)
            .with_context(|| format!("failed to list {}", path.display()))?
            .count();
        fs::remove_dir_all(path)
            .with_context(|| format!("failed to remove directory {}", path.display()))?;
        Ok(removed)
    } else {
        fs::remove_file(path)
            .with_context(|| format!("failed to remove file {}", path.display()))?;
        Ok(1)
    }
}

fn copy_test(root: &Path, chapter: usize) -> Result<CopyReport> {
    let available = available_chapters(root)?;
    let last_chapter = *available
        .last()
        .context("no checkpoint tests are available")?;
    if chapter == 0 || chapter > last_chapter {
        bail!("no tests are available for chapter {chapter}");
    }

    let source_dir = root.join("type-exercise/supplied-tests/src");
    let target_dir = root.join("type-exercise-starter/supplied-tests/src");
    let mut specs = (1..=chapter)
        .map(|number| {
            (
                source_dir.join(format!("chapter_{number}.rs")),
                target_dir.join(format!("chapter_{number}.rs")),
            )
        })
        .collect::<Vec<_>>();
    specs.push((
        root.join(format!(
            "type-exercise/supplied-tests/roots/chapter_{chapter}.rs"
        )),
        target_dir.join("lib.rs"),
    ));
    for (source, _) in &specs {
        if !source.is_file() {
            bail!("copy source is not a file: {}", source.display());
        }
    }

    fs::create_dir_all(&target_dir).context("failed to create the starter test directory")?;
    let mut changed_files = 0;
    for entry in fs::read_dir(&target_dir).context("failed to list copied starter tests")? {
        let path = entry?.path();
        if chapter_number(&path).is_some() && !specs.iter().any(|(_, target)| target == &path) {
            changed_files += remove_path(&path)?;
        }
    }
    for (source, target) in &specs {
        changed_files += usize::from(copy_file_if_changed(source, target)?);
    }
    for (source, target) in specs {
        if fs::read(&source)? != fs::read(&target)? {
            bail!("copied test is not byte-identical: {}", target.display());
        }
    }

    println!(
        "copied cumulative Chapters 1-{chapter} tests into type-exercise-starter-supplied-tests"
    );
    Ok(CopyReport { changed_files })
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.action {
        Action::CopyTest { chapter } => {
            copy_test(&workspace_root()?, chapter)?;
        }
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
        fs::create_dir_all(root.path().join("checkpoint/chapter-01")).unwrap();
        fs::create_dir_all(root.path().join("type-exercise/supplied-tests/src")).unwrap();
        fs::create_dir_all(root.path().join("type-exercise/supplied-tests/roots")).unwrap();
        fs::create_dir_all(root.path().join("type-exercise-starter/supplied-tests/src")).unwrap();
        fs::write(
            root.path()
                .join("type-exercise/supplied-tests/src/chapter_1.rs"),
            b"// checkpoint one\n",
        )
        .unwrap();
        fs::write(
            root.path()
                .join("type-exercise/supplied-tests/roots/chapter_1.rs"),
            b"mod chapter_1;\n",
        )
        .unwrap();
        root
    }

    #[test]
    fn copies_only_visible_checkpoint_tests_byte_for_byte() {
        let root = fixture();
        assert_eq!(copy_test(root.path(), 1).unwrap().changed_files, 2);
        assert_eq!(copy_test(root.path(), 1).unwrap().changed_files, 0);
        assert_eq!(
            fs::read(
                root.path()
                    .join("type-exercise-starter/supplied-tests/src/chapter_1.rs")
            )
            .unwrap(),
            b"// checkpoint one\n"
        );
        assert!(copy_test(root.path(), 2).is_err());
    }

    #[test]
    fn removes_stale_course_tests_without_touching_learner_sources() {
        let root = fixture();
        let target = root.path().join("type-exercise-starter/supplied-tests/src");
        fs::write(target.join("chapter_2.rs"), b"// stale\n").unwrap();
        fs::create_dir_all(root.path().join("type-exercise-starter/expr/src")).unwrap();
        fs::write(
            root.path()
                .join("type-exercise-starter/expr/src/learner.rs"),
            b"// learner-owned\n",
        )
        .unwrap();

        copy_test(root.path(), 1).unwrap();
        assert!(!target.join("chapter_2.rs").exists());
        assert_eq!(
            fs::read(
                root.path()
                    .join("type-exercise-starter/expr/src/learner.rs")
            )
            .unwrap(),
            b"// learner-owned\n"
        );
    }
}
