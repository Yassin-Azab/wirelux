<div align="center">

# wirelux

**A lightweight, per-process network traffic monitor for Linux, built on eBPF `fexit` probes.**

[![License: AGPL v3](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)
[![Made with Rust](https://img.shields.io/badge/made%20with-Rust-orange.svg)](https://www.rust-lang.org/)
[![Platform: Linux](https://img.shields.io/badge/platform-Linux-lightgrey.svg)](#kernel-compatibility)
[![Kernel: 5.11+](https://img.shields.io/badge/kernel-5.11%2B-informational.svg)](#kernel-compatibility)

</div>

---

`wirelux` instruments kernel network sending and receiving for IPv4 TCP/UDP traffic and saves the collected events to SQLite. It reports how much data each process **actually** transmits and receives — not what it merely attempted to send, and not dropped packets — using kernel function-exit (`fexit`) probes instead of kprobes for lower overhead and direct, typed access to kernel structures.

Inspired by [GlassWire](https://www.glasswire.com/) (Windows), built for Linux.

## Table of contents

- [What wirelux does](#what-wirelux-does)
- [Features](#features)
- [Architecture](#architecture)
- [Data schema](#data-schema)
- [Kernel compatibility](#kernel-compatibility)
- [Prerequisites](#prerequisites)
- [Build and run](#build-and-run)
- [Configuration](#configtoml)
- [GeoIP lookup](#geoip-lookup)
- [Tuning](#tuning)
- [Limitations](#limitations)
- [What this project is (and isn't) for](#what-this-project-is-and-isnt-for)
- [Roadmap](#roadmap)
- [Troubleshooting](#troubleshooting)
- [License](#license)

## What wirelux does

`wirelux` attaches to the kernel exit points of four functions, covering both directions of IPv4 TCP and UDP traffic:

| Function      | Traffic       |
| ------------- | ------------- |
| `tcp_sendmsg` | TCP, outbound |
| `tcp_recvmsg` | TCP, inbound  |
| `udp_sendmsg` | UDP, outbound |
| `udp_recvmsg` | UDP, inbound  |

For every call, it records the actual byte count, the remote IPv4 address, the local port, the owning PID, the protocol, and the process's command name. A background task batches these events and flushes them to a local SQLite database on a fixed interval.

## Features

- **Actual bytes, not attempted bytes.** Reads each function's return value, not the requested size — so a short write or a failed call doesn't get logged as a full-size success.
- **Per-process attribution.** Every event is tied to a PID and a process name.
- **Remote IP + local port capture.** Read directly off the kernel's socket structure at exit time.
- **SQLite persistence.** WAL-mode, indexed by timestamp, remote address, and PID.
- **Low overhead.** Built on `fexit`/BTF trampolines rather than kprobe/kretprobe — no register-snapshot parsing, no trap-based instrumentation.
- **Optional GeoIP country lookup** for remote addresses.
- **Configurable** ring buffer size and save interval.

## Architecture

```mermaid
flowchart LR
    A["tcp_sendmsg / tcp_recvmsg<br/>udp_sendmsg / udp_recvmsg"] -- fexit probe --> B["eBPF program<br/>(wirelux-ebpf)"]
    B -- AppBytes struct --> C[(Ring buffer<br/>EVENTS map)]
    C --> D["Async reader task"]
    D --> E["In-memory queue"]
    E -- every N seconds --> F["Saver task"]
    H --> G[(SQLite database)]
    F --> H["GeoIP lookup"]
```

The eBPF side (`wirelux-ebpf`) does the minimum work possible inside the kernel: read the socket and return-value fields, and push a fixed-size record onto a ring buffer. Everything else — batching, GeoIP resolution, and disk persistence — happens in userspace (`wirelux`), off the hot path.

## Data schema

Events are appended to a table named `events`:

| Column         | Type      | Description                                            |
| -------------- | --------- | ------------------------------------------------------ |
| `id`           | `INTEGER` | Autoincrementing primary key                           |
| `timestamp_ns` | `INTEGER` | Kernel timestamp of the event, in nanoseconds          |
| `size`         | `INTEGER` | Actual bytes sent or received                          |
| `remote_addr`  | `TEXT`    | Remote IPv4 address, dotted-quad                       |
| `pid`          | `INTEGER` | Process ID                                             |
| `local_port`   | `INTEGER` | Local port                                             |
| `direction`    | `INTEGER` | `0` = received, `1` = sent                             |
| `protocol`     | `INTEGER` | `6` = TCP, `17` = UDP (IANA protocol numbers)          |
| `comms`        | `TEXT`    | Process command name                                   |
| `country`      | `TEXT`    | Remote address's country, if GeoIP lookup is available |

The database file is created automatically if it doesn't already exist.

## Kernel compatibility

`wirelux` is built around BTF-based `fexit` attachment, which has real, non-negotiable kernel requirements — this isn't a "should work everywhere" tool.

| Requirement         | Detail                                                           |
| ------------------- | ---------------------------------------------------------------- |
| Kernel              | 5.11+ on x86_64, 6.0+ on ARM                                     |
| BTF                 | `/sys/kernel/btf/vmlinux` must exist (`CONFIG_DEBUG_INFO_BTF=y`) |
| Tested architecture | x86_64 only — ARM is unverified                                  |
| OS                  | Linux only                                                       |

If `/sys/kernel/btf/vmlinux` doesn't exist on your system, `wirelux` cannot run — there is no fallback path.

## Prerequisites

1. **Install Rust.**
   - Recommended: `rustup toolchain install stable`
   - If you hit BPF build issues: `rustup toolchain install nightly`
2. **Install `bpf-linker`.**
   - `cargo install bpf-linker`
   - On macOS, install LLVM first, then: `cargo install bpf-linker --no-default-features`
3. **Cross-compiling from macOS or another host?**
   - `rustup target add ${ARCH}-unknown-linux-musl`
   - For Apple Silicon/Intel macOS, use `rust-lld` or another cross linker.

## Build and run

From `wirelux/wirelux`:

```bash
cargo build --package wirelux --release
cargo run --package wirelux --release
```

This build path automatically compiles the eBPF object and embeds it into the binary.

### Run with a custom database path

```bash
cargo run --package wirelux --release -- --out /tmp/wirelux_log.db
```

The custom path is persisted to `config.toml`.

### Optional build helper

An `xtask` helper builds only the eBPF crate:

```bash
cargo run -p xtask -- build-ebpf --release
```

Force a nightly build for the eBPF target if needed:

```bash
cargo +nightly run -p xtask -- build-ebpf --release
```

## `config.toml`

`wirelux` reads a simple one-line configuration file at `./config.toml`. The first non-empty line is the SQLite database path.

- Missing `config.toml` → created automatically.
- Empty `config.toml` → defaults to `./wirelux_log.db`.
- `config.toml` with a path → that path is used.

```text
./wirelux_log.db
```

## GeoIP lookup

`wirelux` looks for a GeoIP database at `../geolite_lookup/geolite.bin`, relative to the `wirelux` crate directory.

- **Present and loads successfully** → remote IPs are resolved to country names in the `country` column.
- **Missing** → events still save normally; `country` contains a fallback message instead.

## Tuning

### Ring buffer size

`wirelux/wirelux-ebpf/src/main.rs`:

```rust
const RING_BUF_SIZE: u32 = 1024 * 1024 * 4; // 4 MiB
```

### Save interval

`wirelux/wirelux/src/main.rs`:

```rust
const SAVE_INTERVAL_SECS: u64 = 30;
```

Rebuild after changing either.

## Limitations

- **Linux-only.** Depends on eBPF, BTF, and kernel `fexit` support.
- **x86_64 only, verified.** ARM is not tested by this repository.
- **IPv4 only.** The event data format uses `u32` addresses.
- **Assumes a specific `sock_common` layout** and kernel BTF metadata; unsupported or unusual kernels may fail verifier checks or yield invalid data.
- **No per-interface uniqueness** — traffic isn't distinguished by network interface.
- **Dropped packets are invisible by design**, to avoid destabilizing the eBPF program.
- **UDP truncation:** if a UDP receive buffer is smaller than the incoming datagram, the recorded size is the datagram's full length, not the bytes actually copied into the buffer.
- **Process name accuracy is approximate.** It reads `/proc/<pid>/comm` and falls back to the kernel-captured `comm` value; both can be stale or wrong if a PID is reused quickly.
- **GeoIP is optional.** Without `geolite.bin`, events still save, but `country` holds a fallback message instead of a real lookup.

## What this project is (and isn't) for

| Helps with                                                       | Doesn't help with                                                   |
| ---------------------------------------------------------------- | ------------------------------------------------------------------- |
| Spotting a process sending an unreasonable amount of data        | Malware injected into a legitimate, low-traffic process             |
| Noticing traffic to destinations that shouldn't be receiving any | Anything that doesn't show up as elevated or unusual network volume |

This is a visibility tool, not an intrusion-detection or anti-malware system.

## Roadmap

- [x] Country lookup for remote IPs
- [ ] YARA rule integration for malware scanning
- [ ] Total bytes sent, reported per network interface
- [ ] IPv6 support
- [ ] Include local IP addresses in events

## Troubleshooting

| Error                           | Likely cause                                                                                                                 | Fix                                                                                                                                                                                            |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Failed to load EBPF object.`   | `target/bpfel-unknown-none/release/wirelux-ebpf` doesn't exist, or `bpf-linker` isn't installed                              | Build the eBPF crate first (`cargo run -p xtask -- build-ebpf --release`); confirm `bpf-linker` is on your `PATH`                                                                              |
| `Failed to load BTF from sysfs` | `/sys/kernel/btf/vmlinux` is missing                                                                                         | Install a kernel debug/BTF package, or boot a kernel that exposes BTF                                                                                                                          |
| `BPF verifier rejected it`      | Kernel doesn't support the required `fexit` attachments, or its socket struct layout doesn't match what this project assumes | Check kernel version against [Kernel compatibility](#kernel-compatibility); see the project's fentry/fexit migration notes for how to verify function signatures against your own kernel's BTF |

## License

Distributed under the [GNU Affero General Public License, version 3](LICENSE).

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project is licensed under the same terms.
