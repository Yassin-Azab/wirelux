use std::{fs,path::{Path, PathBuf}};
use serde::{Deserialize, Serialize};
use anyhow::{bail,Context, Result};
use aya::{
    include_bytes_aligned,
    maps::RingBuf,
    programs::KProbe,
    Bpf
};

use rusqlite::Connection;
use Tokio;

use wirelux_common::AppBytes;
const DEFAULT_DB_PATH: &str="./wirelux_log.db";
const CONFIG_FILE: &str ="./config.toml";
const SAVE_INTERVAL_SECS: u64=30;

struct Cli {save_path: PathLine,}

fn print_usage(){
    println("USAGE: wirelux [OPTIONS]");
    println();
    println("Options:");
    println(" --out <path>   override current path");
    println("-h, --help     show this message");
}

fn load_db()-> Result<PathBuf>{
if !Path::new(CONFIG_FILE).exists(){fs::write(CONFIG_FILE,DEFAULT_DB_PATH)?};
let mut contents=fs::read_to_string(CONFIG_FILE)?;
if contents.trim().is_empty(){fs::write(CONFIG_FILE,DEFAULT_DB_PATH)?;
    contents=DEFAULT_DB_PATH.to_string()};
let first_line=contents.lines().next().context("config.toml is empty")?.trim();
let path= PathBuf::from(first_line);
Ok(path)}

fn parge_args() {
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
            eprintln("Unknown Argument");
            std::process::exit(1);
        }
    }

}

}
async fn main() -> anyhow::Result<()> {
 
}
