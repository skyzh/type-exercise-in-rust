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
        /// Copy only the cumulative tests through one checkpoint of a staged chapter.
        #[arg(long)]
        checkpoint: Option<usize>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CopyReport {
    changed_files: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PathKind {
    Missing,
    File,
    Directory,
    Symlink,
    Other,
}

fn path_kind(path: &Path) -> Result<PathKind> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(PathKind::Missing),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to inspect {}", path.display()));
        }
    };
    let file_type = metadata.file_type();
    Ok(if file_type.is_symlink() {
        PathKind::Symlink
    } else if file_type.is_file() {
        PathKind::File
    } else if file_type.is_dir() {
        PathKind::Directory
    } else {
        PathKind::Other
    })
}

fn require_directory(path: &Path, label: &str) -> Result<()> {
    match path_kind(path)? {
        PathKind::Directory => Ok(()),
        kind => bail!(
            "{label} must be a real directory, found {kind:?}: {}",
            path.display()
        ),
    }
}

fn require_file(path: &Path, label: &str) -> Result<()> {
    match path_kind(path)? {
        PathKind::File => Ok(()),
        kind => bail!(
            "{label} must be a regular file, found {kind:?}: {}",
            path.display()
        ),
    }
}

fn require_file_or_missing(path: &Path, label: &str) -> Result<()> {
    match path_kind(path)? {
        PathKind::File | PathKind::Missing => Ok(()),
        kind => bail!(
            "{label} must be a regular file or absent, found {kind:?}: {}",
            path.display()
        ),
    }
}

fn require_directory_chain(root: &Path, components: &[&str], label: &str) -> Result<PathBuf> {
    let mut path = root.to_path_buf();
    for component in components {
        path.push(component);
        require_directory(&path, label)?;
    }
    Ok(path)
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
    require_directory(source_dir, "chapter source root")?;
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

fn chapter_checkpoint_count(chapter: usize) -> Option<usize> {
    match chapter {
        1 => Some(4),
        6 => Some(3),
        7 => Some(3),
        8 => Some(2),
        9 => Some(3),
        10 => Some(3),
        11 => Some(2),
        12 => Some(3),
        _ => None,
    }
}

fn chapter_checkpoint_root(root: &Path, chapter: usize, checkpoint: usize) -> Result<PathBuf> {
    let checkpoint_count = chapter_checkpoint_count(chapter)
        .with_context(|| format!("--checkpoint is not available with --chapter {chapter}"))?;
    if !(1..=checkpoint_count).contains(&checkpoint) {
        bail!("no tests are available for Chapter {chapter} checkpoint {checkpoint}");
    }
    let path = if checkpoint == checkpoint_count {
        root.join(format!(
            "type-exercise/supplied-tests/src/chapter_{chapter}/mod.rs"
        ))
    } else {
        let checkpoints = root.join("type-exercise/supplied-tests/checkpoints");
        require_directory(&checkpoints, "checkpoint source root")?;
        let chapter_root = checkpoints.join(format!("chapter_{chapter}"));
        require_directory(&chapter_root, "checkpoint chapter root")?;
        chapter_root.join(format!("checkpoint_{checkpoint}.rs"))
    };
    Ok(path)
}

fn chapter_source(source_dir: &Path, chapter: usize) -> Result<PathBuf> {
    let file = source_dir.join(format!("chapter_{chapter}.rs"));
    let directory = source_dir.join(format!("chapter_{chapter}"));
    let file_kind = path_kind(&file)?;
    let directory_kind = path_kind(&directory)?;
    if !matches!(file_kind, PathKind::Missing | PathKind::File) {
        bail!(
            "chapter {chapter} file source has an unexpected type {file_kind:?}: {}",
            file.display()
        );
    }
    if !matches!(directory_kind, PathKind::Missing | PathKind::Directory) {
        bail!(
            "chapter {chapter} directory source has an unexpected type {directory_kind:?}: {}",
            directory.display()
        );
    }
    match (file_kind, directory_kind) {
        (PathKind::File, PathKind::Missing) => Ok(file),
        (PathKind::Missing, PathKind::Directory) => Ok(directory),
        (PathKind::File, PathKind::Directory) => {
            bail!("chapter {chapter} has both file and directory sources")
        }
        (PathKind::Missing, PathKind::Missing) => bail!("chapter {chapter} has no test source"),
        _ => unreachable!("source kinds were validated above"),
    }
}

fn preflight_destination(target_dir: &Path, specs: &[(PathBuf, PathBuf)]) -> Result<()> {
    let Some(target_parent) = target_dir.parent() else {
        bail!(
            "managed destination root has no parent: {}",
            target_dir.display()
        );
    };
    require_directory(target_parent, "managed destination parent")?;
    match path_kind(target_dir)? {
        PathKind::Missing => {}
        PathKind::Directory => {
            for entry in fs::read_dir(target_dir)
                .with_context(|| format!("failed to list {}", target_dir.display()))?
            {
                let path = entry?.path();
                let Some(_) = chapter_number(&path) else {
                    continue;
                };
                let is_file_representation = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with(".rs"));
                let kind = path_kind(&path)?;
                if is_file_representation {
                    if kind != PathKind::File {
                        bail!(
                            "managed destination chapter file has an unexpected type {kind:?}: {}",
                            path.display()
                        );
                    }
                    continue;
                }
                if kind != PathKind::Directory {
                    bail!(
                        "managed destination chapter directory has an unexpected type {kind:?}: {}",
                        path.display()
                    );
                }
                for child in fs::read_dir(&path)
                    .with_context(|| format!("failed to list {}", path.display()))?
                {
                    require_file_or_missing(&child?.path(), "managed destination chapter leaf")?;
                }
            }
        }
        kind => bail!(
            "managed destination root must be a real directory or absent, found {kind:?}: {}",
            target_dir.display()
        ),
    }

    for (_, target) in specs {
        let Some(parent) = target.parent() else {
            bail!("copy target has no parent: {}", target.display());
        };
        if parent != target_dir {
            match path_kind(parent)? {
                PathKind::Directory | PathKind::Missing => {}
                kind => bail!(
                    "managed destination chapter directory has an unexpected type {kind:?}: {}",
                    parent.display()
                ),
            }
        }
        require_file_or_missing(target, "managed destination leaf")?;
    }
    Ok(())
}

fn copy_specs(
    root: &Path,
    source_dir: &Path,
    target_dir: &Path,
    chapter: usize,
    checkpoint: Option<usize>,
) -> Result<Vec<(PathBuf, PathBuf)>> {
    require_directory(source_dir, "chapter source root")?;
    let learner_roots = root.join("type-exercise/supplied-tests/roots");
    require_directory(&learner_roots, "learner root source directory")?;
    let mut specs = Vec::new();
    for number in 1..=chapter {
        let source = chapter_source(source_dir, number)?;
        if source.is_file() {
            specs.push((source, target_dir.join(format!("chapter_{number}.rs"))));
            continue;
        }

        let checkpoint_count = chapter_checkpoint_count(number)
            .with_context(|| format!("chapter {number} directory has no checkpoint count"))?;
        let copied_count = if number == chapter {
            checkpoint.unwrap_or(checkpoint_count)
        } else {
            checkpoint_count
        };
        let root_source = if number == chapter && checkpoint.is_some() {
            chapter_checkpoint_root(root, number, copied_count)?
        } else {
            source.join("mod.rs")
        };
        let target = target_dir.join(format!("chapter_{number}"));
        specs.push((root_source, target.join("mod.rs")));
        for checkpoint in 1..=copied_count {
            specs.push((
                source.join(format!("checkpoint_{checkpoint}.rs")),
                target.join(format!("checkpoint_{checkpoint}.rs")),
            ));
        }
    }
    specs.push((
        learner_roots.join(format!("chapter_{chapter}.rs")),
        target_dir.join("lib.rs"),
    ));
    for (source, _) in &specs {
        require_file(source, "copy source")?;
    }
    Ok(specs)
}

fn copy_test(root: &Path, chapter: usize, checkpoint: Option<usize>) -> Result<CopyReport> {
    let source_dir = require_directory_chain(
        root,
        &["type-exercise", "supplied-tests", "src"],
        "chapter source path component",
    )?;
    let target_parent = require_directory_chain(
        root,
        &["type-exercise-starter", "supplied-tests"],
        "managed destination path component",
    )?;
    let available = available_chapters(&source_dir)?;
    let last_chapter = available
        .last()
        .copied()
        .context("no chapter tests are available")?;
    if chapter == 0 || chapter > last_chapter {
        bail!("no tests are available for chapter {chapter}");
    }
    if let Some(checkpoint) = checkpoint {
        chapter_checkpoint_root(root, chapter, checkpoint)?;
    }

    let target_dir = target_parent.join("src");
    let specs = copy_specs(root, &source_dir, &target_dir, chapter, checkpoint)?;
    preflight_destination(&target_dir, &specs)?;
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

    // Inside retained chapter directories, remove module files beyond the selected root.
    for entry in fs::read_dir(&target_dir).context("failed to list copied starter tests")? {
        let path = entry?.path();
        if path_kind(&path)? != PathKind::Directory || chapter_number(&path).is_none() {
            continue;
        }
        for child in
            fs::read_dir(&path).with_context(|| format!("failed to list {}", path.display()))?
        {
            let child = child?.path();
            if !specs.iter().any(|(_, target)| target == &child) {
                changed_files += remove_path(&child)?;
            }
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

    if let Some(checkpoint) = checkpoint {
        println!(
            "copied cumulative Chapter {chapter} checkpoint {checkpoint} tests into type-exercise-starter-supplied-tests"
        );
    } else {
        println!(
            "copied cumulative Chapters 1-{chapter} tests into type-exercise-starter-supplied-tests"
        );
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
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{Duration, SystemTime};

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    use tempfile::TempDir;

    use super::{copy_test, workspace_root};

    #[derive(Debug, Eq, PartialEq)]
    struct SnapshotEntry {
        path: String,
        kind: &'static str,
        data: Vec<u8>,
    }

    fn snapshot(root: &Path) -> Vec<SnapshotEntry> {
        fn visit(base: &Path, path: &Path, entries: &mut Vec<SnapshotEntry>) {
            let metadata = fs::symlink_metadata(path).unwrap();
            let relative = path
                .strip_prefix(base)
                .unwrap_or_else(|_| Path::new(""))
                .to_string_lossy()
                .into_owned();
            let file_type = metadata.file_type();
            if file_type.is_symlink() {
                entries.push(SnapshotEntry {
                    path: relative,
                    kind: "symlink",
                    data: fs::read_link(path)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned()
                        .into_bytes(),
                });
            } else if file_type.is_file() {
                entries.push(SnapshotEntry {
                    path: relative,
                    kind: "file",
                    data: fs::read(path).unwrap(),
                });
            } else if file_type.is_dir() {
                entries.push(SnapshotEntry {
                    path: relative,
                    kind: "directory",
                    data: Vec::new(),
                });
                let mut children = fs::read_dir(path)
                    .unwrap()
                    .map(|entry| entry.unwrap().path())
                    .collect::<Vec<_>>();
                children.sort();
                for child in children {
                    visit(base, &child, entries);
                }
            } else {
                entries.push(SnapshotEntry {
                    path: relative,
                    kind: "other",
                    data: Vec::new(),
                });
            }
        }

        let mut entries = Vec::new();
        visit(root, root, &mut entries);
        entries
    }

    fn seed_target(root: &Path) -> PathBuf {
        let target = root.join("type-exercise-starter/supplied-tests/src");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("learner-marker.rs"), b"// unchanged\n").unwrap();
        target
    }

    #[cfg(unix)]
    fn replace_with_outside_symlink(root: &Path, relative: &str, outside_is_dir: bool) -> PathBuf {
        let victim = root.join(relative);
        let outside = root.join("outside-sentinel");
        if outside_is_dir {
            fs::create_dir(&outside).unwrap();
            fs::write(outside.join("sentinel"), b"outside\n").unwrap();
        } else {
            fs::write(&outside, b"outside\n").unwrap();
        }
        let metadata = fs::symlink_metadata(&victim).unwrap();
        if metadata.is_dir() {
            fs::remove_dir_all(&victim).unwrap();
        } else {
            fs::remove_file(&victim).unwrap();
        }
        symlink(&outside, &victim).unwrap();
        outside
    }

    #[cfg(unix)]
    fn move_directory_to_outside_symlink(root: &Path, relative: &str) -> PathBuf {
        let victim = root.join(relative);
        let outside = root.join("outside-ancestor-sentinel");
        fs::rename(&victim, &outside).unwrap();
        fs::write(outside.join("sentinel"), b"outside\n").unwrap();
        symlink(&outside, &victim).unwrap();
        outside
    }

    fn fixture() -> TempDir {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("type-exercise/supplied-tests/src");
        let chapter_1 = source.join("chapter_1");
        let checkpoints = root
            .path()
            .join("type-exercise/supplied-tests/checkpoints/chapter_1");
        let roots = root.path().join("type-exercise/supplied-tests/roots");
        fs::create_dir_all(&chapter_1).unwrap();
        fs::create_dir_all(&checkpoints).unwrap();
        fs::create_dir_all(&roots).unwrap();
        fs::create_dir_all(root.path().join("type-exercise-starter/supplied-tests")).unwrap();
        fs::create_dir_all(root.path().join("type-exercise-starter/expr/src")).unwrap();

        fs::write(
            chapter_1.join("mod.rs"),
            b"mod checkpoint_1;\nmod checkpoint_2;\nmod checkpoint_3;\nmod checkpoint_4;\n",
        )
        .unwrap();
        for checkpoint in 1..=4 {
            fs::write(
                chapter_1.join(format!("checkpoint_{checkpoint}.rs")),
                format!("fn checkpoint_{checkpoint}() {{}}\n"),
            )
            .unwrap();
        }
        for checkpoint in 1..=3 {
            fs::write(
                checkpoints.join(format!("checkpoint_{checkpoint}.rs")),
                (1..=checkpoint)
                    .map(|number| format!("mod checkpoint_{number};\n"))
                    .collect::<String>(),
            )
            .unwrap();
        }
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
            let error = copy_test(root.path(), chapter, None).unwrap_err();
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
        copy_test(root.path(), 3, None).unwrap();

        for chapter in 2..=3 {
            let source = root.path().join(format!(
                "type-exercise/supplied-tests/src/chapter_{chapter}.rs"
            ));
            let target = root.path().join(format!(
                "type-exercise-starter/supplied-tests/src/chapter_{chapter}.rs"
            ));
            assert_eq!(fs::read(target).unwrap(), fs::read(source).unwrap());
        }
        for name in [
            "mod.rs",
            "checkpoint_1.rs",
            "checkpoint_2.rs",
            "checkpoint_3.rs",
            "checkpoint_4.rs",
        ] {
            assert_eq!(
                fs::read(
                    root.path()
                        .join("type-exercise-starter/supplied-tests/src/chapter_1")
                        .join(name)
                )
                .unwrap(),
                fs::read(
                    root.path()
                        .join("type-exercise/supplied-tests/src/chapter_1")
                        .join(name)
                )
                .unwrap()
            );
        }

        copy_test(root.path(), 2, None).unwrap();
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
    fn copies_only_the_selected_checkpoint_root_and_incremental_modules() {
        let root = fixture();
        copy_test(root.path(), 1, Some(2)).unwrap();
        let target = root
            .path()
            .join("type-exercise-starter/supplied-tests/src/chapter_1");
        assert_eq!(
            fs::read(target.join("mod.rs")).unwrap(),
            fs::read(
                root.path()
                    .join("type-exercise/supplied-tests/checkpoints/chapter_1/checkpoint_2.rs")
            )
            .unwrap()
        );
        for checkpoint in 1..=2 {
            assert_eq!(
                fs::read(target.join(format!("checkpoint_{checkpoint}.rs"))).unwrap(),
                fs::read(root.path().join(format!(
                    "type-exercise/supplied-tests/src/chapter_1/checkpoint_{checkpoint}.rs"
                )))
                .unwrap()
            );
        }
        assert!(!target.join("checkpoint_3.rs").exists());
        assert!(!target.join("checkpoint_4.rs").exists());
    }

    #[test]
    fn repeated_copy_is_byte_identical_and_does_not_rewrite_files() {
        let root = fixture();
        assert_eq!(copy_test(root.path(), 3, None).unwrap().changed_files, 8);
        assert_eq!(copy_test(root.path(), 3, None).unwrap().changed_files, 0);
        assert_eq!(
            fs::read(
                root.path()
                    .join("type-exercise-starter/supplied-tests/src/chapter_1/mod.rs")
            )
            .unwrap(),
            fs::read(
                root.path()
                    .join("type-exercise/supplied-tests/src/chapter_1/mod.rs")
            )
            .unwrap()
        );
    }

    #[test]
    fn repairs_only_supplied_test_drift() {
        let root = fixture();
        let target = root.path().join("type-exercise-starter/supplied-tests/src");
        fs::create_dir_all(target.join("chapter_1")).unwrap();
        fs::write(target.join("chapter_1/mod.rs"), b"// drifted\n").unwrap();
        fs::write(target.join("chapter_1/checkpoint_5.rs"), b"// stale\n").unwrap();
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

        let report = copy_test(root.path(), 2, None).unwrap();
        assert_eq!(report.changed_files, 8);
        assert_eq!(
            fs::read(target.join("chapter_1/mod.rs")).unwrap(),
            fs::read(
                root.path()
                    .join("type-exercise/supplied-tests/src/chapter_1/mod.rs")
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
            root.join("type-exercise/supplied-tests/src/chapter_1/mod.rs")
                .is_file()
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_every_repository_relative_ancestor_symlink_before_target_mutation() {
        let cases = [
            ("type-exercise", 1, Some(1)),
            ("type-exercise/supplied-tests", 1, Some(1)),
            ("type-exercise-starter", 1, Some(1)),
        ];

        for (relative, chapter, checkpoint) in cases {
            let root = fixture();
            let target = seed_target(root.path());
            let outside = move_directory_to_outside_symlink(root.path(), relative);
            let target_before = snapshot(&target);
            let outside_before = snapshot(&outside);

            assert!(copy_test(root.path(), chapter, checkpoint).is_err());
            assert_eq!(
                snapshot(&target),
                target_before,
                "target changed for {relative}"
            );
            assert_eq!(
                snapshot(&outside),
                outside_before,
                "outside sentinel changed for {relative}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn rejects_every_selected_source_symlink_before_target_mutation() {
        let cases = [
            (
                "type-exercise/supplied-tests/src/chapter_2.rs",
                false,
                2,
                None,
            ),
            (
                "type-exercise/supplied-tests/src/chapter_1",
                true,
                1,
                Some(1),
            ),
            (
                "type-exercise/supplied-tests/src/chapter_1/checkpoint_1.rs",
                false,
                1,
                Some(1),
            ),
            (
                "type-exercise/supplied-tests/checkpoints/chapter_1/checkpoint_1.rs",
                false,
                1,
                Some(1),
            ),
            (
                "type-exercise/supplied-tests/roots/chapter_1.rs",
                false,
                1,
                Some(1),
            ),
        ];

        for (relative, outside_is_dir, chapter, checkpoint) in cases {
            let root = fixture();
            let target = seed_target(root.path());
            let outside = replace_with_outside_symlink(root.path(), relative, outside_is_dir);
            let target_before = snapshot(&target);
            let outside_before = snapshot(&outside);

            assert!(copy_test(root.path(), chapter, checkpoint).is_err());
            assert_eq!(
                snapshot(&target),
                target_before,
                "target changed for {relative}"
            );
            assert_eq!(
                snapshot(&outside),
                outside_before,
                "outside sentinel changed for {relative}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn rejects_every_managed_destination_symlink_before_target_mutation() {
        let cases = [
            ("type-exercise-starter/supplied-tests/src", true, false),
            (
                "type-exercise-starter/supplied-tests/src/chapter_1",
                true,
                true,
            ),
            (
                "type-exercise-starter/supplied-tests/src/chapter_1/mod.rs",
                false,
                true,
            ),
        ];

        for (relative, outside_is_dir, seed_chapter) in cases {
            let root = fixture();
            let target = seed_target(root.path());
            if seed_chapter {
                fs::create_dir_all(target.join("chapter_1")).unwrap();
                fs::write(target.join("chapter_1/mod.rs"), b"// old root\n").unwrap();
            }
            let outside = replace_with_outside_symlink(root.path(), relative, outside_is_dir);
            let target_before = snapshot(&target);
            let outside_before = snapshot(&outside);

            assert!(copy_test(root.path(), 1, Some(1)).is_err());
            assert_eq!(
                snapshot(&target),
                target_before,
                "target changed for {relative}"
            );
            assert_eq!(
                snapshot(&outside),
                outside_before,
                "outside sentinel changed for {relative}"
            );
        }
    }

    #[test]
    fn rejects_unexpected_source_and_destination_types_before_mutation() {
        let root = fixture();
        let target = seed_target(root.path());
        let module = root
            .path()
            .join("type-exercise/supplied-tests/src/chapter_1/checkpoint_1.rs");
        fs::remove_file(&module).unwrap();
        fs::create_dir(&module).unwrap();
        let target_before = snapshot(&target);
        assert!(copy_test(root.path(), 1, Some(1)).is_err());
        assert_eq!(snapshot(&target), target_before);

        let root = fixture();
        let target = seed_target(root.path());
        fs::write(target.join("chapter_1"), b"not a directory\n").unwrap();
        let target_before = snapshot(&target);
        assert!(copy_test(root.path(), 1, Some(1)).is_err());
        assert_eq!(snapshot(&target), target_before);

        let root = fixture();
        let supplied_tests = root.path().join("type-exercise-starter/supplied-tests");
        let target = supplied_tests.join("src");
        fs::write(&target, b"not a directory\n").unwrap();
        let target_before = snapshot(&target);
        assert!(copy_test(root.path(), 1, Some(1)).is_err());
        assert_eq!(snapshot(&target), target_before);
    }

    #[test]
    fn checkpoint_progress_is_visible_to_cargo_without_cleaning() {
        let root = fixture();
        let chapter = root
            .path()
            .join("type-exercise/supplied-tests/src/chapter_1");
        for checkpoint in 1..=2 {
            fs::write(
                chapter.join(format!("checkpoint_{checkpoint}.rs")),
                format!("#[test]\nfn checkpoint_{checkpoint}() {{}}\n"),
            )
            .unwrap();
        }
        fs::write(
            root.path()
                .join("type-exercise/supplied-tests/roots/chapter_1.rs"),
            b"mod chapter_1;\n",
        )
        .unwrap();
        fs::write(
            root.path()
                .join("type-exercise-starter/supplied-tests/Cargo.toml"),
            b"[package]\nname = \"copy-test-cargo-probe\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[workspace]\n",
        )
        .unwrap();

        let old_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_600_000_000);
        for checkpoint in 1..=2 {
            let path = root.path().join(format!(
                "type-exercise/supplied-tests/checkpoints/chapter_1/checkpoint_{checkpoint}.rs"
            ));
            fs::File::options()
                .write(true)
                .open(path)
                .unwrap()
                .set_times(fs::FileTimes::new().set_modified(old_time))
                .unwrap();
        }

        let manifest = root
            .path()
            .join("type-exercise-starter/supplied-tests/Cargo.toml");
        let cargo_target = root.path().join("cargo-target");
        let run = |checkpoint: usize| {
            copy_test(root.path(), 1, Some(checkpoint)).unwrap();
            let output = Command::new("cargo")
                .arg("test")
                .arg("--manifest-path")
                .arg(&manifest)
                .arg("--")
                .arg("--list")
                .env("CARGO_TARGET_DIR", &cargo_target)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "cargo test failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8(output.stdout).unwrap()
        };

        let first = run(1);
        assert!(first.contains("chapter_1::checkpoint_1::checkpoint_1: test"));
        assert!(!first.contains("chapter_1::checkpoint_2::checkpoint_2: test"));

        let second = run(2);
        assert!(second.contains("chapter_1::checkpoint_1::checkpoint_1: test"));
        assert!(second.contains("chapter_1::checkpoint_2::checkpoint_2: test"));
        assert_eq!(
            fs::read(
                root.path()
                    .join("type-exercise-starter/supplied-tests/src/chapter_1/mod.rs")
            )
            .unwrap(),
            fs::read(
                root.path()
                    .join("type-exercise/supplied-tests/checkpoints/chapter_1/checkpoint_2.rs")
            )
            .unwrap()
        );
    }
}
