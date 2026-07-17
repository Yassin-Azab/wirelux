fn main() {
    println!("cargo:rerun-if-changed=../wifi-monitor-ebpf/src/main.rs");
    println!("cargo:rerun-if-changed=../wifi-monitor-ebpf/Cargo.toml");
}