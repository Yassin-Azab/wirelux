//! `cargo run --release --bin builder -- <blocks.csv> <locations.csv> <geolite.bin>`
//!
//! Converts MaxMind's `GeoLite2-Country-Blocks-IPv4.csv` and
//! `GeoLite2-Country-Locations-en.csv` into a single `geolite.bin` file
//! using [`geolite_lookup::write_database`]. This is the *only* place in
//! the whole project that parses CSV or expands a CIDR block; the runtime
//! reader never does either.

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::net::Ipv4Addr;
use std::process;

use geolite_lookup::models::{IpRange, UNKNOWN_COUNTRY_NAME};
use geolite_lookup::{write_database, GeoError, Result};

/// One row of `GeoLite2-Country-Locations-en.csv`. Field names match the
/// CSV header exactly, since `csv`'s serde support maps by header name; any
/// extra columns present in newer MaxMind exports (e.g.
/// `is_in_european_union`) are simply ignored.
#[derive(Debug, serde::Deserialize)]
struct LocationRecord {
    geoname_id: String,
    #[allow(dead_code)]
    locale_code: String,
    #[allow(dead_code)]
    continent_code: String,
    #[allow(dead_code)]
    continent_name: String,
    #[allow(dead_code)]
    country_iso_code: String,
    country_name: String,
}

/// One row of `GeoLite2-Country-Blocks-IPv4.csv`.
#[derive(Debug, serde::Deserialize)]
struct BlockRecord {
    network: String,
    geoname_id: String,
    registered_country_geoname_id: String,
    represented_country_geoname_id: String,
    #[allow(dead_code)]
    is_anonymous_proxy: String,
    #[allow(dead_code)]
    is_satellite_provider: String,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        let program = args.first().map(String::as_str).unwrap_or("builder");
        eprintln!(
            "usage: {program} <GeoLite2-Country-Blocks-IPv4.csv> <GeoLite2-Country-Locations-en.csv> <geolite.bin>"
        );
        process::exit(2);
    }
    let blocks_path = &args[1];
    let locations_path = &args[2];
    let output_path = &args[3];

    eprintln!("loading locations from {locations_path} ...");
    let geoname_to_country = load_locations(locations_path)?;
    eprintln!(
        "loaded {} geoname_id -> country_name mappings",
        geoname_to_country.len()
    );

    eprintln!("loading blocks from {blocks_path} ...");
    let (countries, ranges) = load_blocks(blocks_path, &geoname_to_country)?;
    eprintln!(
        "parsed {} IP ranges across {} distinct countries",
        ranges.len(),
        countries.len()
    );

    eprintln!("writing {output_path} ...");
    write_database(output_path, &countries, &ranges)?;
    eprintln!("done: {output_path}");
    Ok(())
}

/// Builds a `geoname_id -> country_name` map from the locations CSV. Rows
/// with an empty `country_name` (continent-level or otherwise unresolved
/// entries) are skipped; blocks that resolve to such a `geoname_id` will
/// fall back through `resolve_country` and ultimately land in "Unknown".
fn load_locations(path: &str) -> Result<HashMap<String, String>> {
    let file = File::open(path)?;
    let mut reader = csv::Reader::from_reader(file);
    let mut map = HashMap::new();

    for result in reader.deserialize() {
        let record: LocationRecord = result?;
        if record.country_name.trim().is_empty() {
            continue;
        }
        map.insert(record.geoname_id, record.country_name);
    }

    Ok(map)
}

/// Reads the blocks CSV, resolving each row to a country and expanding its
/// CIDR into a `[start, end]` range. Returns the deduplicated country
/// table (with "Unknown" fixed at index 0) and the list of ranges, sorted
/// by start address.
fn load_blocks(
    path: &str,
    geoname_to_country: &HashMap<String, String>,
) -> Result<(Vec<String>, Vec<IpRange>)> {
    let file = File::open(path)?;
    let mut reader = csv::Reader::from_reader(file);

    let mut countries: Vec<String> = vec![UNKNOWN_COUNTRY_NAME.to_string()];
    let mut country_indices: HashMap<String, u16> = HashMap::new();
    country_indices.insert(UNKNOWN_COUNTRY_NAME.to_string(), 0);

    let mut ranges = Vec::new();
    let mut skipped = 0u32;

    for result in reader.deserialize() {
        let record: BlockRecord = result?;

        let (start, end) = match parse_cidr(&record.network) {
            Ok(bounds) => bounds,
            Err(_) => {
                eprintln!("skipping malformed network '{}'", record.network);
                skipped += 1;
                continue;
            }
        };

        let country_name = resolve_country(&record, geoname_to_country);
        let country_index = *country_indices
            .entry(country_name.clone())
            .or_insert_with(|| {
                let idx = countries.len() as u16;
                countries.push(country_name.clone());
                idx
            });

        ranges.push(IpRange { start, end, country_index });
    }

    if skipped > 0 {
        eprintln!("skipped {skipped} malformed row(s)");
    }

    ranges.sort_by_key(|r| r.start);
    Ok((countries, ranges))
}

/// Resolves a block row to a country name, falling back from the most
/// specific `geoname_id` column to the least specific, then to "Unknown".
/// This mirrors how MaxMind expects consumers to handle rows where the
/// primary `geoname_id` column is blank (common for anonymizing/satellite
/// ranges where only `registered_country_geoname_id` is populated).
fn resolve_country(record: &BlockRecord, geoname_to_country: &HashMap<String, String>) -> String {
    for candidate in [
        &record.geoname_id,
        &record.registered_country_geoname_id,
        &record.represented_country_geoname_id,
    ] {
        if candidate.trim().is_empty() {
            continue;
        }
        if let Some(name) = geoname_to_country.get(candidate) {
            return name.clone();
        }
    }
    UNKNOWN_COUNTRY_NAME.to_string()
}

/// Parses `a.b.c.d/prefix` into an inclusive `[start, end]` pair of u32
/// host addresses, without ever materializing the individual addresses in
/// between.
fn parse_cidr(cidr: &str) -> Result<(u32, u32)> {
    let (ip_part, prefix_part) = cidr
        .split_once('/')
        .ok_or_else(|| GeoError::InvalidCidr(cidr.to_string()))?;

    let ip: Ipv4Addr = ip_part
        .parse()
        .map_err(|_| GeoError::InvalidCidr(cidr.to_string()))?;
    let prefix: u32 = prefix_part
        .parse()
        .map_err(|_| GeoError::InvalidCidr(cidr.to_string()))?;
    if prefix > 32 {
        return Err(GeoError::InvalidCidr(cidr.to_string()));
    }

    let ip_u32 = u32::from(ip);
    let host_bits = 32 - prefix;
    let mask: u32 = if host_bits == 32 { 0 } else { !0u32 << host_bits };
    let start = ip_u32 & mask;
    let end = start | !mask;
    Ok((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_slash_24() {
        assert_eq!(parse_cidr("1.0.0.0/24").unwrap(), (16_777_216, 16_777_471));
    }

    #[test]
    fn parses_slash_32_as_single_address() {
        let (start, end) = parse_cidr("8.8.8.8/32").unwrap();
        assert_eq!(start, end);
        assert_eq!(start, u32::from(Ipv4Addr::new(8, 8, 8, 8)));
    }

    #[test]
    fn parses_slash_0_as_entire_space() {
        let (start, end) = parse_cidr("0.0.0.0/0").unwrap();
        assert_eq!(start, 0);
        assert_eq!(end, u32::MAX);
    }

    #[test]
    fn rejects_prefix_over_32() {
        assert!(parse_cidr("1.2.3.4/33").is_err());
    }

    #[test]
    fn rejects_missing_prefix() {
        assert!(parse_cidr("1.2.3.4").is_err());
    }

    #[test]
    fn rejects_garbage_ip() {
        assert!(parse_cidr("not.an.ip.addr/24").is_err());
    }

    #[test]
    fn resolves_country_falls_back_through_geoname_columns() {
        let mut map = HashMap::new();
        map.insert("100".to_string(), "Australia".to_string());

        // Primary geoname_id is blank, registered_country_geoname_id has it.
        let record = BlockRecord {
            network: "1.0.0.0/24".to_string(),
            geoname_id: String::new(),
            registered_country_geoname_id: "100".to_string(),
            represented_country_geoname_id: String::new(),
            is_anonymous_proxy: "0".to_string(),
            is_satellite_provider: "0".to_string(),
        };
        assert_eq!(resolve_country(&record, &map), "Australia");
    }

    #[test]
    fn resolves_country_defaults_to_unknown() {
        let map = HashMap::new();
        let record = BlockRecord {
            network: "1.0.0.0/24".to_string(),
            geoname_id: String::new(),
            registered_country_geoname_id: String::new(),
            represented_country_geoname_id: String::new(),
            is_anonymous_proxy: "0".to_string(),
            is_satellite_provider: "0".to_string(),
        };
        assert_eq!(resolve_country(&record, &map), UNKNOWN_COUNTRY_NAME);
    }
}
