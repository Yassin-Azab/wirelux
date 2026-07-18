//! Reading and writing the `geolite.bin` binary format.
//!
//! # Binary format
//!
//! ```text
//! +--------------------------------------------------------------+
//! | HEADER (24 bytes, little-endian)                             |
//! |   magic:               [u8; 4]  "GLDB"                       |
//! |   version:              u32                                  |
//! |   country_count:        u32                                  |
//! |   range_count:          u32                                  |
//! |   range_table_offset:   u64     (byte offset of range table) |
//! +--------------------------------------------------------------+
//! | COUNTRY TABLE (variable length, country_count entries)       |
//! |   for each country:                                          |
//! |     name_len: u16                                            |
//! |     name_bytes: [u8; name_len]  (UTF-8, no terminator)        |
//! |   index 0 is always reserved for "Unknown"                   |
//! +--------------------------------------------------------------+
//! | RANGE TABLE (range_count * 10 bytes, sorted by start)        |
//! |   for each range:                                             |
//! |     start:         u32                                       |
//! |     end:           u32   (inclusive)                         |
//! |     country_index: u16   (index into the country table)      |
//! +--------------------------------------------------------------+
//! ```
//!
//! The country table is small (at most a few hundred entries for
//! GeoLite2 Country data) and is parsed into a `Vec<String>` once, at
//! `open()` time. The range table can hold hundreds of thousands of
//! records, so it is *never* copied into a `Vec`: `GeoDatabase` keeps the
//! file memory-mapped and reads each 10-byte record directly out of the
//! mapped pages on demand, during the binary search. This is what lets
//! multiple processes share the same physical pages and keeps per-process
//! RAM usage close to O(1) regardless of database size.

use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::net::Ipv4Addr;
use std::path::Path;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use memmap2::{Mmap, MmapOptions};

use crate::error::{GeoError, Result};
use crate::lookup::binary_search_range;
use crate::models::{
    Header, IpRange, HEADER_SIZE, MAGIC, RANGE_RECORD_SIZE, UNKNOWN_COUNTRY_INDEX,
    UNKNOWN_COUNTRY_NAME, VERSION,
};

/// A read-only, memory-mapped IPv4 -> country lookup database.
#[derive(Debug)]
pub struct GeoDatabase {
    // Kept alive for as long as the database is open; range lookups read
    // directly out of this mapping rather than a heap-allocated copy.
    mmap: Mmap,
    countries: Vec<String>,
    range_table_offset: usize,
    range_count: usize,
}

impl GeoDatabase {
    /// Opens and validates a database file, memory-mapping it.
    ///
    /// This parses and validates the header and the (small) country table
    /// eagerly, but does *not* touch the range table beyond checking that
    /// the file is long enough to hold it -- range records are read lazily
    /// on each [`GeoDatabase::lookup`] call.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;

        // Safety: `Mmap::map` is unsafe because the OS cannot guarantee the
        // backing file won't be truncated or modified by another process
        // while it's mapped, which could turn out-of-bounds reads (which
        // we otherwise carefully avoid) into undefined behavior. We accept
        // this standard, well-understood tradeoff in exchange for O(1)
        // memory usage regardless of file size; the file is intended to be
        // a build artifact that isn't mutated while a reader has it open.
        let mmap = unsafe { MmapOptions::new().map(&file)? };

        if mmap.len() < HEADER_SIZE {
            return Err(GeoError::CorruptDatabase(format!(
                "file is only {} bytes, smaller than the {HEADER_SIZE}-byte header",
                mmap.len()
            )));
        }

        let header = Self::parse_header(&mmap)?;

        let (countries, table_end) = Self::parse_country_table(&mmap, header.country_count)?;

        if table_end as u64 != header.range_table_offset {
            return Err(GeoError::CorruptDatabase(format!(
                "country table ends at byte {table_end}, but header declares the range table \
                 starts at byte {}",
                header.range_table_offset
            )));
        }

        let range_table_offset = header.range_table_offset as usize;
        let range_count = header.range_count as usize;
        let required_len = range_table_offset
            .checked_add(range_count.saturating_mul(RANGE_RECORD_SIZE))
            .ok_or_else(|| GeoError::CorruptDatabase("range table size overflows".into()))?;

        if mmap.len() < required_len {
            return Err(GeoError::CorruptDatabase(format!(
                "file is {} bytes but the range table needs {required_len}",
                mmap.len()
            )));
        }

        Ok(Self {
            mmap,
            countries,
            range_table_offset,
            range_count,
        })
    }

    /// Number of distinct country names in the database (including the
    /// reserved "Unknown" entry at index 0).
    pub fn country_count(&self) -> usize {
        self.countries.len()
    }

    /// Number of IP ranges in the database.
    pub fn range_count(&self) -> usize {
        self.range_count
    }

    /// Looks up the country for a dotted-quad IPv4 address string, e.g.
    /// `"8.8.8.8"`.
    ///
    /// Returns `Ok("Unknown")` (rather than an error) for addresses that
    /// don't fall in any range in the database -- this is the expected,
    /// non-exceptional outcome for private, reserved, or otherwise
    /// unallocated ranges. It returns `Err` only for malformed input or a
    /// corrupt database.
    pub fn lookup(&self, ip: &str) -> Result<&str> {
        let addr: Ipv4Addr = ip.parse().map_err(|_| GeoError::InvalidIp(ip.to_string()))?;
        self.lookup_addr(addr)
    }

    /// Same as [`GeoDatabase::lookup`] but takes an already-parsed
    /// [`Ipv4Addr`], skipping string parsing.
    pub fn lookup_addr(&self, addr: Ipv4Addr) -> Result<&str> {
        self.lookup_u32(u32::from(addr))
    }

    /// Same as [`GeoDatabase::lookup`] but takes a raw host-byte-order u32,
    /// skipping string parsing entirely. This is the hot path used by the
    /// other two.
    pub fn lookup_u32(&self, addr: u32) -> Result<&str> {
        let country_index = match binary_search_range(self.range_count, addr, |i| self.range_at(i)) {
            Some(idx) => self.range_at(idx).country_index,
            None => UNKNOWN_COUNTRY_INDEX,
        };
        self.country_name(country_index)
    }

    /// Reads the `idx`-th range record directly out of the memory-mapped
    /// file. Bounds for the whole range table were already validated in
    /// `open()`, and `idx` is only ever produced by `binary_search_range`
    /// operating over `0..self.range_count`, so this never reads out of
    /// bounds in practice; we still use checked reads (rather than
    /// `unsafe` pointer casts) so a logic bug here fails loudly instead of
    /// causing undefined behavior.
    fn range_at(&self, idx: usize) -> IpRange {
        debug_assert!(idx < self.range_count, "range index out of bounds");
        let offset = self.range_table_offset + idx * RANGE_RECORD_SIZE;
        let mut cursor = &self.mmap[offset..offset + RANGE_RECORD_SIZE];
        let start = cursor
            .read_u32::<LittleEndian>()
            .expect("range record size was validated at open()");
        let end = cursor
            .read_u32::<LittleEndian>()
            .expect("range record size was validated at open()");
        let country_index = cursor
            .read_u16::<LittleEndian>()
            .expect("range record size was validated at open()");
        IpRange { start, end, country_index }
    }

    fn country_name(&self, index: u16) -> Result<&str> {
        self.countries
            .get(index as usize)
            .map(String::as_str)
            .ok_or(GeoError::CountryIndexOutOfRange(index))
    }

    fn parse_header(mmap: &Mmap) -> Result<Header> {
        let mut cursor: &[u8] = &mmap[0..HEADER_SIZE];

        let mut magic = [0u8; 4];
        cursor.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(GeoError::CorruptDatabase(format!(
                "bad magic bytes: expected {MAGIC:?}, found {magic:?}"
            )));
        }

        let version = cursor.read_u32::<LittleEndian>()?;
        if version != VERSION {
            return Err(GeoError::version_mismatch(version));
        }

        let country_count = cursor.read_u32::<LittleEndian>()?;
        let range_count = cursor.read_u32::<LittleEndian>()?;
        let range_table_offset = cursor.read_u64::<LittleEndian>()?;

        Ok(Header {
            version,
            country_count,
            range_count,
            range_table_offset,
        })
    }

    /// Parses `country_count` length-prefixed UTF-8 strings starting right
    /// after the header, returning the parsed table and the byte offset
    /// where the table ended (which should equal the header's
    /// `range_table_offset` -- checked by the caller as a corruption check).
    fn parse_country_table(mmap: &Mmap, country_count: u32) -> Result<(Vec<String>, usize)> {
        let mut offset = HEADER_SIZE;
        let mut countries = Vec::with_capacity(country_count as usize);

        for _ in 0..country_count {
            if offset + 2 > mmap.len() {
                return Err(GeoError::CorruptDatabase(
                    "country table truncated: missing a name-length prefix".into(),
                ));
            }
            let len = (&mmap[offset..offset + 2]).read_u16::<LittleEndian>()? as usize;
            offset += 2;

            if offset + len > mmap.len() {
                return Err(GeoError::CorruptDatabase(
                    "country table truncated: missing name bytes".into(),
                ));
            }
            let name = std::str::from_utf8(&mmap[offset..offset + len])
                .map_err(|_| GeoError::CorruptDatabase("country name is not valid UTF-8".into()))?
                .to_string();
            offset += len;

            countries.push(name);
        }

        Ok((countries, offset))
    }
}

/// Serializes a country table and a set of IP ranges into the `geolite.bin`
/// binary format documented at the top of this module.
///
/// `countries[0]` must be the reserved `"Unknown"` entry (see
/// [`crate::models::UNKNOWN_COUNTRY_NAME`]); every `IpRange::country_index`
/// must be a valid index into `countries`. `ranges` does not need to be
/// pre-sorted -- this function sorts a local copy by `start` before
/// writing, since that ordering is what makes the runtime binary search
/// correct.
///
/// This is used by `src/builder.rs` (the CSV -> binary converter) and
/// directly by the integration tests in `tests/lookup_tests.rs`, so the
/// two never risk drifting apart on what a "valid" file looks like.
pub fn write_database<P: AsRef<Path>>(
    path: P,
    countries: &[String],
    ranges: &[IpRange],
) -> Result<()> {
    if countries.is_empty() || countries[0] != UNKNOWN_COUNTRY_NAME {
        return Err(GeoError::CorruptDatabase(format!(
            "country table must start with the reserved \"{UNKNOWN_COUNTRY_NAME}\" entry at index 0"
        )));
    }
    if countries.len() > u32::MAX as usize {
        return Err(GeoError::CorruptDatabase("too many countries to index".into()));
    }
    if ranges.len() > u32::MAX as usize {
        return Err(GeoError::CorruptDatabase("too many ranges to index".into()));
    }
    for r in ranges {
        if r.country_index as usize >= countries.len() {
            return Err(GeoError::CorruptDatabase(format!(
                "range [{}, {}] references country index {}, but the country table only has {} entries",
                r.start, r.end, r.country_index, countries.len()
            )));
        }
    }

    let mut sorted = ranges.to_vec();
    sorted.sort_by_key(|r| r.start);

    let mut country_bytes_len: u64 = 0;
    for name in countries {
        if name.len() > u16::MAX as usize {
            return Err(GeoError::CorruptDatabase(format!(
                "country name '{name}' is longer than {} bytes",
                u16::MAX
            )));
        }
        country_bytes_len += 2 + name.len() as u64;
    }
    let range_table_offset = HEADER_SIZE as u64 + country_bytes_len;

    let file = File::create(path)?;
    let mut w = BufWriter::new(file);

    w.write_all(MAGIC)?;
    w.write_u32::<LittleEndian>(VERSION)?;
    w.write_u32::<LittleEndian>(countries.len() as u32)?;
    w.write_u32::<LittleEndian>(sorted.len() as u32)?;
    w.write_u64::<LittleEndian>(range_table_offset)?;

    for name in countries {
        w.write_u16::<LittleEndian>(name.len() as u16)?;
        w.write_all(name.as_bytes())?;
    }

    for r in &sorted {
        w.write_u32::<LittleEndian>(r.start)?;
        w.write_u32::<LittleEndian>(r.end)?;
        w.write_u16::<LittleEndian>(r.country_index)?;
    }

    w.flush()?;
    Ok(())
}
