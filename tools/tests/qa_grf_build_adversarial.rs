//! RO QA adversarial tests for PL-22 grf-build (post-merge review of PL-19).

use std::io::Write;
use std::path::{Path, PathBuf};

use ragnarok_asset_names::normalize;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_tools::grf_build::{self, BuildError, Options};

/// Writes a GRF 0x200 the way the original tools do: names stored exactly as
/// given (EUC-KR, any case, any separator), in the order given.
fn write_raw_grf_flags(path: &Path, files: &[(&str, &[u8], u8)]) {
    use flate2::Compression;
    use flate2::write::ZlibEncoder;

    const HEADER: usize = 46;
    let zlib = |data: &[u8]| {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    };

    let mut body = Vec::new();
    let mut table = Vec::new();
    for (name, data, flags) in files {
        let compressed = zlib(data);
        let (encoded, _, _) = encoding_rs::EUC_KR.encode(name);
        table.extend_from_slice(&encoded);
        table.push(0);
        table.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        table.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        table.extend_from_slice(&(data.len() as u32).to_le_bytes());
        table.push(*flags);
        table.extend_from_slice(&(body.len() as u32).to_le_bytes());
        body.extend_from_slice(&compressed);
    }
    let table_compressed = zlib(&table);

    let mut header = [0u8; HEADER];
    header[..16].copy_from_slice(b"Master of Magic\0");
    header[30..34].copy_from_slice(&(body.len() as u32).to_le_bytes());
    header[38..42].copy_from_slice(&((files.len() + 7) as u32).to_le_bytes());
    header[42..46].copy_from_slice(&0x200u32.to_le_bytes());

    let mut out = Vec::new();
    out.extend_from_slice(&header);
    out.extend_from_slice(&body);
    out.extend_from_slice(&(table_compressed.len() as u32).to_le_bytes());
    out.extend_from_slice(&(table.len() as u32).to_le_bytes());
    out.extend_from_slice(&table_compressed);
    std::fs::write(path, out).unwrap();
}

/// A scratch directory removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("grf_build_{}_{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    fn path(&self, file: &str) -> PathBuf {
        self.0.join(file)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn options(sources: &[&Path], out: &Path) -> Options {
    Options {
        sources: sources.iter().map(|p| p.to_path_buf()).collect(),
        out: out.to_path_buf(),
        all: true,
        server: None,
        keep: Vec::new(),
        manifest: None,
        allow_collisions: false,
    }
}

/// AC 11 says a collision is two *different* original names. The same name
/// stored twice (an appended patch) is not a rename collision and must not
/// make grf-build exit 2.
#[test]
fn qa_same_original_name_twice_is_not_a_collision() {
    let dir = Scratch::new("qa_dup_same");
    let source = dir.path("old.grf");
    let out = dir.path("data.grf");
    write_raw_grf_flags(
        &source,
        &[("data\\a.txt", b"old", 0x01), ("data\\a.txt", b"new", 0x01)],
    );

    let grf = GrfArchive::open(&source).unwrap();
    assert!(
        grf.name_collisions().is_empty(),
        "identical names reported as collision: {:?}",
        grf.name_collisions()
    );
    let result = grf_build::build(&options(&[&source], &out));
    assert!(result.is_ok(), "{result:?}");
}

/// Before PL-22 a duplicated name resolved to the *last* entry of the table
/// (HashMap::insert). PL-22 silently switched it to the first one.
#[test]
fn qa_same_original_name_twice_keeps_pre_pl22_winner() {
    let dir = Scratch::new("qa_dup_winner");
    let source = dir.path("old.grf");
    write_raw_grf_flags(
        &source,
        &[("data\\a.txt", b"old", 0x01), ("data\\a.txt", b"new", 0x01)],
    );
    let grf = GrfArchive::open(&source).unwrap();
    assert_eq!(grf.read_file("data/a.txt").unwrap(), b"new");
}

/// A GRF table can hold directory entries (no FILE flag 0x01, size 0).
/// grf-build must skip them instead of failing the whole build.
#[test]
fn qa_directory_entry_does_not_fail_the_build() {
    let dir = Scratch::new("qa_dir_entry");
    let source = dir.path("old.grf");
    let out = dir.path("data.grf");
    write_raw_grf_flags(
        &source,
        &[
            ("data\\sprite\\몬스터", b"", 0x00),
            ("data\\sprite\\몬스터\\poring.spr", b"spr", 0x01),
        ],
    );
    let result = grf_build::build(&options(&[&source], &out));
    assert!(result.is_ok(), "{result:?}");
    let built = GrfArchive::open(&out).unwrap();
    assert_eq!(
        built
            .read_file(&normalize("data/sprite/몬스터/poring.spr"))
            .unwrap(),
        b"spr"
    );
}

/// Romanization is lossy (닥 and 닭 both give `dak`): two real names that
/// romanize alike must be reported, never merged silently.
#[test]
fn qa_romanization_ambiguity_is_reported_as_collision() {
    assert_eq!(normalize("닥"), normalize("닭"), "precondition");
    let dir = Scratch::new("qa_roman");
    let source = dir.path("old.grf");
    let out = dir.path("data.grf");
    write_raw_grf_flags(
        &source,
        &[("data\\닥.bmp", b"1", 0x01), ("data\\닭.bmp", b"2", 0x01)],
    );
    let err = grf_build::build(&options(&[&source], &out)).unwrap_err();
    assert!(
        matches!(err, BuildError::Collisions(ref l) if l.len() == 1),
        "{err:?}"
    );
    assert!(!out.exists());
}

#[allow(dead_code)]
fn _unused(_: PathBuf) {}
