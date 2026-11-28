//! Characterization of the built-in identity tables: the lookup key of each
//! sprite name they carry must not change when Korean literals become English.
//!
//! The snapshot holds `normalize(name)` per row, which is what the GRF reader
//! looks up. To regenerate it after an intended change:
//! `BLESS_SNAPSHOT=1 cargo test -p ragnarok-formats --test normalized_tables`.

use ragnarok_asset_names::normalize;
use ragnarok_formats::builtin_accessory_table::BUILTIN_ACCESSORY_TABLE;
use ragnarok_formats::builtin_name_table::BUILTIN_NAME_TABLE;

const SNAPSHOT_FILE: &str = "tests/snapshots/normalized_tables.txt";
const SNAPSHOT: &str = include_str!("snapshots/normalized_tables.txt");

fn dump() -> Vec<String> {
    let mut out = Vec::new();
    for (id, name) in BUILTIN_NAME_TABLE {
        out.push(format!("name {id} {}", normalize(name)));
    }
    for (id, suffix) in BUILTIN_ACCESSORY_TABLE {
        out.push(format!("accessory {id} {}", normalize(suffix)));
    }
    out
}

#[test]
fn normalized_names_match_the_snapshot() {
    let actual = dump();
    if std::env::var_os("BLESS_SNAPSHOT").is_some() {
        let path = format!("{}/{SNAPSHOT_FILE}", env!("CARGO_MANIFEST_DIR"));
        std::fs::write(path, actual.join("\n") + "\n").expect("write snapshot");
        return;
    }
    let expected: Vec<&str> = SNAPSHOT.lines().collect();
    assert_eq!(actual.len(), expected.len(), "number of snapshot lines");
    for (a, e) in actual.iter().zip(&expected) {
        assert_eq!(a, e);
    }
}
