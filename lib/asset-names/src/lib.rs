//! Korean resource path -> English ASCII snake_case path.
//!
//! The original `data.grf` names its resources in Korean. The client reads
//! only the English names this crate produces, and `grf-build` rebuilds the
//! archive with them. The rules are in `docs/internal/asset-naming.md`; the
//! translations are in `dictionary.toml`.

pub mod romanize;

use std::collections::HashMap;
use std::sync::OnceLock;

use romanize::{is_hangul_syllable, romanize};

const BUILTIN_DICTIONARY: &str = include_str!("../dictionary.toml");

/// Hangul run -> English word, loaded from `dictionary.toml`.
pub struct Dictionary {
    words: HashMap<String, String>,
    /// Length in characters of the longest key, to bound the greedy split.
    longest_key: usize,
}

impl Dictionary {
    /// Parses the dictionary file format, validating every key and value.
    ///
    /// The format is flat: an optional `[words]` header, then
    /// `"<hangul run>" = "<snake_case value>"` lines. `#` starts a comment.
    /// Errors name the offending line, key or value.
    pub fn parse(toml: &str) -> Result<Dictionary, String> {
        let mut words: HashMap<String, String> = HashMap::new();
        let mut key_of_value: HashMap<String, String> = HashMap::new();
        let mut in_words = false;

        for (index, raw) in toml.lines().enumerate() {
            let number = index + 1;
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') {
                if line == "[words]" {
                    in_words = true;
                    continue;
                }
                return Err(format!(
                    "line {number}: unexpected table {line}, only [words] is allowed"
                ));
            }
            if !in_words {
                return Err(format!("line {number}: entry before the [words] table"));
            }

            let (key, value) = parse_entry(line).map_err(|e| format!("line {number}: {e}"))?;
            validate_key(&key).map_err(|e| format!("line {number}: {e}"))?;
            validate_value(&key, &value).map_err(|e| format!("line {number}: {e}"))?;

            if words.contains_key(&key) {
                return Err(format!("line {number}: duplicate key \"{key}\""));
            }
            if let Some(first) = key_of_value.get(&value) {
                return Err(format!(
                    "line {number}: duplicate value \"{value}\" for keys \"{first}\" and \"{key}\""
                ));
            }
            key_of_value.insert(value.clone(), key.clone());
            words.insert(key, value);
        }

        let longest_key = words.keys().map(|k| k.chars().count()).max().unwrap_or(0);
        Ok(Dictionary { words, longest_key })
    }

    /// The dictionary compiled into the crate, parsed once.
    ///
    /// Panics if `dictionary.toml` is invalid; a test guards the shipped file.
    pub fn builtin() -> &'static Dictionary {
        static BUILTIN: OnceLock<Dictionary> = OnceLock::new();
        BUILTIN.get_or_init(|| {
            Dictionary::parse(BUILTIN_DICTIONARY)
                .unwrap_or_else(|e| panic!("invalid built-in asset name dictionary: {e}"))
        })
    }

    /// Number of words.
    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    pub fn get(&self, run: &str) -> Option<&str> {
        self.words.get(run).map(String::as_str)
    }

    /// Translates one Hangul run: the whole run when it is a key, otherwise a
    /// greedy left-to-right split on the longest keys, romanizing what no key
    /// covers. The pieces are joined with `_`.
    fn translate_run(&self, run: &[char]) -> String {
        let whole: String = run.iter().collect();
        if let Some(value) = self.get(&whole) {
            return value.to_string();
        }

        let mut pieces: Vec<String> = Vec::new();
        let mut unmatched = String::new();
        let mut at = 0;
        while at < run.len() {
            let longest = self.longest_key.min(run.len() - at);
            let hit = (1..=longest).rev().find_map(|len| {
                let candidate: String = run[at..at + len].iter().collect();
                self.get(&candidate).map(|value| (len, value))
            });
            match hit {
                Some((len, value)) => {
                    if !unmatched.is_empty() {
                        pieces.push(romanize(&std::mem::take(&mut unmatched)));
                    }
                    pieces.push(value.to_string());
                    at += len;
                }
                None => {
                    unmatched.push(run[at]);
                    at += 1;
                }
            }
        }
        if !unmatched.is_empty() {
            pieces.push(romanize(&unmatched));
        }
        pieces.join("_")
    }
}

fn parse_entry(line: &str) -> Result<(String, String), String> {
    let rest = line
        .strip_prefix('"')
        .ok_or("expected a quoted key at the start of the line")?;
    let (key, rest) = rest.split_once('"').ok_or("unterminated key quote")?;
    let rest = rest
        .trim_start()
        .strip_prefix('=')
        .ok_or("expected `=` after the key")?;
    let rest = rest
        .trim_start()
        .strip_prefix('"')
        .ok_or("expected a quoted value after `=`")?;
    let (value, rest) = rest.split_once('"').ok_or("unterminated value quote")?;
    let rest = rest.trim();
    if !rest.is_empty() && !rest.starts_with('#') {
        return Err(format!("unexpected text after the value: {rest}"));
    }
    Ok((key.to_string(), value.to_string()))
}

fn validate_key(key: &str) -> Result<(), String> {
    if key.is_empty() {
        return Err("key is empty".to_string());
    }
    if let Some(bad) = key.chars().find(|&c| !is_hangul_syllable(c)) {
        return Err(format!(
            "key \"{key}\" has non-Hangul character {bad:?}; keys are Hangul syllables only"
        ));
    }
    Ok(())
}

fn validate_value(key: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("value for \"{key}\" is empty"));
    }
    let shape_ok = value
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !value.starts_with('_')
        && !value.ends_with('_')
        && !value.contains("__");
    if !shape_ok {
        return Err(format!(
            "value \"{value}\" for \"{key}\" must match ^[a-z0-9]+(_[a-z0-9]+)*$"
        ));
    }
    Ok(())
}

/// Renames `path` with `dict`.
///
/// 1. `\` becomes `/` and ASCII letters become lowercase.
/// 2. Each Hangul run is translated on its own (see [`Dictionary`]).
/// 3. A space becomes `_`; other ASCII is kept.
/// 4. Any other non-ASCII character becomes `u` and its lowercase hex code point.
///
/// No `_` is added around a run, so for a split that does not fall inside a
/// run `rename(a + b) == rename(a) + rename(b)`. The result is always ASCII and
/// `rename(rename(x)) == rename(x)`.
pub fn rename_with(dict: &Dictionary, path: &str) -> String {
    // Every lookup passes through here; most names are already ASCII.
    if path.is_ascii() {
        return path
            .chars()
            .map(|c| match c {
                '\\' => '/',
                ' ' => '_',
                c => c.to_ascii_lowercase(),
            })
            .collect();
    }
    let chars: Vec<char> = path.chars().collect();
    let mut out = String::with_capacity(path.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if is_hangul_syllable(c) {
            let end = chars[i..]
                .iter()
                .position(|&c| !is_hangul_syllable(c))
                .map_or(chars.len(), |n| i + n);
            out.push_str(&dict.translate_run(&chars[i..end]));
            i = end;
            continue;
        }
        match c {
            '\\' => out.push('/'),
            ' ' => out.push('_'),
            c if c.is_ascii() => out.push(c.to_ascii_lowercase()),
            c => out.push_str(&format!("u{:x}", c as u32)),
        }
        i += 1;
    }
    out
}

/// `rename_with` using the built-in dictionary.
pub fn rename(path: &str) -> String {
    rename_with(Dictionary::builtin(), path)
}

/// The lookup key used by every archive and data_dir index and every lookup.
///
/// The same as [`rename`]; a separate name so call sites say what they mean.
pub fn normalize(path: &str) -> String {
    rename(path)
}
