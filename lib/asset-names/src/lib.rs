//! Korean resource path -> English ASCII snake_case path.
//!
//! See `docs/internal/asset-naming.md` for the rules.

pub mod romanize;

use std::collections::HashMap;

/// Hangul run -> English word, loaded from `dictionary.toml`.
pub struct Dictionary {
    #[allow(dead_code)]
    words: HashMap<String, String>,
}

impl Dictionary {
    /// Parses the dictionary file format, validating every key and value.
    pub fn parse(_toml: &str) -> Result<Dictionary, String> {
        todo!()
    }

    /// The dictionary compiled into the crate, parsed once.
    pub fn builtin() -> &'static Dictionary {
        todo!()
    }
}

/// Renames `path` with `dict`: separators and case first, then every Hangul run.
pub fn rename_with(_dict: &Dictionary, _path: &str) -> String {
    todo!()
}

/// `rename_with` using the built-in dictionary.
pub fn rename(path: &str) -> String {
    rename_with(Dictionary::builtin(), path)
}

/// The lookup key used by every archive and data_dir index and every lookup.
pub fn normalize(path: &str) -> String {
    rename(path)
}
