#![no_std]

#[repr(C)]
#[derive(Clone,Copy,Debug, Default)]
pub struct AppBytes{
    pub timestamp_ns: u64,
    pub size: u64,
    pub remote_addr: u32,
    pub pid: u32,
    pub local_port: u16,
    pub direction: u8,
    pub protocol: u8,
    pub comm: [u8;16],
    pub _pad: [u8; 4],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for AppBytes{}