//! Rebuilds a GRF with English resource names.
//!
//! The source archives are only read. Every entry is written under its
//! normalized (`ragnarok_asset_names`) name, and a manifest lists
//! `original<TAB>new` for each one.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use ragnarok_formats::grf::GrfArchive;

use crate::grf_audit::{self, Category};

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

/// The manifest path used when `Options::manifest` is `None`: the output's
/// file name with `.manifest.tsv` appended.
pub fn default_manifest_path(out: &Path) -> PathBuf {
    let mut name = out.file_name().unwrap_or_default().to_os_string();
    name.push(".manifest.tsv");
    out.with_file_name(name)
}

fn fail<T>(message: impl Into<String>) -> Result<T, BuildError> {
    Err(BuildError::Failed(message.into()))
}

/// A path that may not exist yet, made comparable: canonical when it exists,
/// otherwise its canonical parent plus the file name.
fn comparable(path: &Path) -> PathBuf {
    if let Ok(canonical) = path.canonicalize() {
        return canonical;
    }
    let parent = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    match (parent.canonicalize(), path.file_name()) {
        (Ok(parent), Some(name)) => parent.join(name),
        _ => path.to_path_buf(),
    }
}

/// Entries to leave out in the default mode: what `grf-prune` would drop with
/// its default categories (unreferenced textures and models), minus `--keep`.
fn dropped_by_audit(grf: &GrfArchive, opts: &Options) -> HashSet<String> {
    let report = grf_audit::run(
        grf,
        &grf_audit::Options {
            server_path: opts.server.clone(),
            ..grf_audit::Options::default()
        },
    );
    let categories: HashSet<Category> = [Category::Texture, Category::Model].into_iter().collect();
    report
        .prunable(&categories)
        .filter(|e| !opts.keep.iter().any(|p| e.name.starts_with(p.as_str())))
        .map(|e| e.name.clone())
        .collect()
}

pub fn build(opts: &Options) -> Result<Summary, BuildError> {
    if opts.sources.is_empty() {
        return fail("no source archive given");
    }
    let manifest_path = opts
        .manifest
        .clone()
        .unwrap_or_else(|| default_manifest_path(&opts.out));

    let sources: Vec<PathBuf> = opts.sources.iter().map(|p| comparable(p)).collect();
    for (label, target) in [("--out", &opts.out), ("--manifest", &manifest_path)] {
        if sources.contains(&comparable(target)) {
            return fail(format!(
                "{label} {} is also a source archive; refusing to overwrite it",
                target.display()
            ));
        }
    }
    if comparable(&opts.out) == comparable(&manifest_path) {
        return fail("--out and --manifest are the same file");
    }
    for target in [&opts.out, &manifest_path] {
        if target.exists() {
            return fail(format!(
                "{} already exists; refusing to overwrite it",
                target.display()
            ));
        }
    }
    for source in &opts.sources {
        if !source.is_file() {
            return fail(format!("source {} is not a file", source.display()));
        }
    }

    let source_paths: Vec<String> = opts
        .sources
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let grf = GrfArchive::open_layered(&source_paths, None)
        .map_err(|e| BuildError::Failed(format!("cannot open {}: {e}", source_paths[0])))?;
    if grf.layer_count() != opts.sources.len() {
        return fail("a source archive could not be opened as a GRF (see the log above)");
    }

    let collisions = grf.layered_name_collisions();
    if !collisions.is_empty() && !opts.allow_collisions {
        return Err(BuildError::Collisions(collisions));
    }

    let dropped_names = if opts.all {
        HashSet::new()
    } else {
        dropped_by_audit(&grf, opts)
    };

    let mut names: Vec<String> = grf
        .layered_file_list()
        .into_iter()
        .map(|f| f.name)
        .filter(|name| !dropped_names.contains(name))
        .collect();
    names.sort();

    let result = write_archive(&grf, &names, &opts.out)
        .and_then(|()| write_manifest(&grf, &names, &manifest_path));
    if let Err(e) = result {
        // `create` made the file, so it is ours to remove.
        let _ = std::fs::remove_file(&opts.out);
        return fail(e);
    }

    let size = |path: &Path| std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    Ok(Summary {
        files: names.len(),
        dropped: dropped_names.len(),
        source_bytes: opts.sources.iter().map(|p| size(p)).sum(),
        out_bytes: size(&opts.out),
        collisions: collisions.len(),
        manifest: manifest_path,
    })
}

fn write_archive(grf: &GrfArchive, names: &[String], out: &Path) -> Result<(), String> {
    let mut archive =
        GrfArchive::create(out).map_err(|e| format!("cannot create {}: {e}", out.display()))?;
    for name in names {
        let data = grf
            .read_file(name)
            .map_err(|e| format!("read {name}: {e}"))?;
        archive
            .add_file(name, &data)
            .map_err(|e| format!("write {name}: {e}"))?;
    }
    archive
        .save()
        .map_err(|e| format!("save {}: {e}", out.display()))
}

/// `original<TAB>new` per written entry, sorted by new name.
fn write_manifest(grf: &GrfArchive, names: &[String], path: &Path) -> Result<(), String> {
    let kept: HashSet<&str> = names.iter().map(String::as_str).collect();
    let mut text = String::new();
    for (new, original) in grf.layered_entries_with_original_names() {
        if kept.contains(new) {
            text.push_str(original);
            text.push('\t');
            text.push_str(new);
            text.push('\n');
        }
    }
    std::fs::write(path, text).map_err(|e| format!("write {}: {e}", path.display()))
}
