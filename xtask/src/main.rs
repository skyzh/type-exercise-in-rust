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
    /// Copy cumulative supplied tests through one chapter into the starter.
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
    let number = name.strip_prefix("chapter_")?;
    let number = number.strip_suffix(".rs").unwrap_or(number);
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

fn chapter_source(source_dir: &Path, chapter: usize) -> Result<PathBuf> {
    let file = source_dir.join(format!("chapter_{chapter}.rs"));
    let directory = source_dir.join(format!("chapter_{chapter}"));
    match (file.is_file(), directory.is_dir()) {
        (true, false) => Ok(file),
        (false, true) => Ok(directory),
        (true, true) => bail!("chapter {chapter} has both file and directory sources"),
        (false, false) => bail!("chapter {chapter} has no test source"),
    }
}

fn copy_specs(
    root: &Path,
    source_dir: &Path,
    target_dir: &Path,
    chapter: usize,
) -> Result<Vec<(PathBuf, PathBuf)>> {
    let mut specs = Vec::new();
    for number in 1..=chapter {
        let source = chapter_source(source_dir, number)?;
        if !source.is_file() {
            bail!("chapter {number} is not one chapter test file");
        }
        specs.push((source, target_dir.join(format!("chapter_{number}.rs"))));
    }
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
    Ok(specs)
}

fn copy_test(root: &Path, chapter: usize) -> Result<CopyReport> {
    let source_dir = root.join("type-exercise/supplied-tests/src");
    let available = available_chapters(&source_dir)?;
    let last_chapter = available
        .last()
        .copied()
        .context("no chapter tests are available")?;
    if chapter == 0 || chapter > last_chapter {
        bail!("no tests are available for chapter {chapter}");
    }
    let target_dir = root.join("type-exercise-starter/supplied-tests/src");
    let specs = copy_specs(root, &source_dir, &target_dir, chapter)?;
    fs::create_dir_all(&target_dir).context("failed to create the starter test directory")?;
    let mut changed_files = 0;

    // Remove stale chapter files/directories and old representations before copying.
    for entry in fs::read_dir(&target_dir).context("failed to list copied starter tests")? {
        let path = entry?.path();
        let Some(number) = chapter_number(&path) else {
            continue;
        };
        let expected = specs.iter().any(|(_, target)| target.starts_with(&path));
        if number > chapter || !expected {
            changed_files += remove_path(&path)?;
        }
    }

    for (source, target) in &specs {
        changed_files += usize::from(copy_file_if_changed(source, target)?);
    }

    for (source, target) in specs {
        let source_bytes = fs::read(&source)
            .with_context(|| format!("failed to verify copy source {}", source.display()))?;
        let target_bytes = fs::read(&target)
            .with_context(|| format!("failed to verify copied target {}", target.display()))?;
        if target_bytes != source_bytes {
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

    use super::{Args, copy_test, workspace_root};

    fn fixture() -> TempDir {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("type-exercise/supplied-tests/src");
        let roots = root.path().join("type-exercise/supplied-tests/roots");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&roots).unwrap();
        fs::create_dir_all(root.path().join("type-exercise-starter/supplied-tests")).unwrap();
        fs::create_dir_all(root.path().join("type-exercise-starter/expr/src")).unwrap();

        fs::write(source.join("chapter_1.rs"), b"// one\n").unwrap();
        fs::write(source.join("chapter_2.rs"), b"// two\n").unwrap();
        fs::write(source.join("chapter_3.rs"), b"// three\n").unwrap();
        for chapter in 1..=3 {
            fs::write(
                roots.join(format!("chapter_{chapter}.rs")),
                format!("// root through chapter {chapter}\n"),
            )
            .unwrap();
        }
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
        assert!(
            !root
                .path()
                .join("type-exercise-starter/supplied-tests/src")
                .exists()
        );
        assert!(
            !root
                .path()
                .join("type-exercise-starter/supplied-tests/src/lib.rs")
                .exists()
        );
    }

    #[test]
    fn copies_an_exact_cumulative_prefix_and_removes_later_supplied_tests() {
        let root = fixture();
        copy_test(root.path(), 3).unwrap();

        for chapter in 1..=3 {
            let source = root.path().join(format!(
                "type-exercise/supplied-tests/src/chapter_{chapter}.rs"
            ));
            let target = root.path().join(format!(
                "type-exercise-starter/supplied-tests/src/chapter_{chapter}.rs"
            ));
            assert_eq!(fs::read(target).unwrap(), fs::read(source).unwrap());
        }
        copy_test(root.path(), 2).unwrap();
        assert!(
            !root
                .path()
                .join("type-exercise-starter/supplied-tests/src/chapter_3.rs")
                .exists()
        );
        assert_eq!(
            fs::read(
                root.path()
                    .join("type-exercise-starter/supplied-tests/src/lib.rs")
            )
            .unwrap(),
            fs::read(
                root.path()
                    .join("type-exercise/supplied-tests/roots/chapter_2.rs")
            )
            .unwrap()
        );
    }

    #[test]
    fn rejects_obsolete_intra_chapter_checkpoint_selection() {
        use clap::Parser;

        assert!(
            Args::try_parse_from([
                "cargo-x",
                "copy-test",
                "--chapter",
                "1",
                "--checkpoint",
                "1",
            ])
            .is_err()
        );
    }

    #[test]
    fn repeated_copy_is_byte_identical_and_does_not_rewrite_files() {
        let root = fixture();
        assert_eq!(copy_test(root.path(), 3).unwrap().changed_files, 4);
        assert_eq!(copy_test(root.path(), 3).unwrap().changed_files, 0);
        assert_eq!(
            fs::read(
                root.path()
                    .join("type-exercise-starter/supplied-tests/src/chapter_1.rs")
            )
            .unwrap(),
            fs::read(
                root.path()
                    .join("type-exercise/supplied-tests/src/chapter_1.rs")
            )
            .unwrap()
        );
    }

    #[test]
    fn repairs_only_supplied_test_drift() {
        let root = fixture();
        let target = root.path().join("type-exercise-starter/supplied-tests/src");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("chapter_1.rs"), b"// drifted\n").unwrap();
        fs::write(target.join("chapter_3.rs"), b"// stale\n").unwrap();
        fs::write(
            root.path()
                .join("type-exercise-starter/supplied-tests/src/lib.rs"),
            b"mod stale;\n",
        )
        .unwrap();
        fs::write(
            root.path()
                .join("type-exercise-starter/expr/src/learner.rs"),
            b"// learner-owned\n",
        )
        .unwrap();

        let report = copy_test(root.path(), 2).unwrap();
        assert_eq!(report.changed_files, 4);
        assert_eq!(
            fs::read(target.join("chapter_1.rs")).unwrap(),
            fs::read(
                root.path()
                    .join("type-exercise/supplied-tests/src/chapter_1.rs")
            )
            .unwrap()
        );
        assert_eq!(
            fs::read(target.join("chapter_2.rs")).unwrap(),
            fs::read(
                root.path()
                    .join("type-exercise/supplied-tests/src/chapter_2.rs")
            )
            .unwrap()
        );
        assert_eq!(
            fs::read(
                root.path()
                    .join("type-exercise-starter/expr/src/learner.rs")
            )
            .unwrap(),
            b"// learner-owned\n"
        );
    }

    #[test]
    fn workspace_copy_target_is_available() {
        let root = workspace_root().unwrap();
        assert!(
            root.join("type-exercise/supplied-tests/src/chapter_1.rs")
                .is_file()
        );
    }

    #[test]
    fn chapter_progress_replaces_the_cumulative_root_without_stale_files() {
        let root = fixture();
        let target = root.path().join("type-exercise-starter/supplied-tests/src");

        copy_test(root.path(), 1).unwrap();
        assert!(target.join("chapter_1.rs").is_file());
        assert!(!target.join("chapter_2.rs").exists());

        copy_test(root.path(), 2).unwrap();
        assert!(target.join("chapter_1.rs").is_file());
        assert!(target.join("chapter_2.rs").is_file());
        assert_eq!(
            fs::read(target.join("lib.rs")).unwrap(),
            fs::read(
                root.path()
                    .join("type-exercise/supplied-tests/roots/chapter_2.rs")
            )
            .unwrap()
        );
    }
}
