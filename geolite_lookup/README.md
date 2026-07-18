# geolite_lookup

A high-performance, offline IPv4 → country lookup library and CLI, backed by
a memory-mapped, CIDR/range-based binary database built from the free
MaxMind GeoLite2 Country CSV files.

- No runtime CSV parsing — CSV is only read once, at build time.
- No CIDR expansion — a `/8` is stored as two `u32`s, never as 16.7 million addresses.
- No hash map at lookup time — ranges are binary-searched in `O(log n)`.
- No full-file heap copy — the database is `mmap`'d; the OS pages it in on demand.

```
$ cargo run --release --bin geolite_lookup -- geolite.bin 8.8.8.8
8.8.8.8 -> United States
```

## Project layout

```
geolite_lookup/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs        # public API surface (GeoDatabase, write_database, errors)
│   ├── models.rs      # IpRange, Header, binary format constants
│   ├── error.rs       # GeoError (thiserror)
│   ├── lookup.rs       # the O(log n) binary search, storage-agnostic
│   ├── database.rs    # mmap reader (GeoDatabase) + binary format writer
│   ├── builder.rs      # bin: CSV -> geolite.bin (the ONLY place CSV is parsed)
│   └── main.rs         # bin: geolite.bin -> lookup results (no CSV, ever)
└── tests/
    └── lookup_tests.rs # integration tests against the public API
```

Two binaries come out of this one crate:

| Binary            | Purpose                                             |
|--------------------|------------------------------------------------------|
| `builder`          | One-time/offline: CSVs → `geolite.bin`               |
| `geolite_lookup`   | Runtime: opens `geolite.bin`, answers lookups        |

`database.rs`'s writer (`write_database`) is shared by both `builder.rs` and
the integration tests, so the format the builder writes and the format the
tests validate against can never silently drift apart.

## Build instructions

```bash
cargo build --release
```

Requires only the crates listed in `Cargo.toml` (`csv`, `serde`, `memmap2`,
`byteorder`, `thiserror`) — no system dependencies.

## Usage

### 1. Get the data

Download `GeoLite2-Country-CSV.zip` from MaxMind (free account required)
and unzip it. You need:

- `GeoLite2-Country-Blocks-IPv4.csv`
- `GeoLite2-Country-Locations-en.csv`

### 2. Build the database (once, offline)

```bash
cargo run --release --bin builder -- \
    GeoLite2-Country-Blocks-IPv4.csv \
    GeoLite2-Country-Locations-en.csv \
    geolite.bin
```

This is the only step that touches CSV or expands a CIDR block. Progress
and a final summary (range count, distinct country count) are printed to
stderr.

### 3. Look things up (repeatedly, fast)

CLI:

```bash
cargo run --release --bin geolite_lookup -- geolite.bin 8.8.8.8 1.1.1.1
8.8.8.8 -> United States
1.1.1.1 -> Australia
```

As a library:

```rust
use geolite_lookup::GeoDatabase;

fn main() -> geolite_lookup::Result<()> {
    let db = GeoDatabase::open("geolite.bin")?;
    let country = db.lookup("8.8.8.8")?;
    println!("{country}"); // "United States"
    Ok(())
}
```

`GeoDatabase::open` mmaps the file once; `lookup` (and the lower-level
`lookup_addr` / `lookup_u32`) can then be called millions of times with no
further I/O, allocation, or parsing. `GeoDatabase` is `Sync`-friendly to
share behind an `Arc` across threads, since `Mmap` itself is safe to read
from multiple threads concurrently.

An IP that doesn't fall into any known range (private/reserved space, or
simply not in the dataset) resolves to `"Unknown"` rather than an error;
`Err` is reserved for malformed input or a corrupt database file.

## Binary file format

```
+--------------------------------------------------------------+
| HEADER (24 bytes, little-endian)                             |
|   magic:               [u8; 4]  "GLDB"                       |
|   version:              u32                                  |
|   country_count:        u32                                  |
|   range_count:          u32                                  |
|   range_table_offset:   u64     (byte offset of range table) |
+--------------------------------------------------------------+
| COUNTRY TABLE (variable length, country_count entries)       |
|   for each country:                                          |
|     name_len: u16                                            |
|     name_bytes: [u8; name_len]  (UTF-8, no terminator)        |
|   index 0 is always reserved for "Unknown"                   |
+--------------------------------------------------------------+
| RANGE TABLE (range_count * 10 bytes, sorted by start)        |
|   for each range:                                             |
|     start:         u32                                       |
|     end:           u32   (inclusive)                         |
|     country_index: u16   (index into the country table)      |
+--------------------------------------------------------------+
```

Design notes:

- **Country names are stored once**, in a small table, and referenced from
  range records by a 2-byte index — never duplicated per-range. For the
  full GeoLite2 Country dataset that's on the order of a few hundred
  strings versus ~450k+ IP ranges, so this matters a lot.
- **`range_table_offset` is stored explicitly** in the header rather than
  computed by summing the country table on every open. `GeoDatabase::open`
  still parses the country table and *cross-checks* that it ends exactly at
  `range_table_offset` — if it doesn't, the file is flagged corrupt. This
  gives a cheap, free consistency check on every open.
- **10-byte range records, unpadded.** Fields are read with explicit
  little-endian accessors (`byteorder`), not pointer casts, so there's no
  alignment requirement on the mapped bytes and no risk of unaligned-access
  undefined behavior.
- **Ranges are inclusive `[start, end]`**, sorted by `start`, and must not
  overlap. `write_database` sorts its input for you but does not currently
  merge adjacent/overlapping ranges — the source CSV data is expected to
  already be well-formed in that respect.

## Memory mapping

`GeoDatabase::open` uses `memmap2::Mmap` instead of `std::fs::read`:

```rust
let mmap = unsafe { MmapOptions::new().map(&file)? };
```

- The **country table** (small — a few hundred entries at most) is parsed
  once into a `Vec<String>` at open time, for O(1) name lookups.
- The **range table** (potentially hundreds of thousands of records) is
  *never* copied into a `Vec`. Every `GeoDatabase::lookup` call reads
  10-byte records directly out of the memory-mapped pages via
  `GeoDatabase::range_at`. The OS pages in only the parts of the file that
  are actually touched by the binary search — typically a handful of
  4 KiB pages per lookup — and the page cache is shared across processes
  that open the same file, so running many instances of a service against
  the same `geolite.bin` costs a small, roughly constant amount of extra
  RAM per process instead of a full copy each.

The `unsafe` at the `Mmap::map` call site is the standard, well-understood
caveat of memory-mapped I/O: the OS can't guarantee the backing file won't
be modified or truncated by another process while it's mapped. Every read
after that point uses bounds-checked slice indexing and `byteorder`, not
raw pointer casts, so a corrupt or truncated file produces a clean
`GeoError` (verified at `open()` time and defensively on every range read)
rather than undefined behavior.

## Lookup algorithm

`src/lookup.rs` implements the search independently of any storage
backend:

```rust
pub fn binary_search_range<F>(count: usize, target: u32, range_at: F) -> Option<usize>
where
    F: Fn(usize) -> IpRange,
```

Given `count` ranges (accessed lazily through the `range_at` closure),
sorted by `start` and non-overlapping, it runs the classic three-way
binary search for an *interval* containing `target`:

1. Look at the midpoint range.
2. If `target` is below its `start`, discard the upper half.
3. If `target` is above its `end`, discard the lower half.
4. Otherwise `target` is inside this range — done.

This is `O(log n)` comparisons and performs **no allocation**. Because the
accessor is a closure rather than a concrete `&[IpRange]`, the exact same
function backs both the in-memory unit tests in `lookup.rs` and
`GeoDatabase::range_at`'s direct reads from the `mmap`.

## Testing

```bash
cargo test
```

- **`src/lookup.rs`** unit tests the binary search in isolation (middle of
  a range, exact start/end boundaries, a single-address `/32` range, gaps
  between ranges, an empty table).
- **`src/builder.rs`** unit tests CIDR parsing (`/0`, `/24`, `/32`, invalid
  prefixes, malformed IPs) and the geoname-fallback logic used when a
  block's primary `geoname_id` column is blank.
- **`tests/lookup_tests.rs`** integration-tests the public API end to end:
  building a small database with `write_database`, then exercising known
  lookups, inclusive range boundaries, unmapped IPs resolving to
  `"Unknown"`, invalid IP strings, a missing file, corrupt magic bytes, a
  version mismatch, and a truncated file — each producing the expected
  `GeoError` variant instead of a panic.

All of this was verified against this exact repository: `cargo build`,
`cargo build --release` (with LTO), `cargo test` (30 tests, all passing),
and an end-to-end run of `builder` → `geolite_lookup` against a small
hand-built CSV pair mirroring the real MaxMind schema (including an extra
`is_in_european_union` column and rows with missing `geoname_id`, to check
the fallback and "Unknown" bucketing).

## Error handling

`GeoError` (`src/error.rs`, via `thiserror`) covers:

- `Io` — file open/read/write failures
- `Csv` — malformed CSV rows (builder only)
- `InvalidIp` — a `lookup()` argument that isn't a valid dotted-quad IPv4 address
- `InvalidCidr` — a malformed `network` column (builder only)
- `CorruptDatabase` — bad magic, truncated tables, a country/range table
  that doesn't line up with the header, or non-UTF-8 country names
- `VersionMismatch` — file was written by a different format version
- `CountryIndexOutOfRange` — a range points at a country index past the
  end of the country table (only possible via a hand-corrupted file, since
  `write_database` validates this before writing)

## IPv6

Not supported, by design, per the original scope (IPv4 only).
