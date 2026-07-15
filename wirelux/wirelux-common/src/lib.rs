#![no_std]

#[repr(C)]
#[derive(Clone,Copy,Debug, Default)]
pub struct AppBytes{
    pub timestamp_ns: u64,
    pub comms: [u8;16],
    pub size: u64,
    pub protocol: u8,
    pub direction: u8,
    pub pid: u32,
    pub _pad: [u8;2]
}
