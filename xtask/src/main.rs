use std::{path::PathBuf, process::{Command, ExitStatus}, result};
use anyhow::{bail, Context, Result};

fn main() -> Result<()>{
let mut args =std::env::args().skip(1);
match args.next().as_deref(){
    Some("build-ebpf")=>{
    let release =args.any(|a| a=="--release");
    build_ebpf(release);
    Ok(())
}
unk =>{
    bail!("Unknown command: {:?}. Usage: cargo xtask build-ebpf [--release]", unk)
}
}

}

fn find_root()-> PathBuf{
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn build_ebpf(release: bool){
let root=find_root();
let mut cmd =Command::new("cargo");
cmd.current_dir(root.clone()).args([
    //decide on args
]);
if release {cmd.arg("--release");}
let status =match cmd.status().context("failed to run cargo"){
Ok((status)) => println!("works"),
Err(e) => eprintln!("failed to execute")};
}