//! Runtime lookup CLI. This binary never touches CSV -- it only opens an
//! already-built `geolite.bin` and answers lookups against it.
//!
//! ```text
//! cargo run --release --bin geolite_lookup -- geolite.bin 8.8.8.8 1.1.1.1
//! ```

use std::env;
use std::process;

use geolite_lookup::GeoDatabase;

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args.first().map(String::as_str).unwrap_or("geolite_lookup");

    if args.len() < 3 {
        eprintln!("usage: {program} <geolite.bin> <ip> [ip ...]");
        eprintln!("example: {program} geolite.bin 8.8.8.8 1.1.1.1");
        process::exit(2);
    }

    let db_path = &args[1];
    let db = match GeoDatabase::open(db_path) {
        Ok(db) => db,
        Err(err) => {
            eprintln!("failed to open '{db_path}': {err}");
            process::exit(1);
        }
    };

    let mut had_error = false;
    for ip in &args[2..] {
        match db.lookup(ip) {
            Ok(country) => println!("{ip} -> {country}"),
            Err(err) => {
                eprintln!("{ip} -> error: {err}");
                had_error = true;
            }
        }
    }

    if had_error {
        process::exit(1);
    }
}
