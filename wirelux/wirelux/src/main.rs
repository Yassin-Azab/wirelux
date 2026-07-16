use std::{fs, path::{Path, PathBuf}, time::Duration};
use libc::user;
use serde::{Deserialize, Serialize};
use anyhow::{bail,Context, Result};
use aya::{
    Bpf, include_bytes_aligned, maps::{RingBuf, ring_buf}, programs::fexit
};

use rusqlite::Connection;
use tokio::{self, io::unix::AsyncFd, signal};

use wirelux_common::AppBytes;
const DEFAULT_DB_PATH: &str="./wirelux_log.db";
const CONFIG_FILE: &str ="./config.toml";
const SAVE_INTERVAL_SECS: u64=30;

fn print_usage(){
    println!("USAGE: wirelux [OPTIONS]");
    println!();
    println!("Options:");
    println!(" --out <path>   override current path");
    println!("-h, --help     show this message");
}

fn load_db()-> Result<PathBuf>{
    if !Path::new(CONFIG_FILE).exists(){fs::write(CONFIG_FILE,DEFAULT_DB_PATH)?};
    let contents = fs::read_to_string(CONFIG_FILE)?;
    let contents = if contents.trim().is_empty() {
        fs::write(CONFIG_FILE, DEFAULT_DB_PATH)?;
        DEFAULT_DB_PATH.to_string()
    } else {
        contents
    };
    let first_line = contents.lines().next().context("config.toml is empty")?.trim();
    let path = PathBuf::from(first_line);
    Ok(path)
}

fn parse_args() {
let mut args =std::env::args().skip(1);
while let Some(arg)= args.next(){
    match arg.as_str(){
        "--out"=> {
            let new_path=match args.next(){
                Some(path) =>PathBuf::from(path),
                None => {
                    eprintln!("Error: --out requires a path");
                    std::process::exit(1);
                }
            }; 
            if let Err(e)=fs::write(CONFIG_FILE, new_path.clone().to_string_lossy().to_string()){
                eprintln!("error writing config file: {e})");
                std::process::exit(1);

            }
        },
        "-h" | "--help" =>{
            print_usage();
            std::process::exit(0);
        }
        other =>{
            print_usage();
            eprintln!("Unknown Argument");
            std::process::exit(1);
        }
    }

}

}
#[tokio::main]
async fn main() {
env::logger::init();
parse_args();
let path= load_db().unwrap_or_else({
    ||
    eprintln!("failed to load db");
    std::process::exit(1);
});

#[cfg(debug_assertions)] {
    println!("Debug mode enabled");
    let bpf_bytes=include_bytes_aligned!("../../wirelux-ebpf/target/bpfel-unknown-none/debug/wirelux-ebpf");
}
#[cfg(not(debug_assertions))] {
    println!("Debug mode disabled");
    let bpf_bytes=include_bytes_aligned!("../../wirelux-ebpf/target/bpfel-unknown-none/release/wirelux-ebpf");
}
let mut bpf= Bpf::load(bpf_bytes).context("Failed to load EBPF object.");

//Attach Fexits here


fn attach_fexists(bpf: &mut Bpf, prog_name: &str, kernel_fn: &str) -> Result<()>{
let program: &mut fexit= bpf.program_mut(prog_name).with_context(|| println!("Bpf Program '{}' not found in ELF Obj", prog_name))?;
program.load().with_context(|| println!("Failed to load program '{}'", prog_name))?;
program.attach(kernel_fn, 0).with_context(|| println!("Failed to attach program '{}' to kernel function '{}'", prog_name, kernel_fn))?;

Ok(())
}

let shared_events: Arc<Mutex<Vec<AppBytes>>> = Arc::new(Mutex::new(Vec::new()));
let shared_events_reader= Arc::clone(&shared_events);
let shared_events_saver = Arc::clone(&shared_events);


let userspace_buff=RingBuf::try_from(bpf.map_mut("EVENTS")?).context("Failed to get events ring buffer")?;
let mut async_ring=AsyncFd::new(userspace_buff).context("Failed to create async ring buffer")?;


let reader_task=tokio::spawn(async move{
    loop{
        let mut guard=match async_ring.readable_mut().await{
            Ok(g)=>g,
            Err(e)=>
            {
                eprintln!("Error waiting for ring buffer: {e}");
                std::process::exit(3);
            }
        };
    }
    let ring =guard.get_inner_mut();
    while let Some(entry)=ring.next(){
        if item.len()==std::mem::size_of::<AppBytes>{
            let event = unsafe {  &*(item.as_ptr() as *const AppBytes)};
            let event_copy= *event;
            let mut events = shared_events_reader.lock().await();
            events.push(event_copy);
        }
    }
guard.clear_ready();

});
let shared_events_saver_clone= Arc::clone(&shared_events_saver);
let mut last_saved_index:usize =0;


let saver_task= tokio::task::spawn(async move{
let mut ticker=interval(Duration::from_secs(SAVE_INTERVAL_SECS));
ticker.tick().await;
loop{
let events=shared.events_saver_clone.lock().await;
let new_event: &[AppBytes]= &events[last_saved_index..];
if !new_events.is_empty(){
    match append_to_disk(new_events, path.clone()){
        Ok(()) => {last_saved_index+= new_events.len();},
        Err(e) => eprintln!("Error saving events to disk: {e}"),
    }
}
}});


fn append_to_disk(events: &[AppBytes], pathScope: PathBuf)-> Result<()>{
let mut conn=connection::open(pathScope).context("Failed to open database")?;
conn.execute_batch(
    "PRAGMA journal_mode=WAL;
    CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp_ns INTEGER NOT NULL,
    pid INTEGER NOT NULL,
    comms TEXT NOT NULL,
    size INTEGER NOT NULL,
    direction INTEGER NOT NULL,
    protocol INTEGER NOT NULL)
    "//Create Indexing after deciding on final AppByte
).context("Failed to create events table");

let tx=conn.transaction().context("Failed to start transaction")?;
{
    let mut stmt=tx.prepare(
        "INSERT INTO events (timestamp_ns, pid, comms, size, direction, protocol) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
    ).context("Failed to prepare insert statement")?;
    for event in events{
        let comms_str=String::from_utf8_lossy(&event.comms).trim_end_matches('\0').to_string();
        stmt.execute(params![
            event.timestamp_ns,
            event.pid,
            comms_str,
            event.size,
            event.direction,
            event.protocol
        ]).context("Failed to execute insert statement")?;
    }
}
let tx =conn.transaction().context("Failed to start transaction")?; 
{
    //COmplete after deciding on final AppBytes
}

Ok(())
};


signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
println!("Ctrl+C received, saving events to disk...");
reader_task.abort();
saver_task.abort();
let events=shared_events.lock().await;
let remaining: &[AppBytes]= &events[last_saved_index..];
if !remaining.is_empty(){
    match append_to_disk(remaining, path.clone()){
        Ok(()) => println!("Events saved to disk successfully."),
        Err(e) => eprintln!("Error saving events to disk: {e}"),
    }
}
}
