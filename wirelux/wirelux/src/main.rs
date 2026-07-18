use std::{fs, path::{Path, PathBuf}, net::Ipv4Addr};
use anyhow::{Context, Result};
use aya::{
    maps::{RingBuf}, programs::FExit, Btf, Ebpf, include_bytes_aligned,
};
use rusqlite::{params, Connection};
use tokio::{io::unix::AsyncFd, signal, sync::Mutex, time::{interval, Duration}};
use std::sync::Arc;
use geolite_lookup::GeoDatabase;

use wirelux_common::AppBytes;

const DEFAULT_DB_PATH: &str = "./wirelux_log.db";
const CONFIG_FILE:     &str = "./config.toml";
const SAVE_INTERVAL_SECS: u64 = 30;

fn print_usage() {
    println!("USAGE: wirelux [OPTIONS]");
    println!();
    println!("Options:");
    println!(" --out <path>   override current path");
    println!("-h, --help     show this message");
}

fn load_db() -> Result<PathBuf> {
    if !Path::new(CONFIG_FILE).exists() { fs::write(CONFIG_FILE, DEFAULT_DB_PATH)?; }
    let contents = fs::read_to_string(CONFIG_FILE)?;
    let contents = if contents.trim().is_empty() {
        fs::write(CONFIG_FILE, DEFAULT_DB_PATH)?;
        DEFAULT_DB_PATH.to_string()
    } else {
        contents
    };
    let first_line = contents.lines().next().context("config.toml is empty")?.trim();
    Ok(PathBuf::from(first_line))
}

fn parse_args() {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => {
                let new_path = match args.next() {
                    Some(path) => PathBuf::from(path),
                    None => {
                        eprintln!("Error: --out requires a path");
                        std::process::exit(1);
                    }
                };
                if let Err(e) = fs::write(CONFIG_FILE, new_path.to_string_lossy().to_string()) {
                    eprintln!("error writing config file: {e}");
                    std::process::exit(1);
                }
            }
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            _ => {
                print_usage();
                eprintln!("Unknown Argument");
                std::process::exit(1);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    parse_args();
    let path = load_db().unwrap_or_else(|_| {
        eprintln!("failed to load db");
        std::process::exit(1);
    });

    let bpf_bytes = include_bytes_aligned!("../../target/bpfel-unknown-none/release/wirelux-ebpf");
    let mut bpf = Ebpf::load(bpf_bytes).context("Failed to load EBPF object.")?;
    let btf = Btf::from_sys_fs().context("Failed to load BTF from sysfs")?;

    attach_fexit(&mut bpf, &btf, "fexit_tcp_send_msg", "tcp_sendmsg")?;
    attach_fexit(&mut bpf, &btf, "fexit_tcp_recv_msg", "tcp_recvmsg")?;
    attach_fexit(&mut bpf, &btf, "fexit_udp_send_msg", "udp_sendmsg")?;
    attach_fexit(&mut bpf, &btf, "fexit_udp_recv_msg", "udp_recvmsg")?;

    let ring_map = bpf.take_map("EVENTS").context("EVENTS map not found")?;
    let ring_buf = RingBuf::try_from(ring_map).context("Failed to convert EVENTS to RingBuf")?;
    let async_ring = AsyncFd::new(ring_buf).context("Failed to create async ring buffer")?;

    let shared_events: Arc<Mutex<Vec<AppBytes>>> = Arc::new(Mutex::new(Vec::new()));
    let shared_events_reader = Arc::clone(&shared_events);
    let shared_events_saver  = Arc::clone(&shared_events);

    let reader_task = tokio::spawn({
        let shared_events_reader = shared_events_reader.clone();
        async move {
            let mut async_ring = async_ring;
            loop {
                let mut guard = match async_ring.readable_mut().await {
                    Ok(g)  => g,
                    Err(e) => { eprintln!("Error waiting for ring buffer: {e}"); break; }
                };
                let ring = guard.get_inner_mut();
                while let Some(entry) = ring.next() {
                    if entry.len() == std::mem::size_of::<AppBytes>() {
                        let event = unsafe { &*(entry.as_ptr() as *const AppBytes) };
                        shared_events_reader.lock().await.push(*event);
                    }
                }
                guard.clear_ready();
            }
        }
    });

    let path_saver = path.clone();
    let shared_events_saver_clone = Arc::clone(&shared_events_saver);

    let saver_task = tokio::task::spawn(async move {
        let mut ticker = interval(Duration::from_secs(SAVE_INTERVAL_SECS));
        loop {
            ticker.tick().await;

            let batch: Vec<AppBytes> = {
                let mut events = shared_events_saver_clone.lock().await;
                std::mem::take(&mut *events) // empties the Vec; lock released right after
            };

            if !batch.is_empty() {
                let path = path_saver.clone();
                match tokio::task::spawn_blocking(move || append_to_disk(&batch, path)).await {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => eprintln!("Error saving events to disk: {e}"),
                    Err(e)     => eprintln!("Saver task panicked: {e}"),
                }
            }
        }
    });

    signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
    println!("Ctrl+C received, saving events to disk...");
    reader_task.abort();
    saver_task.abort();

    let remaining: Vec<AppBytes> = {
        let mut events = shared_events.lock().await;
        std::mem::take(&mut *events)
    };
    if !remaining.is_empty() {
        tokio::task::spawn_blocking(move || append_to_disk(&remaining, path.clone())).await??;
        println!("Events saved to disk successfully.");
    }
    Ok(())
}

fn attach_fexit(bpf: &mut Ebpf, btf: &Btf, prog_name: &str, kernel_fn: &str) -> Result<()> {
    let program: &mut FExit = bpf
        .program_mut(prog_name)
        .with_context(|| format!("Bpf Program '{}' not found in ELF Obj", prog_name))?
        .try_into()
        .context("Wrong Program type")?;
    program
        .load(kernel_fn, btf)
        .with_context(|| format!("Failed to load program '{}'", prog_name))?;
    program
        .attach()
        .with_context(|| format!("Failed to attach program '{}' to kernel function '{}' — BPF verifier rejected it", prog_name, kernel_fn))?;
    Ok(())
}
fn find_process_name(pid: u32) -> Option<String>{let proc_path=format!("/proc/{}/comm",pid);
    if Path::new(&proc_path).exists()
    {    return fs::read_to_string(proc_path)
    .map(|name| name.trim().to_string()).ok();}
None}
fn geolite_lookup(db: &GeoDatabase, ip_addr: &str) -> Result<String> {
    Ok(db.lookup(ip_addr)?.to_string())
}
fn append_to_disk(events: &[AppBytes], db_path: PathBuf) -> Result<()> {
    let mut conn = Connection::open(db_path).context("Failed to open database")?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
        CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp_ns INTEGER NOT NULL,
            size INTEGER NOT NULL,
            remote_addr TEXT NOT NULL,
            pid INTEGER NOT NULL,
            local_port INTEGER NOT NULL,
            direction INTEGER NOT NULL,
            protocol INTEGER NOT NULL,
            comms TEXT NOT NULL,
            country TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_timestamp ON events (timestamp_ns);
        CREATE INDEX IF NOT EXISTS idx_ip ON events (remote_addr);
        CREATE INDEX IF NOT EXISTS idx_pid ON events (pid);",
    )
    .context("Failed to create events table")?;

    let tx = conn.transaction().context("Failed to start transaction")?;
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO events (timestamp_ns, size, remote_addr, pid, local_port, direction, protocol, comms, country) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            )
            .context("Failed to prepare insert statement")?;
        // Construct path to geolite.bin relative to cargo workspace
        let geolite_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("geolite_lookup/geolite.bin"))
            .unwrap_or_else(|| PathBuf::from("geolite.bin"));
        
        let db = match GeoDatabase::open(&geolite_path) {
    Ok(db) => Some(db),
    Err(e) => {
        eprintln!("Warning: could not open {}: {e}", geolite_path.display());
        None
    }
};
        for event in events {
            let comms_str = find_process_name(event.pid).unwrap_or_else(|| {
                match event.comm.iter().position(|&a| a == 0) {
                    Some(index) => std::str::from_utf8(&event.comm[..index])
                        .unwrap_or("")
                        .to_string(),
                    None => std::str::from_utf8(&event.comm)
                        .unwrap_or("")
                        .to_string(),
                }
            });
            let remote_addr_str = Ipv4Addr::from(u32::from_be(event.remote_addr)).to_string();
            let country = match db.as_ref() {
    None => "DB didn't open".to_string(),
    Some(db) => match geolite_lookup(db, &remote_addr_str) {
        Ok(country) => country,
        Err(e) => {
            eprintln!("Lookup failed: {e}");
            "Not found".to_string()
        }
    },
};

            stmt.execute(params![
                event.timestamp_ns as i64,
                event.size as i64,
                remote_addr_str,
                event.pid,
                event.local_port,
                event.direction,
                event.protocol,
                comms_str,
                country
            ])
            .context("Failed to execute insert statement")?;
        }
    }
    tx.commit().context("Failed to commit transaction")?;
    Ok(())
}