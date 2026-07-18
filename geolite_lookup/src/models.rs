//! Core data structures and binary format constants shared between the
//! database builder (`src/builder.rs`) and the runtime reader
//! (`src/database.rs`).
//!
//! Keeping these in one place means the writer and the reader can never
//! silently drift apart on layout.

/// Magic bytes identifying a valid `geolite.bin` database file.
pub const MAGIC: &[u8; 4] = b"GLDB";

/// Current on-disk format version. Bump this if the layout below changes,
/// and `GeoDatabase::open` will refuse to read files written with a
/// different version instead of misinterpreting them.
pub const VERSION: u32 = 1;

/// Fixed size of the file header, in bytes:
/// `magic(4) + version(4) + country_count(4) + range_count(4) + range_table_offset(8)`
pub const HEADER_SIZE: usize = 24;

/// On-disk size of a single IP range record: `start(4) + end(4) + country_index(2)`.
pub const RANGE_RECORD_SIZE: usize = 10;

/// Country table index reserved for ranges whose country could not be
/// determined at build time (e.g. missing `geoname_id`).
pub const UNKNOWN_COUNTRY_INDEX: u16 = 0;

/// Display name used for the reserved "unknown" country slot. Every
/// database written by [`crate::write_database`] must have this as
/// country index 0.
pub const UNKNOWN_COUNTRY_NAME: &str = "Unknown";

/// A single IPv4 address range mapped to a country, as stored on disk.
///
/// `start` and `end` are inclusive bounds, in host byte order, produced by
/// expanding a CIDR block (e.g. `1.0.0.0/24` -> `start = 16_777_216`,
/// `end = 16_777_471`). Individual addresses are never materialized: only
/// these two bounds are stored, regardless of how large the block is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpRange {
    pub start: u32,
    pub end: u32,
    pub country_index: u16,
}

impl IpRange {
    /// Whether `addr` falls within this inclusive range.
    #[inline]
    pub fn contains(&self, addr: u32) -> bool {
        addr >= self.start && addr <= self.end
    }
}

/// Parsed representation of the fixed-size file header.
#[derive(Debug, Clone, Copy)]
pub struct Header {
    pub version: u32,
    pub country_count: u32,
    pub range_count: u32,
    /// Absolute byte offset into the file where the range table begins
    /// (i.e. immediately after the variable-length country table).
    pub range_table_offset: u64,
}
