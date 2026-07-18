//! Integration tests against the public API only (no internal modules).
//! Each test builds its own small, throwaway `.bin` file with
//! `write_database` so these tests never depend on the real GeoLite2 CSVs
//! being present.

use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use geolite_lookup::models::IpRange;
use geolite_lookup::{write_database, GeoDatabase, GeoError};

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A unique path under the OS temp dir so parallel test threads never
/// collide, plus a guard that deletes the file when dropped.
struct TempDbFile(PathBuf);

impl TempDbFile {
    fn new(label: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "geolite_lookup_test_{label}_{}_{n}.bin",
            std::process::id()
        ));
        TempDbFile(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDbFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn ip_u32(a: u8, b: u8, c: u8, d: u8) -> u32 {
    u32::from(Ipv4Addr::new(a, b, c, d))
}

fn sample_countries() -> Vec<String> {
    vec![
        "Unknown".to_string(),
        "Australia".to_string(),
        "United States".to_string(),
    ]
}

fn sample_ranges() -> Vec<IpRange> {
    vec![
        // 1.0.0.0/24 -> Australia
        IpRange {
            start: ip_u32(1, 0, 0, 0),
            end: ip_u32(1, 0, 0, 255),
            country_index: 1,
        },
        // 8.8.8.0/24 -> United States
        IpRange {
            start: ip_u32(8, 8, 8, 0),
            end: ip_u32(8, 8, 8, 255),
            country_index: 2,
        },
        // A single /32 host route, to exercise start == end.
        IpRange {
            start: ip_u32(9, 9, 9, 9),
            end: ip_u32(9, 9, 9, 9),
            country_index: 2,
        },
    ]
}

fn build_sample_db(path: &Path) {
    write_database(path, &sample_countries(), &sample_ranges())
        .expect("failed to write test database");
}

#[test]
fn looks_up_known_ip_addresses() {
    let db_file = TempDbFile::new("known_ip");
    build_sample_db(db_file.path());

    let db = GeoDatabase::open(db_file.path()).unwrap();
    assert_eq!(db.lookup("8.8.8.8").unwrap(), "United States");
    assert_eq!(db.lookup("1.0.0.128").unwrap(), "Australia");
}

#[test]
fn range_boundaries_are_inclusive() {
    let db_file = TempDbFile::new("boundaries");
    build_sample_db(db_file.path());

    let db = GeoDatabase::open(db_file.path()).unwrap();
    assert_eq!(db.lookup("1.0.0.0").unwrap(), "Australia"); // first address in range
    assert_eq!(db.lookup("1.0.0.255").unwrap(), "Australia"); // last address in range
    assert_eq!(db.lookup("9.9.9.9").unwrap(), "United States"); // /32 host route
}

#[test]
fn ip_just_outside_a_range_is_unknown() {
    let db_file = TempDbFile::new("just_outside");
    build_sample_db(db_file.path());

    let db = GeoDatabase::open(db_file.path()).unwrap();
    assert_eq!(db.lookup("1.0.1.0").unwrap(), "Unknown"); // one past the /24
    assert_eq!(db.lookup("0.255.255.255").unwrap(), "Unknown"); // one before the /24
}

#[test]
fn unmapped_ip_returns_unknown_not_an_error() {
    let db_file = TempDbFile::new("unmapped");
    build_sample_db(db_file.path());

    let db = GeoDatabase::open(db_file.path()).unwrap();
    assert_eq!(db.lookup("192.168.1.1").unwrap(), "Unknown");
}

#[test]
fn invalid_ip_string_is_an_error() {
    let db_file = TempDbFile::new("invalid_ip");
    build_sample_db(db_file.path());

    let db = GeoDatabase::open(db_file.path()).unwrap();
    let err = db.lookup("not-an-ip").unwrap_err();
    assert!(matches!(err, GeoError::InvalidIp(_)));

    let err = db.lookup("999.999.999.999").unwrap_err();
    assert!(matches!(err, GeoError::InvalidIp(_)));
}

#[test]
fn empty_database_resolves_everything_to_unknown() {
    let db_file = TempDbFile::new("empty");
    write_database(db_file.path(), &sample_countries(), &[]).unwrap();

    let db = GeoDatabase::open(db_file.path()).unwrap();
    assert_eq!(db.range_count(), 0);
    assert_eq!(db.lookup("8.8.8.8").unwrap(), "Unknown");
}

#[test]
fn rejects_missing_file() {
    let err = GeoDatabase::open("/nonexistent/path/geolite.bin").unwrap_err();
    assert!(matches!(err, GeoError::Io(_)));
}

#[test]
fn rejects_corrupt_magic_bytes() {
    let db_file = TempDbFile::new("corrupt_magic");
    build_sample_db(db_file.path());

    let mut bytes = std::fs::read(db_file.path()).unwrap();
    bytes[0] = b'X';
    std::fs::write(db_file.path(), &bytes).unwrap();

    let err = GeoDatabase::open(db_file.path()).unwrap_err();
    assert!(matches!(err, GeoError::CorruptDatabase(_)));
}

#[test]
fn rejects_version_mismatch() {
    let db_file = TempDbFile::new("version_mismatch");
    build_sample_db(db_file.path());

    let mut bytes = std::fs::read(db_file.path()).unwrap();
    // Version is the little-endian u32 immediately after the 4-byte magic.
    bytes[4..8].copy_from_slice(&999u32.to_le_bytes());
    std::fs::write(db_file.path(), &bytes).unwrap();

    let err = GeoDatabase::open(db_file.path()).unwrap_err();
    assert!(matches!(err, GeoError::VersionMismatch { found: 999, .. }));
}

#[test]
fn rejects_truncated_range_table() {
    let db_file = TempDbFile::new("truncated");
    build_sample_db(db_file.path());

    let bytes = std::fs::read(db_file.path()).unwrap();
    let truncated = &bytes[..bytes.len() - 4]; // chop part of the last range record
    std::fs::write(db_file.path(), truncated).unwrap();

    let err = GeoDatabase::open(db_file.path()).unwrap_err();
    assert!(matches!(err, GeoError::CorruptDatabase(_)));
}

#[test]
fn rejects_truncated_header() {
    let db_file = TempDbFile::new("truncated_header");
    build_sample_db(db_file.path());

    let bytes = std::fs::read(db_file.path()).unwrap();
    std::fs::write(db_file.path(), &bytes[..10]).unwrap(); // shorter than the 24-byte header
    let err = GeoDatabase::open(db_file.path()).unwrap_err();
    assert!(matches!(err, GeoError::CorruptDatabase(_)));
}

#[test]
fn write_database_rejects_missing_unknown_entry() {
    let db_file = TempDbFile::new("no_unknown");
    let countries = vec!["Australia".to_string()]; // missing reserved "Unknown" at 0
    let err = write_database(db_file.path(), &countries, &[]).unwrap_err();
    assert!(matches!(err, GeoError::CorruptDatabase(_)));
}

#[test]
fn write_database_rejects_out_of_range_country_index() {
    let db_file = TempDbFile::new("bad_country_index");
    let countries = vec!["Unknown".to_string()];
    let ranges = vec![IpRange { start: 0, end: 10, country_index: 5 }];
    let err = write_database(db_file.path(), &countries, &ranges).unwrap_err();
    assert!(matches!(err, GeoError::CorruptDatabase(_)));
}

#[test]
fn write_database_accepts_unsorted_input_and_sorts_it() {
    let db_file = TempDbFile::new("unsorted");
    let countries = sample_countries();
    let mut ranges = sample_ranges();
    ranges.reverse(); // deliberately out of order

    write_database(db_file.path(), &countries, &ranges).unwrap();
    let db = GeoDatabase::open(db_file.path()).unwrap();
    assert_eq!(db.lookup("8.8.8.8").unwrap(), "United States");
    assert_eq!(db.lookup("1.0.0.0").unwrap(), "Australia");
}
