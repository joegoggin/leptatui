//! Structural checks for the `view!` macro syntax layout.
//!
//! These tests enforce the local convention that each syntax type keeps its
//! implementation in its declaration file or substantive owner directory.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use syn::{File, Item, Type};

/// View syntax type names and the source files that declare them.
const SYNTAX_TYPES: &[(&str, &str)] = &[
    ("Attr", "attr.rs"),
    ("Child", "child.rs"),
    ("Element", "element/mod.rs"),
    ("TextContent", "text_content.rs"),
    ("ViewRoot", "root.rs"),
];

/// Verifies view syntax implementations stay within their owning source area.
///
/// # Example Under Test
///
/// ```text
/// src/view/attr.rs
/// src/view/child.rs
/// src/view/element/{mod,attributes,builtins,children,custom_component}.rs
/// src/view/text_content.rs
/// src/view/root.rs
/// ```
///
/// # Assertions
///
/// - Each declaration file contains exactly one top-level syntax item with the
///   expected name.
/// - The first item after each syntax declaration is an impl for the same type.
/// - Each ordinary syntax type keeps its impls in its declaration file.
/// - `Element` impls stay inside the substantive `element` owner directory.
///
/// # Why
///
/// Keeping declarations and behavior within clear ownership boundaries makes
/// macro parsing and expansion logic easier to audit.
#[test]
fn view_syntax_types_keep_impls_with_their_owner() {
    let view_dir = manifest_dir().join("src/view");
    let declaration_files = declaration_files(&view_dir);

    for (type_name, file_name) in SYNTAX_TYPES {
        let path = view_dir.join(file_name);
        let file = parse_file(&path);

        assert_single_syntax_item(type_name, file_name, &file);
        assert_first_item_after_type_is_its_impl(type_name, file_name, &file);
    }

    for path in rust_files(&view_dir) {
        let file = parse_file(&path);
        for item in file.items {
            let Item::Impl(item_impl) = item else {
                continue;
            };

            let Some(type_name) = impl_syntax_type_name(&item_impl.self_ty) else {
                continue;
            };
            let Some(declaration_path) = declaration_files.get(type_name) else {
                continue;
            };

            assert!(
                impl_is_owned_by(type_name, &path, declaration_path),
                "impl for {type_name} must live with its owner at {}",
                declaration_path.display(),
            );
        }
    }
}

/// Asserts a declaration file contains exactly one top-level syntax item.
///
/// # Arguments
///
/// * `type_name` — Expected syntax type name.
/// * `file_name` — Declaration source file name used in diagnostics.
/// * `file` — Parsed Rust source file to inspect.
fn assert_single_syntax_item(type_name: &str, file_name: &str, file: &File) {
    let syntax_items = file
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Struct(item_struct) => Some(item_struct.ident.to_string()),
            Item::Enum(item_enum) => Some(item_enum.ident.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        syntax_items,
        vec![type_name],
        "{file_name} must contain exactly one top-level syntax item named {type_name}",
    );
}

/// Asserts the first item after a syntax declaration is its impl block.
///
/// # Arguments
///
/// * `type_name` — Expected syntax type name.
/// * `file_name` — Declaration source file name used in diagnostics.
/// * `file` — Parsed Rust source file to inspect.
fn assert_first_item_after_type_is_its_impl(type_name: &str, file_name: &str, file: &File) {
    let type_position = file
        .items
        .iter()
        .position(|item| item_ident(item).is_some_and(|ident| ident == type_name))
        .unwrap_or_else(|| panic!("{file_name} does not contain syntax item {type_name}"));

    let next_item = file
        .items
        .get(type_position + 1)
        .unwrap_or_else(|| panic!("{file_name} must put an impl directly below {type_name}"));

    let Item::Impl(item_impl) = next_item else {
        panic!("{file_name} must put an impl directly below {type_name}");
    };

    assert_eq!(
        impl_syntax_type_name(&item_impl.self_ty),
        Some(type_name),
        "{file_name} must put an impl for {type_name} directly below the declaration",
    );
}

/// Returns the identifier for a struct or enum item.
///
/// # Arguments
///
/// * `item` — Parsed Rust item to inspect.
///
/// # Returns
///
/// An [`Option<String>`] containing the struct or enum identifier.
fn item_ident(item: &Item) -> Option<String> {
    match item {
        Item::Struct(item_struct) => Some(item_struct.ident.to_string()),
        Item::Enum(item_enum) => Some(item_enum.ident.to_string()),
        _ => None,
    }
}

/// Returns the known view syntax type name implemented by a type.
///
/// # Arguments
///
/// * `ty` — Parsed impl target type.
///
/// # Returns
///
/// An [`Option<&'static str>`] containing the matched syntax type name.
fn impl_syntax_type_name(ty: &Type) -> Option<&'static str> {
    let Type::Path(type_path) = ty else {
        return None;
    };

    SYNTAX_TYPES
        .iter()
        .map(|(type_name, _)| *type_name)
        .find(|type_name| type_path.path.is_ident(type_name))
}

/// Returns the declaration path for every view syntax type.
///
/// # Arguments
///
/// * `view_dir` — Directory containing view syntax source files.
///
/// # Returns
///
/// A [`HashMap`] from syntax type names to declaration paths.
fn declaration_files(view_dir: &Path) -> HashMap<&'static str, PathBuf> {
    SYNTAX_TYPES
        .iter()
        .map(|(type_name, file_name)| (*type_name, view_dir.join(file_name)))
        .collect()
}

/// Returns whether an impl path belongs to a syntax type's source owner.
///
/// `Element` expansion is intentionally divided among direct files in the
/// `element` directory. Other syntax types keep every impl in their declaration
/// file.
///
/// # Arguments
///
/// * `type_name` — Syntax type implemented by the source file.
/// * `impl_path` — Source file containing the impl.
/// * `declaration_path` — Source file declaring the syntax type.
///
/// # Returns
///
/// `true` when the impl remains within the expected ownership boundary.
fn impl_is_owned_by(type_name: &str, impl_path: &Path, declaration_path: &Path) -> bool {
    if type_name == "Element" {
        return declaration_path
            .parent()
            .is_some_and(|element_dir| impl_path.starts_with(element_dir));
    }

    impl_path == declaration_path
}

/// Returns every Rust source file under a root directory.
///
/// # Arguments
///
/// * `root` — Directory to traverse recursively.
///
/// # Returns
///
/// A [`Vec<PathBuf>`] containing Rust source file paths.
fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rust_files(root, &mut files);
    files
}

/// Collects Rust source files recursively into an existing vector.
///
/// # Arguments
///
/// * `root` — Directory to traverse.
/// * `files` — Output collection receiving Rust source paths.
fn collect_rust_files(root: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", root.display());
    }) {
        let path = entry
            .unwrap_or_else(|err| panic!("failed to read entry in {}: {err}", root.display()))
            .path();

        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

/// Parses a Rust source file.
///
/// # Arguments
///
/// * `path` — Source file path to read and parse.
///
/// # Returns
///
/// A [`File`] syntax tree for the source file.
///
/// # Panics
///
/// Panics if the file cannot be read or parsed.
fn parse_file(path: &Path) -> File {
    let source = fs::read_to_string(path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", path.display());
    });

    syn::parse_file(&source).unwrap_or_else(|err| {
        panic!("failed to parse {}: {err}", path.display());
    })
}

/// Returns the macro crate manifest directory.
///
/// # Returns
///
/// A [`PathBuf`] containing `CARGO_MANIFEST_DIR`.
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
