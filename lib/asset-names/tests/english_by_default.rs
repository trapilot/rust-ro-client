//! Guard for the "English by default" rule: file names are ASCII, and Rust code outside tests
//! holds no Hangul. See `docs/internal/asset-naming.md`.

use std::fs;
use std::path::{Path, PathBuf};

const SCANNED_DIRS: [&str; 3] = ["lib", "client", "tools"];

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
fn scan(_root: &Path) -> Vec<Violation> {
    todo!()
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
