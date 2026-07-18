//! Error handling for `geolite_lookup`.

use std::io;

use thiserror::Error;

use crate::models::VERSION;

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, GeoError>;

/// Everything that can go wrong building, opening, or querying a database.
#[derive(Debug, Error)]
pub enum GeoError {
    /// Underlying filesystem I/O failed (open, read, write, ...).
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// A CSV file could not be parsed (used only by the builder).
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    /// The string handed to `lookup` is not a valid dotted-quad IPv4 address.
    #[error("invalid IPv4 address: '{0}'")]
    InvalidIp(String),

    /// The string handed to the builder is not valid CIDR notation
    /// (e.g. malformed IP, missing prefix, or prefix > 32).
    #[error("invalid CIDR notation: '{0}'")]
    InvalidCidr(String),

    /// The `.bin` file failed structural validation: bad magic bytes, a
    /// truncated table, a table that doesn't line up with the header, or a
    /// country name that isn't valid UTF-8. This is the catch-all for
    /// "this file is not a database we can trust."
    #[error("corrupt database: {0}")]
    CorruptDatabase(String),

    /// The file has valid magic bytes but was written by a different,
    /// incompatible format version.
    #[error("unsupported database version: found {found}, this build supports {expected}")]
    VersionMismatch { found: u32, expected: u32 },

    /// A range record pointed at a country index that doesn't exist in the
    /// country table. This indicates a corrupt or hand-edited file, since
    /// `write_database` never produces one.
    #[error("country index {0} out of range")]
    CountryIndexOutOfRange(u16),
}

impl GeoError {
    /// Shorthand used internally when validating a header's version field.
    pub(crate) fn version_mismatch(found: u32) -> Self {
        GeoError::VersionMismatch {
            found,
            expected: VERSION,
        }
    }
}
