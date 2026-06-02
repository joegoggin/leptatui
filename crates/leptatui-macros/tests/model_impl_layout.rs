//! Structural checks for the `view!` macro model layout.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use syn::{File, Item, Type};

const MODELS: &[(&str, &str)] = &[
    ("Attr", "attr.rs"),
    ("Child", "child.rs"),
    ("Element", "element.rs"),
    ("TextContent", "text_content.rs"),
    ("ViewRoot", "view_root.rs"),
];

#[test]
fn view_models_keep_impls_with_their_type() {
    let model_dir = manifest_dir().join("src/view/model");
    let model_files = model_files(&model_dir);

    for (model_name, file_name) in MODELS {
        let path = model_dir.join(file_name);
        let file = parse_file(&path);

        assert_single_model_item(model_name, file_name, &file);
        assert_first_item_after_model_is_its_impl(model_name, file_name, &file);
    }

    for path in rust_files(&manifest_dir().join("src/view")) {
        let file = parse_file(&path);
        for item in file.items {
            let Item::Impl(item_impl) = item else {
                continue;
            };

            let Some(model_name) = impl_model_name(&item_impl.self_ty) else {
                continue;
            };
            let Some(expected_path) = model_files.get(model_name) else {
                continue;
            };

            assert_eq!(
                path,
                *expected_path,
                "impl for {model_name} must live in {}",
                expected_path.display(),
            );
        }
    }
}

fn assert_single_model_item(model_name: &str, file_name: &str, file: &File) {
    let model_items = file
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Struct(item_struct) => Some(item_struct.ident.to_string()),
            Item::Enum(item_enum) => Some(item_enum.ident.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        model_items,
        vec![model_name],
        "{file_name} must contain exactly one top-level model item named {model_name}",
    );
}

fn assert_first_item_after_model_is_its_impl(model_name: &str, file_name: &str, file: &File) {
    let model_position = file
        .items
        .iter()
        .position(|item| item_ident(item).is_some_and(|ident| ident == model_name))
        .unwrap_or_else(|| panic!("{file_name} does not contain model item {model_name}"));

    let next_item = file
        .items
        .get(model_position + 1)
        .unwrap_or_else(|| panic!("{file_name} must put an impl directly below {model_name}"));

    let Item::Impl(item_impl) = next_item else {
        panic!("{file_name} must put an impl directly below {model_name}");
    };

    assert_eq!(
        impl_model_name(&item_impl.self_ty),
        Some(model_name),
        "{file_name} must put an impl for {model_name} directly below the model",
    );
}

fn item_ident(item: &Item) -> Option<String> {
    match item {
        Item::Struct(item_struct) => Some(item_struct.ident.to_string()),
        Item::Enum(item_enum) => Some(item_enum.ident.to_string()),
        _ => None,
    }
}

fn impl_model_name(ty: &Type) -> Option<&'static str> {
    let Type::Path(type_path) = ty else {
        return None;
    };

    MODELS
        .iter()
        .map(|(model_name, _)| *model_name)
        .find(|model_name| type_path.path.is_ident(model_name))
}

fn model_files(model_dir: &Path) -> HashMap<&'static str, PathBuf> {
    MODELS
        .iter()
        .map(|(model_name, file_name)| (*model_name, model_dir.join(file_name)))
        .collect()
}

fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rust_files(root, &mut files);
    files
}

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

fn parse_file(path: &Path) -> File {
    let source = fs::read_to_string(path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", path.display());
    });

    syn::parse_file(&source).unwrap_or_else(|err| {
        panic!("failed to parse {}: {err}", path.display());
    })
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
