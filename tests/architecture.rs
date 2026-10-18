//! Architecture rules, checked on every `cargo test`.
//!
//! 1. Modules only depend "downwards": `calendar < identity < courses < capture < records`.
//! 2. `shared_lib` depends on no module.
//!
//! Rust's privacy already stops a module reaching into another module's internals; this test
//! protects the *direction* of dependencies, which the compiler cannot.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const MODULES: [&str; 5] = ["calendar", "identity", "courses", "capture", "records"];

/// The modules a module may depend on (everything below it).
fn allowed_dependencies(module: &str) -> &'static [&'static str] {
    match module {
        "calendar" => &[],
        "identity" => &["calendar"],
        "courses" => &["calendar", "identity"],
        "capture" => &["calendar", "identity", "courses"],
        "records" => &["calendar", "identity", "courses", "capture"],
        other => panic!("unknown module `{other}`"),
    }
}

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    files
}

/// Removes `//` comments (including doc comments) so prose that mentions a module is ignored.
fn strip_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| line.find("//").map_or(line, |at| &line[..at]))
        .collect::<Vec<_>>()
        .join("\n")
}

fn identifier_at_start(text: &str) -> &str {
    let end = text
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(text.len());
    &text[..end]
}

/// Names of the modules referenced through `modules::<name>` or `modules::{a, b::c}`.
fn referenced_modules(source: &str) -> BTreeSet<String> {
    let code = strip_comments(source);
    let mut found = BTreeSet::new();
    let mut rest = code.as_str();
    while let Some(at) = rest.find("modules::") {
        rest = &rest[at + "modules::".len()..];
        if let Some(group) = rest.strip_prefix('{') {
            let group = &group[..group.find('}').unwrap_or(group.len())];
            for item in group.split(',') {
                let name = identifier_at_start(item.trim());
                if !name.is_empty() {
                    found.insert(name.to_string());
                }
            }
        } else {
            let name = identifier_at_start(rest);
            if !name.is_empty() {
                found.insert(name.to_string());
            }
        }
    }
    found
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn modules_only_depend_on_modules_below_them() {
    let mut violations = Vec::new();
    for module in MODULES {
        let dir = crate_root().join("src/modules").join(module);
        for file in rust_files(&dir) {
            let source = fs::read_to_string(&file).unwrap();
            for referenced in referenced_modules(&source) {
                let is_other_module =
                    MODULES.contains(&referenced.as_str()) && referenced != module;
                if is_other_module && !allowed_dependencies(module).contains(&referenced.as_str()) {
                    violations.push(format!(
                        "{}: `{module}` must not depend on `{referenced}`",
                        file.strip_prefix(crate_root()).unwrap().display()
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "dependency direction broken:\n{}",
        violations.join("\n")
    );
}

#[test]
fn shared_lib_depends_on_no_module() {
    let mut violations = Vec::new();
    for file in rust_files(&crate_root().join("src/shared_lib")) {
        let source = fs::read_to_string(&file).unwrap();
        if !referenced_modules(&source).is_empty() {
            violations.push(
                file.strip_prefix(crate_root())
                    .unwrap()
                    .display()
                    .to_string(),
            );
        }
    }
    assert!(
        violations.is_empty(),
        "shared_lib must not use modules:\n{}",
        violations.join("\n")
    );
}

#[test]
fn every_module_directory_is_known_to_this_test() {
    let mut on_disk: Vec<String> = fs::read_dir(crate_root().join("src/modules"))
        .unwrap()
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    on_disk.sort();
    let mut known: Vec<String> = MODULES.iter().map(ToString::to_string).collect();
    known.sort();
    assert_eq!(
        on_disk, known,
        "add the new module to MODULES and allowed_dependencies"
    );
}

#[cfg(test)]
mod parser_tests {
    use super::*;

    #[test]
    fn finds_a_plain_path() {
        let found = referenced_modules("use crate::modules::capture::api::Session;");
        assert_eq!(found, BTreeSet::from(["capture".to_string()]));
    }

    #[test]
    fn finds_every_name_in_a_group() {
        let found = referenced_modules("use crate::modules::{courses::api, records::api::Status};");
        assert_eq!(
            found,
            BTreeSet::from(["courses".to_string(), "records".to_string()])
        );
    }

    #[test]
    fn ignores_comments() {
        let found = referenced_modules("// crate::modules::records is mentioned here\nfn x() {}");
        assert!(found.is_empty());
    }

    #[test]
    fn finds_nothing_when_there_are_no_module_paths() {
        assert!(referenced_modules("use std::fmt;").is_empty());
    }
}
