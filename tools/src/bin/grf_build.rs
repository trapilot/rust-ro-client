//! Rebuilds `data.grf` with English resource names.
//!
//! Reads one or more source archives (Korean or English names), writes a new
//! archive whose entries are renamed with `ragnarok_asset_names::normalize`,
//! and a manifest of `original<TAB>new`. The sources are never modified.

use ragnarok_tools::grf_audit;
use ragnarok_tools::grf_build::{self, BuildError, Options};
use std::path::PathBuf;

const USAGE: &str = "\
usage: grf-build <source.grf> [more.grf ...] --out <data.grf> [options]

  --out <path>         archive to write (required; must not exist)
  --all                copy every entry (default: drop only what grf-prune
                       would drop with its default categories)
  --server <path>      same meaning as grf-prune
  --keep <prefix>      always keep entries under this normalized prefix
                       (repeatable)
  --manifest <path>    write TSV \"original<TAB>new\" sorted by new
                       (default: <out>.manifest.tsv)
  --allow-collisions   do not fail when two names become the same resource

Sources are listed highest priority first. Exit codes: 0 ok, 1 failure,
2 name collisions (nothing written), 64 usage error.
";

fn parse_args() -> Result<Options, String> {
    let mut sources = Vec::new();
    let mut out = None;
    let mut opts = Options {
        sources: Vec::new(),
        out: PathBuf::new(),
        all: false,
        server: None,
        keep: Vec::new(),
        manifest: None,
        allow_collisions: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut value = || {
            it.next()
                .ok_or_else(|| format!("{arg} needs a value\n\n{USAGE}"))
        };
        match arg.as_str() {
            "--out" => out = Some(PathBuf::from(value()?)),
            "--all" => opts.all = true,
            "--server" => opts.server = Some(PathBuf::from(value()?)),
            "--keep" => opts.keep.push(grf_audit::normalize(&value()?)),
            "--manifest" => opts.manifest = Some(PathBuf::from(value()?)),
            "--allow-collisions" => opts.allow_collisions = true,
            "-h" | "--help" => return Err(USAGE.to_string()),
            flag if flag.starts_with('-') => {
                return Err(format!("unknown flag `{flag}`\n\n{USAGE}"));
            }
            source => sources.push(PathBuf::from(source)),
        }
    }
    if sources.is_empty() {
        return Err(format!("no source archive given\n\n{USAGE}"));
    }
    opts.sources = sources;
    opts.out = out.ok_or_else(|| format!("--out is required\n\n{USAGE}"))?;
    Ok(opts)
}

fn mb(bytes: u64) -> String {
    format!("{:.1}MB", bytes as f64 / 1e6)
}

fn main() {
    let opts = match parse_args() {
        Ok(o) => o,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(64);
        }
    };

    match grf_build::build(&opts) {
        Ok(summary) => {
            println!(
                "wrote {}: {} files ({} dropped), {} -> {}, {} name collisions",
                opts.out.display(),
                summary.files,
                summary.dropped,
                mb(summary.source_bytes),
                mb(summary.out_bytes),
                summary.collisions
            );
            println!("manifest: {}", summary.manifest.display());
        }
        Err(BuildError::Collisions(list)) => {
            eprintln!(
                "{} name collision(s): different names became the same resource. \
                 Nothing was written. Pass --allow-collisions to keep the first of each.",
                list.len()
            );
            for (key, kept, dropped) in &list {
                eprintln!("  {key}\n    keep: {kept}\n    drop: {dropped}");
            }
            std::process::exit(2);
        }
        Err(BuildError::Failed(message)) => {
            eprintln!("grf-build failed: {message}");
            std::process::exit(1);
        }
    }
}
