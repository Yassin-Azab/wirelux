#![no_std]
#![no_main]

use aya_ebpf::{
    macros::{kprobe,kretprobe, map},
    programs::{ProbeContext, RetProbeContext}
    maps::RingBuf,
    helpers::{bpf_get_current_comm,bpf_get_current_pid_tgid,bpf_ktime_get_real_ns}
};
use aya_log_ebpf::info;
use wirelux_common::AppBytes;

const Ring_buf_Size: u32= 1024*1024*4
    
    #[map]
    static mut Events: RingBuf=RingBuf::with_size_bytes(Ring_Buf_Size,0);


unsafe fn write_event(pid::u32, comm:[u8;16], bytes::u64, direction:u8, protocol:: u8) -> Result<(),i64>{

}
                      
#[kprobe]
pub fn wirelux(ctx: ProbeContext) -> u32 {
    match try_wirelux(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_wirelux(ctx: ProbeContext) -> Result<u32, u32> {
    info!(&ctx, "kprobe called");
    Ok(0)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(link_section = "license")]
#[unsafe(no_mangle)]
static LICENSE: [u8; 13] = *b"Dual MIT/GPL\0";
