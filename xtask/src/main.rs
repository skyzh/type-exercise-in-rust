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

fn unique_struct<'a>(syntax: &'a syn::File, name: &str) -> Result<&'a syn::ItemStruct> {
    let mut matches = syntax.items.iter().filter_map(|item| match item {
        syn::Item::Struct(item) if item.ident == name => Some(item),
        _ => None,
    });
    let item = matches
        .next()
        .with_context(|| format!("source is missing the real `{name}` struct item"))?;
    if matches.next().is_some() {
        bail!("source contains more than one real `{name}` struct item");
    }
    Ok(item)
}

fn is_simple_type(ty: &syn::Type, name: &str) -> bool {
    matches!(
        ty,
        syn::Type::Path(path)
            if path.qself.is_none()
                && path.path.segments.len() == 1
                && path.path.is_ident(name)
    )
}

fn is_type_named(ty: &syn::Type, name: &str) -> bool {
    matches!(
        ty,
        syn::Type::Path(path)
            if path.qself.is_none()
                && path.path.segments.last().is_some_and(|segment| segment.ident == name)
    )
}

fn is_vec_of(ty: &syn::Type, element: &str) -> bool {
    let syn::Type::Path(path) = ty else {
        return false;
    };
    let Some(segment) = path.path.segments.first() else {
        return false;
    };
    if path.qself.is_some() || path.path.segments.len() != 1 || segment.ident != "Vec" {
        return false;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return false;
    };
    let mut arguments = arguments.args.iter();
    let Some(syn::GenericArgument::Type(ty)) = arguments.next() else {
        return false;
    };
    arguments.next().is_none() && is_simple_type(ty, element)
}

fn validate_decimal_struct(item: &syn::ItemStruct, name: &str) -> Result<()> {
    if !matches!(item.vis, syn::Visibility::Public(_)) || !item.generics.params.is_empty() {
        bail!("{name} must remain one public, non-generic struct item");
    }
    let syn::Fields::Named(fields) = &item.fields else {
        bail!("{name} must use named fields");
    };
    let fields = fields.named.iter().collect::<Vec<_>>();
    let expected_names = ["values", "validity", "decimal_type", "null_count"];
    if fields.len() != expected_names.len() {
        bail!(
            "{name} must contain exactly one flat i128 buffer, packed BitVec validity, one shared DecimalType, and the derived null count"
        );
    }
    for (index, (field, expected_name)) in fields.iter().zip(expected_names).enumerate() {
        let name_matches = field
            .ident
            .as_ref()
            .is_some_and(|ident| ident == expected_name);
        let type_matches = match index {
            0 => is_vec_of(&field.ty, "i128"),
            1 => is_simple_type(&field.ty, "BitVec"),
            2 => is_simple_type(&field.ty, "DecimalType"),
            3 => is_simple_type(&field.ty, "usize"),
            _ => unreachable!(),
        };
        if !name_matches || !type_matches || !matches!(field.vis, syn::Visibility::Inherited) {
            bail!("{name} has a noncanonical field at position {}", index + 1);
        }
    }
    Ok(())
}

fn validate_decimal_storage_source(source: &str) -> Result<()> {
    let syntax = syn::parse_file(source).context("failed to parse Decimal storage as Rust")?;
    for name in ["DecimalArray", "DecimalArrayBuilder"] {
        validate_decimal_struct(unique_struct(&syntax, name)?, name)?;
    }
    Ok(())
}

fn is_pub_crate(visibility: &syn::Visibility) -> bool {
    matches!(
        visibility,
        syn::Visibility::Restricted(restricted)
            if restricted.in_token.is_none() && restricted.path.is_ident("crate")
    )
}

struct ReferenceDeclarationShapes {
    list_builder: &'static str,
    bound_evaluate_async: &'static str,
}

fn validate_reference_declaration_sources(
    list_source: &str,
    binder_source: &str,
) -> Result<ReferenceDeclarationShapes> {
    let list_syntax =
        syn::parse_file(list_source).context("failed to parse the reference List source")?;
    let builder = unique_struct(&list_syntax, "ListArrayBuilder")?;
    if !is_pub_crate(&builder.vis) {
        bail!("ListArrayBuilder must remain `pub(crate)` in the frozen reference source");
    }

    let binder_syntax =
        syn::parse_file(binder_source).context("failed to parse the reference binder source")?;
    let mut methods = binder_syntax.items.iter().filter_map(|item| {
        let syn::Item::Impl(item) = item else {
            return None;
        };
        let trait_matches = item
            .trait_
            .as_ref()
            .and_then(|(_, path, _)| path.segments.last())
            .is_some_and(|segment| segment.ident == "AsyncExpression");
        let self_matches = matches!(
            item.self_ty.as_ref(),
            syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident("BoundExpression")
        );
        (trait_matches && self_matches).then_some(item)
    });
    let implementation = methods
        .next()
        .context("missing `AsyncExpression for BoundExpression` implementation")?;
    if methods.next().is_some() {
        bail!("multiple `AsyncExpression for BoundExpression` implementations found");
    }
    let mut evaluate_async = implementation.items.iter().filter_map(|item| match item {
        syn::ImplItem::Fn(method) if method.sig.ident == "evaluate_async" => Some(method),
        _ => None,
    });
    let method = evaluate_async
        .next()
        .context("missing `BoundExpression::evaluate_async` trait method")?;
    if evaluate_async.next().is_some() || !matches!(method.vis, syn::Visibility::Inherited) {
        bail!("BoundExpression::evaluate_async must remain one non-public trait method");
    }
    Ok(ReferenceDeclarationShapes {
        list_builder: "pub(crate) struct ListArrayBuilder",
        bound_evaluate_async: "fn evaluate_async(",
    })
}

fn declaration_for_item<'a>(manifest: &'a ApiManifest, item: &str) -> Result<&'a str> {
    let mut matches = manifest.target.iter().flat_map(|target| {
        target
            .items
            .iter()
            .zip(&target.declarations)
            .filter_map(|(candidate, declaration)| (candidate == item).then_some(declaration))
    });
    let declaration = matches
        .next()
        .with_context(|| format!("starter API roadmap is missing `{item}`"))?;
    if matches.next().is_some() {
        bail!("starter API roadmap contains duplicate `{item}` declarations");
    }
    Ok(declaration)
}

fn validate_reference_declaration_parity(
    manifest: &ApiManifest,
    list_source: &str,
    binder_source: &str,
) -> Result<()> {
    let shapes = validate_reference_declaration_sources(list_source, binder_source)?;
    for (item, actual, expected) in [
        (
            "ListArrayBuilder",
            declaration_for_item(manifest, "ListArrayBuilder")?,
            shapes.list_builder,
        ),
        (
            "BoundExpression::evaluate_async",
            declaration_for_item(manifest, "BoundExpression::evaluate_async")?,
            shapes.bound_evaluate_async,
        ),
    ] {
        if actual != expected {
            bail!(
                "starter API declaration for `{item}` disagrees with the parsed reference source: expected `{expected}`, found `{actual}`"
            );
        }
    }
    Ok(())
}

fn validate_reference_declaration_shapes(root: &Path, manifest: &ApiManifest) -> Result<()> {
    let list_source = fs::read_to_string(root.join("type-exercise/src/array/list_array.rs"))?;
    let binder_source = fs::read_to_string(root.join("type-exercise/src/binder.rs"))?;
    validate_reference_declaration_parity(manifest, &list_source, &binder_source)
}

fn validate_invalid_index_struct(item: &syn::ItemStruct) -> Result<()> {
    if !matches!(item.vis, syn::Visibility::Public(_)) || !item.generics.params.is_empty() {
        bail!("InvalidIndex must remain one public, non-generic struct item");
    }
    let syn::Fields::Named(fields) = &item.fields else {
        bail!("InvalidIndex must use named fields");
    };
    let fields = fields.named.iter().collect::<Vec<_>>();
    let expected = ["row", "index", "values_len"];
    if fields.len() != expected.len() {
        bail!("InvalidIndex must contain exactly public row, index, and values_len fields");
    }
    for (position, (field, expected_name)) in fields.iter().zip(expected).enumerate() {
        let name_matches = field
            .ident
            .as_ref()
            .is_some_and(|ident| ident == expected_name);
        if !name_matches
            || !is_simple_type(&field.ty, "usize")
            || !matches!(field.vis, syn::Visibility::Public(_))
        {
            bail!(
                "InvalidIndex has a noncanonical public field at position {}",
                position + 1
            );
        }
    }
    Ok(())
}

fn is_result_self_invalid_index(output: &syn::ReturnType) -> bool {
    let syn::ReturnType::Type(_, ty) = output else {
        return false;
    };
    let syn::Type::Path(path) = ty.as_ref() else {
        return false;
    };
    let Some(result) = path.path.segments.last() else {
        return false;
    };
    if path.qself.is_some() || result.ident != "Result" {
        return false;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &result.arguments else {
        return false;
    };
    let mut arguments = arguments.args.iter();
    let Some(syn::GenericArgument::Type(success)) = arguments.next() else {
        return false;
    };
    let Some(syn::GenericArgument::Type(error)) = arguments.next() else {
        return false;
    };
    arguments.next().is_none()
        && is_simple_type(success, "Self")
        && is_simple_type(error, "InvalidIndex")
}

#[derive(Default)]
struct InvalidIndexReturnVisitor {
    count: usize,
}

impl<'ast> syn::visit::Visit<'ast> for InvalidIndexReturnVisitor {
    fn visit_expr_return(&mut self, expression: &'ast syn::ExprReturn) {
        let returns_invalid_index = expression.expr.as_deref().is_some_and(|returned| {
            let syn::Expr::Call(call) = returned else {
                return false;
            };
            let syn::Expr::Path(function) = call.func.as_ref() else {
                return false;
            };
            function.qself.is_none()
                && function.path.is_ident("Err")
                && call.args.len() == 1
                && matches!(
                    call.args.first(),
                    Some(syn::Expr::Struct(error))
                        if error.qself.is_none() && error.path.is_ident("InvalidIndex")
                )
        });
        if returns_invalid_index {
            self.count += 1;
        }
        syn::visit::visit_expr_return(self, expression);
    }
}

fn validate_invalid_index_relationships(syntax: &syn::File) -> Result<()> {
    for trait_name in ["Display", "Error"] {
        let mut implementations = syntax.items.iter().filter_map(|item| {
            let syn::Item::Impl(item) = item else {
                return None;
            };
            let trait_matches = item
                .trait_
                .as_ref()
                .and_then(|(_, path, _)| path.segments.last())
                .is_some_and(|segment| segment.ident == trait_name);
            let self_matches = is_simple_type(item.self_ty.as_ref(), "InvalidIndex");
            (trait_matches && self_matches).then_some(item)
        });
        implementations
            .next()
            .with_context(|| format!("InvalidIndex must implement {trait_name}"))?;
        if implementations.next().is_some() {
            bail!("InvalidIndex has multiple {trait_name} implementations");
        }
    }

    let mut methods = syntax
        .items
        .iter()
        .filter_map(|item| {
            let syn::Item::Impl(item) = item else {
                return None;
            };
            let self_matches = is_type_named(item.self_ty.as_ref(), "ColumnViewImpl");
            (item.trait_.is_none() && self_matches).then_some(item)
        })
        .flat_map(|implementation| implementation.items.iter())
        .filter_map(|item| match item {
            syn::ImplItem::Fn(method) if method.sig.ident == "indexed" => Some(method),
            _ => None,
        });
    let method = methods
        .next()
        .context("ColumnViewImpl must define one public indexed method")?;
    if methods.next().is_some()
        || !matches!(method.vis, syn::Visibility::Public(_))
        || !is_result_self_invalid_index(&method.sig.output)
    {
        bail!(
            "ColumnViewImpl::indexed must remain one public method returning Result<Self, InvalidIndex>"
        );
    }

    let mut returns = InvalidIndexReturnVisitor::default();
    syn::visit::Visit::visit_block(&mut returns, &method.block);
    if returns.count != 1 {
        bail!("ColumnViewImpl::indexed must return InvalidIndex on its live error path");
    }
    Ok(())
}

fn validate_indexed_view_source(source: &str) -> Result<()> {
    let syntax = syn::parse_file(source).context("failed to parse Indexed column-view source")?;
    validate_invalid_index_struct(unique_struct(&syntax, "InvalidIndex")?)?;
    validate_invalid_index_relationships(&syntax)?;
    for required in [
        "pub fn indexed(",
        "ColumnViewImplKind::Indexed",
        "ListColumnViewKind::Indexed",
        "ColumnViewKind::Indexed",
    ] {
        if !source.contains(required) {
            bail!("Indexed column-view source is missing `{required}`");
        }
    }
    for stale in [
        "InvalidDictionaryKey",
        "pub fn dictionary(",
        "ColumnViewImplKind::Dictionary",
        "ListColumnViewKind::Dictionary",
        "ColumnViewKind::Dictionary",
        "dictionary_len",
        "pub key: usize",
    ] {
        if source.contains(stale) {
            bail!("Indexed column-view source retains stale symbol `{stale}`");
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
    let column_source = fs::read_to_string(root.join("type-exercise/src/column.rs"))?;
    validate_indexed_view_source(&column_source)?;
    validate_reference_declaration_shapes(root, &manifest)?;

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
        ApiManifest, check_starter_api, copy_test, unique_struct, validate_decimal_storage_source,
        validate_indexed_view_source, validate_invalid_index_relationships,
        validate_invalid_index_struct, validate_manifest_against_approved,
        validate_reference_declaration_parity, validate_reference_declaration_sources,
        workspace_root,
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

        for (needle, replacement) in [
            (
                "    validity: BitVec,\n",
                "    validity: BitVec,\n    _row_types: Vec<DecimalType>,\n",
            ),
            (
                "    validity: BitVec,\n",
                "    validity: BitVec,\n    _row_values: Vec<Decimal>,\n",
            ),
            (
                "    values: Vec<i128>,\n",
                "    values: Vec<Option<Decimal>>,\n",
            ),
            ("    validity: BitVec,\n", "    validity: Vec<bool>,\n"),
            ("    validity: BitVec,\n", "    validity: Vec<u8>,\n"),
        ] {
            let mutated = source.replacen(needle, replacement, 1);
            assert!(validate_decimal_storage_source(&mutated).is_err());
        }
    }

    #[test]
    fn decimal_layout_ignores_commented_decoys_and_checks_the_real_structs() {
        let root = workspace_root().unwrap();
        let source =
            fs::read_to_string(root.join("type-exercise/src/array/decimal_array.rs")).unwrap();
        let decoys = r#"/*
pub struct DecimalArray {
    values: Vec<i128>,
    validity: BitVec,
    decimal_type: DecimalType,
    null_count: usize,
}
pub struct DecimalArrayBuilder {
    values: Vec<i128>,
    validity: BitVec,
    decimal_type: DecimalType,
    null_count: usize,
}
*/
"#;
        let mutated = source
            .replace(
                "    validity: BitVec,\n    decimal_type: DecimalType,\n",
                "    validity: BitVec,\n    _row_types: Vec<DecimalType>,\n    decimal_type: DecimalType,\n",
            )
            .replacen(
                "        Ok(Self {\n            values,\n",
                "        Ok(Self {\n            _row_types: vec![decimal_type; values.len()],\n            values,\n",
                1,
            )
            .replacen(
                "        Ok(Self {\n            values: Vec::with_capacity(capacity),\n",
                "        Ok(Self {\n            _row_types: Vec::with_capacity(capacity),\n            values: Vec::with_capacity(capacity),\n",
                1,
            )
            .replacen(
                "        DecimalArray {\n            values: self.values,\n",
                "        DecimalArray {\n            _row_types: self._row_types,\n            values: self.values,\n",
                1,
            );
        let mutated = format!("{decoys}{mutated}");
        assert_eq!(mutated.matches("_row_types").count(), 6);
        assert!(validate_decimal_storage_source(&mutated).is_err());
    }

    #[test]
    fn indexed_view_guard_rejects_stale_symbols_and_error_redirects() {
        let root = workspace_root().unwrap();
        let source = fs::read_to_string(root.join("type-exercise/src/column.rs")).unwrap();
        let tests = fs::read_to_string(root.join("type-exercise/src/tests/chapter_3.rs")).unwrap();
        validate_indexed_view_source(&source).unwrap();

        for (current, stale) in [
            ("pub fn indexed(", "pub fn dictionary("),
            (
                "ColumnViewImplKind::Indexed",
                "ColumnViewImplKind::Dictionary",
            ),
            (
                "ListColumnViewKind::Indexed",
                "ListColumnViewKind::Dictionary",
            ),
            ("ColumnViewKind::Indexed", "ColumnViewKind::Dictionary"),
            ("InvalidIndex", "InvalidDictionaryKey"),
            ("values_len", "dictionary_len"),
        ] {
            let mutated = source.replacen(current, stale, 1);
            assert!(validate_indexed_view_source(&mutated).is_err(), "{stale}");
        }

        let mutated_source = source
            .replacen("pub index: usize", "pub key: usize", 1)
            .replacen("self.index", "self.key", 1)
            .replacen(
                "                index,\n                values_len:",
                "                key: index,\n                values_len:",
                1,
            );
        let mutated_tests = tests.replace("            index:", "            key:");
        assert!(mutated_source.contains("pub key: usize"));
        assert_eq!(mutated_tests.matches("            key:").count(), 2);
        assert!(!mutated_tests.contains("            index:"));
        assert!(validate_indexed_view_source(&mutated_source).is_err());

        let redirected_source = source
            .replacen(
                "impl Display for InvalidIndex {",
                "#[derive(Clone, Copy, Debug, Eq, PartialEq)]\n\
                 pub struct IndexedViewError {\n\
                     pub row: usize,\n\
                     pub key: std::primitive::usize,\n\
                     pub values_len: usize,\n\
                 }\n\n\
                 impl Display for IndexedViewError {",
                1,
            )
            .replacen("self.index", "self.key", 1)
            .replacen(
                "impl Error for InvalidIndex {}",
                "impl Error for IndexedViewError {}",
                1,
            )
            .replacen(
                "Result<Self, InvalidIndex>",
                "Result<Self, IndexedViewError>",
                1,
            )
            .replacen(
                "return Err(InvalidIndex {",
                "return Err(IndexedViewError {",
                1,
            )
            .replacen("                index,", "                key: index,", 1);
        let redirected_tests = tests
            .replace("InvalidIndex", "IndexedViewError")
            .replace("            index:", "            key:");
        let redirected_syntax = syn::parse_file(&redirected_source).unwrap();
        syn::parse_file(&redirected_tests).unwrap();
        assert!(redirected_source.contains("pub struct InvalidIndex"));
        assert!(redirected_source.contains("pub struct IndexedViewError"));
        assert!(redirected_source.contains("Result<Self, IndexedViewError>"));
        assert_eq!(redirected_tests.matches("IndexedViewError {").count(), 2);
        validate_invalid_index_struct(unique_struct(&redirected_syntax, "InvalidIndex").unwrap())
            .unwrap();
        assert!(validate_invalid_index_relationships(&redirected_syntax).is_err());
        assert!(validate_indexed_view_source(&redirected_source).is_err());
    }

    #[test]
    fn reference_declaration_shapes_are_checked_against_real_syntax() {
        let root = workspace_root().unwrap();
        let list_source =
            fs::read_to_string(root.join("type-exercise/src/array/list_array.rs")).unwrap();
        let binder_source = fs::read_to_string(root.join("type-exercise/src/binder.rs")).unwrap();
        validate_reference_declaration_sources(&list_source, &binder_source).unwrap();
        let manifest: ApiManifest = toml::from_str(
            &fs::read_to_string(root.join("type-exercise-starter/api-roadmap.toml")).unwrap(),
        )
        .unwrap();
        validate_reference_declaration_parity(&manifest, &list_source, &binder_source).unwrap();

        let public_builder = list_source.replacen(
            "pub(crate) struct ListArrayBuilder",
            "pub struct ListArrayBuilder",
            1,
        );
        assert!(validate_reference_declaration_sources(&public_builder, &binder_source).is_err());

        let public_async = binder_source.replacen(
            "    fn evaluate_async<'a>",
            "    pub fn evaluate_async<'a>",
            1,
        );
        assert!(validate_reference_declaration_sources(&list_source, &public_async).is_err());

        let mut coordinated_roadmap_drift = manifest.clone();
        for target in &mut coordinated_roadmap_drift.target {
            for (item, declaration) in target.items.iter().zip(&mut target.declarations) {
                match item.as_str() {
                    "ListArrayBuilder" => {
                        *declaration = "pub struct ListArrayBuilder".to_owned();
                    }
                    "BoundExpression::evaluate_async" => {
                        *declaration = "pub fn evaluate_async(".to_owned();
                    }
                    _ => {}
                }
            }
        }
        assert!(
            validate_reference_declaration_parity(
                &coordinated_roadmap_drift,
                &list_source,
                &binder_source,
            )
            .is_err()
        );
    }
}
