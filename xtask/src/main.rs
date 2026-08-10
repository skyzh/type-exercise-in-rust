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
    },
    /// Compile the supplied tests and the compat fixtures against the opaque
    /// error layout, proving copied learner tests never pin BindError or
    /// ExpressionError variants.
    CheckOpaqueCompat {},
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
    let last_chapter = available
        .last()
        .copied()
        .context("no chapter tests are available")?;
    if chapter == 0 || chapter > last_chapter {
        bail!("no tests are available for chapter {chapter}");
    }

    // Read the complete cumulative source set before writing any supplied tests.
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

    println!("copied cumulative Chapters 1-{chapter} tests into type-exercise-starter");
    Ok(CopyReport { changed_files })
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.action {
        Action::CopyTest { chapter } => {
            copy_test(&workspace_root()?, chapter)?;
        }
        Action::CheckOpaqueCompat {} => {
            check_opaque_compat(&workspace_root()?)?;
        }
    }
    Ok(())
}

/// Compile-based learner-compatibility check.
///
/// The supplied tests are compiled against an opaque, unpinnable
/// `BindError`/`ExpressionError` layout (feature `opaque-errors`). If any
/// copied test references a real variant, that build fails. The focused
/// fixture files then prove the mechanism both ways: the clean fixture must
/// compile against the opaque rlib, and the broken fixture must not.
fn check_opaque_compat(root: &Path) -> Result<()> {
    let status = std::process::Command::new("cargo")
        .args([
            "check",
            "-p",
            "type-exercise",
            "--features",
            "opaque-errors",
            "--tests",
            "--locked",
        ])
        .current_dir(root)
        .status()
        .context("failed to run cargo check for the opaque-error build")?;
    if !status.success() {
        bail!(
            "supplied tests do not compile against the opaque error layout; \
             a copied test must be pinning a BindError/ExpressionError variant"
        );
    }

    let build = std::process::Command::new("cargo")
        .args([
            "build",
            "-p",
            "type-exercise",
            "--features",
            "opaque-errors",
            "--locked",
        ])
        .current_dir(root)
        .status()
        .context("failed to build the opaque-error rlib")?;
    if !build.success() {
        bail!("failed to build the opaque-error rlib");
    }
    let rlib = root.join("target/debug/libtype_exercise.rlib");
    if !rlib.exists() {
        bail!("opaque-error rlib not found at {}", rlib.display());
    }

    let compile_fixture = |name: &str| -> Result<bool> {
        let file = root.join("compat-fixture").join(name);
        let out_dir = root.join("target/compat-fixture-out");
        fs::create_dir_all(&out_dir)?;
        let status = std::process::Command::new("rustc")
            .args(["--edition", "2024", "--crate-type", "lib", "--out-dir"])
            .arg(&out_dir)
            .args(["-L", "dependency=target/debug/deps", "--extern"])
            .arg(format!("type_exercise={}", rlib.display()))
            .arg(&file)
            .current_dir(root)
            .status()
            .with_context(|| format!("failed to run rustc on {name}"))?;
        Ok(status.success())
    };

    if !compile_fixture("clean.rs")? {
        bail!("the clean compat fixture must compile against the opaque layout");
    }
    if compile_fixture("broken.rs")? {
        bail!(
            "the broken compat fixture must NOT compile against the opaque layout; \
             the opaque layout still exposes a pinnable error variant"
        );
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
    fn copies_an_exact_cumulative_prefix_and_removes_later_supplied_tests() {
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
    fn repeated_copy_is_byte_identical_and_does_not_rewrite_files() {
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
