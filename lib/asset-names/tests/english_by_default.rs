//! Guard for the "English by default" rule: file names are ASCII, and Rust code outside tests
//! holds no Hangul. See `docs/internal/asset-naming.md`.

use std::fs;
use std::path::{Path, PathBuf};

const SCANNED_DIRS: [&str; 3] = ["lib", "client", "tools"];

/// Files that may hold Hangul outside tests, each with the reason.
const ALLOWLIST: [(&str, &str); 1] = [(
    "tools/src/grf_merge/tables.rs",
    "doc comment about the CP949 bytes of one syllable",
)];

const HINT: &str = "use the English name from ragnarok_asset_names::rename(), \
                    see docs/internal/asset-naming.md";

/// Paths are relative to the scanned root, with `/` as the separator.
#[derive(Debug, PartialEq, Eq)]
enum Violation {
    NonAsciiPath(String),
    Hangul {
        file: String,
        line: usize,
        text: String,
    },
}

/// Every violation under `lib/`, `client/` and `tools/` of `root`, in that order and sorted by
/// name inside each. `target/` directories are skipped.
fn scan(root: &Path) -> Vec<Violation> {
    let mut violations = Vec::new();
    for dir in SCANNED_DIRS {
        let path = root.join(dir);
        if path.is_dir() {
            scan_dir(&path, dir, &mut violations);
        }
    }
    violations
}

fn scan_dir(path: &Path, relative: &str, violations: &mut Vec<Violation>) {
    let mut entries: Vec<fs::DirEntry> = fs::read_dir(path)
        .and_then(Iterator::collect)
        .unwrap_or_else(|error| panic!("cannot list {path:?}: {error}"));
    entries.sort_by_key(fs::DirEntry::file_name);

    for entry in entries {
        let name = entry.file_name();
        // Does not follow symlinks, so a link cannot lead the walk out of the workspace.
        let kind = entry
            .file_type()
            .unwrap_or_else(|error| panic!("cannot stat {:?}: {error}", entry.path()));
        if kind.is_dir() && name == "target" {
            continue;
        }
        // A name that is not UTF-8 gets U+FFFD, so it is reported too.
        let child = format!("{relative}/{}", name.to_string_lossy());
        if !child.is_ascii() {
            violations.push(Violation::NonAsciiPath(child.clone()));
        }
        if kind.is_dir() {
            scan_dir(&entry.path(), &child, violations);
        } else if kind.is_file() && child.ends_with(".rs") && !may_hold_hangul(&child) {
            scan_source(&entry.path(), &child, violations);
        }
    }
}

fn may_hold_hangul(relative: &str) -> bool {
    let in_tests_dir = relative
        .rsplit_once('/')
        .is_some_and(|(dirs, _)| dirs.split('/').any(|dir| dir == "tests"));
    in_tests_dir
        || relative.starts_with("lib/asset-names/")
        || ALLOWLIST.iter().any(|(file, _reason)| *file == relative)
}

/// Reports Hangul lines up to the first `#[cfg(test)]`; the repo keeps test modules at the end of
/// a file.
fn scan_source(path: &Path, relative: &str, violations: &mut Vec<Violation>) {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("cannot read {path:?}: {error}"));
    for (index, line) in String::from_utf8_lossy(&bytes).lines().enumerate() {
        if line.chars().any(is_hangul_syllable) {
            violations.push(Violation::Hangul {
                file: relative.to_string(),
                line: index + 1,
                text: line.to_string(),
            });
        }
        if line.trim_start().starts_with("#[cfg(test)]") {
            break;
        }
    }
}

fn is_hangul_syllable(c: char) -> bool {
    ('\u{AC00}'..='\u{D7A3}').contains(&c)
}

fn workspace_root() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // A wrong root would scan nothing and pass.
    for dir in SCANNED_DIRS {
        assert!(root.join(dir).is_dir(), "{dir}/ not found under {root:?}");
    }
    root
}

#[test]
fn file_names_are_ascii() {
    let paths: Vec<String> = scan(&workspace_root())
        .into_iter()
        .filter_map(|violation| match violation {
            Violation::NonAsciiPath(path) => Some(path),
            Violation::Hangul { .. } => None,
        })
        .collect();
    assert!(
        paths.is_empty(),
        "non-ASCII file or directory names:\n{}\n{HINT}",
        paths.join("\n")
    );
}

#[test]
fn code_has_no_hangul_outside_tests() {
    let lines: Vec<String> = scan(&workspace_root())
        .into_iter()
        .filter_map(|violation| match violation {
            Violation::Hangul { file, line, text } => Some(format!("{file}:{line}: {text}")),
            Violation::NonAsciiPath(_) => None,
        })
        .collect();
    assert!(
        lines.is_empty(),
        "Hangul outside tests:\n{}\n{HINT}",
        lines.join("\n")
    );
}

/// A throwaway source tree, removed on drop. Each test passes its own `name`.
struct Tree(PathBuf);

impl Tree {
    fn new(name: &str, files: &[(&str, &str)]) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ragnarok-english-by-default-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        for (path, content) in files {
            let path = root.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }
        Tree(root)
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn hangul(file: &str, line: usize, text: &str) -> Violation {
    Violation::Hangul {
        file: file.to_string(),
        line,
        text: text.to_string(),
    }
}

const KOREAN_LINE: &str = "const X: &str = \"몬스터\";";

#[test]
fn scan_reports_a_hangul_line_in_every_scanned_directory() {
    let source = format!("// header\n{KOREAN_LINE}\n");
    let tree = Tree::new(
        "scanned_dirs",
        &[
            ("lib/game/src/a.rs", &source),
            ("client/src/b.rs", &source),
            ("tools/src/c.rs", &source),
            ("docs/d.rs", &source),
        ],
    );
    assert_eq!(
        scan(&tree.0),
        vec![
            hangul("lib/game/src/a.rs", 2, KOREAN_LINE),
            hangul("client/src/b.rs", 2, KOREAN_LINE),
            hangul("tools/src/c.rs", 2, KOREAN_LINE),
        ]
    );
}

#[test]
fn scan_exempts_tests_directories_asset_names_and_the_allowlist() {
    let tree = Tree::new(
        "exempt_paths",
        &[
            ("lib/game/tests/a.rs", KOREAN_LINE),
            ("tools/tests/nested/b.rs", KOREAN_LINE),
            ("lib/asset-names/src/lib.rs", KOREAN_LINE),
            ("tools/src/grf_merge/tables.rs", KOREAN_LINE),
            // Only a directory named exactly `tests` is exempt.
            ("lib/game/src/tests.rs", KOREAN_LINE),
            ("lib/game/src/mytests/c.rs", KOREAN_LINE),
            ("lib/asset-names-extra/d.rs", KOREAN_LINE),
        ],
    );
    assert_eq!(
        scan(&tree.0),
        vec![
            hangul("lib/asset-names-extra/d.rs", 1, KOREAN_LINE),
            hangul("lib/game/src/mytests/c.rs", 1, KOREAN_LINE),
            hangul("lib/game/src/tests.rs", 1, KOREAN_LINE),
        ]
    );
}

#[test]
fn scan_exempts_only_the_lines_after_the_first_cfg_test() {
    let source = format!(
        "{KOREAN_LINE}\n// #[cfg(test)] in a comment\n{KOREAN_LINE}\n    #[cfg(test)]\n\
         mod tests {{\n    {KOREAN_LINE}\n}}\n"
    );
    let tree = Tree::new("cfg_test", &[("lib/a.rs", &source)]);
    assert_eq!(
        scan(&tree.0),
        vec![
            hangul("lib/a.rs", 1, KOREAN_LINE),
            hangul("lib/a.rs", 3, KOREAN_LINE),
        ]
    );
}

#[test]
fn scan_matches_hangul_syllables_only_and_only_in_rust_files() {
    let tree = Tree::new(
        "syllable_range",
        &[
            ("lib/first.rs", "// \u{AC00}\r\n"),
            ("lib/last.rs", "// \u{D7A3}"),
            // Just outside the block, a compatibility jamo, and other non-ASCII text.
            ("lib/other.rs", "// \u{ABFF} \u{D7A4} \u{3131} é →\n"),
            ("lib/empty.rs", ""),
            ("lib/notes.txt", KOREAN_LINE),
        ],
    );
    assert_eq!(
        scan(&tree.0),
        vec![
            hangul("lib/first.rs", 1, "// \u{AC00}"),
            hangul("lib/last.rs", 1, "// \u{D7A3}"),
        ]
    );
}

#[test]
fn scan_reports_non_ascii_names_even_where_hangul_text_is_allowed() {
    let tree = Tree::new(
        "non_ascii_names",
        &[
            ("lib/game/tests/몬스터.txt", ""),
            ("client/données/a.txt", ""),
        ],
    );
    assert_eq!(
        scan(&tree.0),
        vec![
            Violation::NonAsciiPath("lib/game/tests/몬스터.txt".to_string()),
            Violation::NonAsciiPath("client/données".to_string()),
            Violation::NonAsciiPath("client/données/a.txt".to_string()),
        ]
    );
}

#[test]
fn scan_skips_target_directories() {
    let tree = Tree::new(
        "target_dirs",
        &[
            ("lib/game/target/debug/a.rs", KOREAN_LINE),
            ("tools/target/몬스터.txt", ""),
        ],
    );
    assert_eq!(scan(&tree.0), vec![]);
}
