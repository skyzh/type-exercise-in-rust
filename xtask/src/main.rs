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
    /// Copy one chapter's tests from the reference solution into the starter.
    CopyTest {
        #[arg(long)]
        chapter: usize,
    },
}

fn workspace_root() -> Result<PathBuf> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .context("failed to find the workspace root")
}

fn copy_test(root: &Path, chapter: usize) -> Result<()> {
    let test_name = format!("chapter_{chapter}.rs");
    let source = root.join("type-exercise/src/tests").join(&test_name);
    if !source.is_file() {
        bail!("no tests are available for chapter {chapter}");
    }

    let target_dir = root.join("type-exercise-starter/src/tests");
    fs::create_dir_all(&target_dir).context("failed to create the starter test directory")?;
    fs::copy(&source, target_dir.join(&test_name))
        .with_context(|| format!("failed to copy {}", source.display()))?;

    let mut modules = fs::read_dir(&target_dir)
        .context("failed to list copied starter tests")?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    modules.sort();

    let mut test_module = String::new();
    writeln!(
        test_module,
        "//! DO NOT MODIFY -- copied course test modules"
    )?;
    writeln!(
        test_module,
        "//! This file is rewritten by `cargo x copy-test`."
    )?;
    for module in modules {
        if module.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        let module = module
            .file_stem()
            .and_then(|name| name.to_str())
            .context("invalid test module filename")?;
        writeln!(test_module, "mod {module};")?;
    }
    fs::write(root.join("type-exercise-starter/src/tests.rs"), test_module)
        .context("failed to update the starter test module list")?;

    println!("copied Chapter {chapter} tests into type-exercise-starter");
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let root = workspace_root()?;
    match args.action {
        Action::CopyTest { chapter } => copy_test(&root, chapter),
    }
}
