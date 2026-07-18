//! `geolite_lookup`: a high-performance, offline IPv4 -> country lookup
//! library.
//!
//! A one-time build step (`cargo run --release --bin builder`) converts the
//! MaxMind GeoLite2 Country CSV files into a single compact binary file,
//! `geolite.bin`. At runtime, [`GeoDatabase::open`] memory-maps that file
//! and [`GeoDatabase::lookup`] resolves an IPv4 address to a country name
//! in `O(log n)` time with no heap allocation per lookup and no CSV parsing.
//!
//! ```no_run
//! use geolite_lookup::GeoDatabase;
//!
//! # fn main() -> geolite_lookup::Result<()> {
//! let db = GeoDatabase::open("geolite.bin")?;
//! let country = db.lookup("8.8.8.8")?;
//! println!("{country}");
//! # Ok(())
//! # }
//! ```

pub mod database;
pub mod error;
pub mod lookup;
pub mod models;

pub use database::{write_database, GeoDatabase};
pub use error::{GeoError, Result};
pub use models::IpRange;
