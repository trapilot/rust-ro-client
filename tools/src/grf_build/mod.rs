//! Rebuilds a GRF with English resource names.
//!
//! The source archives are only read. Every entry is written under its
//! normalized (`ragnarok_asset_names`) name, and a manifest lists
//! `original<TAB>new` for each one.

use std::path::PathBuf;

/// `(key, original kept, original dropped)`, as `GrfArchive::name_collisions`.
pub type NameCollision = (String, String, String);

pub struct Options {
    /// Highest priority first, like `GrfArchive::open_layered`.
    pub sources: Vec<PathBuf>,
    pub out: PathBuf,
    /// Copy every entry instead of dropping what `grf_audit` proves unused.
    pub all: bool,
    /// rathena checkout or map list, as `grf-prune --server`.
    pub server: Option<PathBuf>,
    /// Normalized prefixes that are never dropped.
    pub keep: Vec<String>,
    /// Defaults to `<out>.manifest.tsv`.
    pub manifest: Option<PathBuf>,
    pub allow_collisions: bool,
}

#[derive(Debug)]
pub enum BuildError {
    /// Different original names normalize to one key; nothing was written.
    Collisions(Vec<NameCollision>),
    /// Anything else: bad paths, unreadable archive, write failure.
    Failed(String),
}

#[derive(Debug)]
pub struct Summary {
    pub files: usize,
    pub dropped: usize,
    pub source_bytes: u64,
    pub out_bytes: u64,
    pub collisions: usize,
    pub manifest: PathBuf,
}

/// The manifest path used when `Options::manifest` is `None`.
pub fn default_manifest_path(_out: &std::path::Path) -> PathBuf {
    todo!()
}

pub fn build(_opts: &Options) -> Result<Summary, BuildError> {
    todo!()
}
