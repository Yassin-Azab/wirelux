#![no_std]
#![no_main]

use core::f32::consts::E;

use aya_ebpf::{
    macros::{kprobe,kretprobe, map},
    programs::{ProbeContext, RetProbeContext},
    maps::RingBuf,
    helpers::{bpf_get_current_comm,bpf_get_current_pid_tgid,bpf_ktime_get_ns}
};
use aya_log_ebpf::info;
use wirelux_common::AppBytes;

const Ring_buf_Size: u32= 1024*1024*4;
    
    #[map]
    static EVENTS: RingBuf=RingBuf::with_byte_size(Ring_buf_Size, 0);


unsafe fn write_event(pid:u32, comm:[u8;16], bytes:u64, direction:u8, protocol: u8) -> Result<(),i64>{
let timestamp_now=unsafe { bpf_ktime_get_ns() };
let mut entry= match EVENTS.reserve::<AppBytes>(0){
Some(e)=>e,
None=>return Err((-1))
};
let ptr=entry.as_mut_ptr();
unsafe{
    (*ptr).timestamp_ns=timestamp_now;
    (*ptr).comms=comm;
    (*ptr).size=bytes;
    (*ptr).protocol=protocol;
    (*ptr).direction=direction;
    (*ptr).pid=pid;
    (*ptr)._pad = [0;2];
};
entry.submit(0);
Ok(())
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

#[kprobe]
pub fn kprobe_tcp_send_msg(ctx: ProbeContext) -> u32{
    match try_tcp_sendmsg(&ctx){
    }
}

unsafe fn try_tcp_sendmsg(ctx: &ProbeContext) -> Result<(), i64>{

}


#[kprobe]
pub fn kprobe_udp_send_msg(ctx: ProbeContext) -> u32{
    match try_udp_sendmsg(&ctx){
    }
}

unsafe fn try_udp_sendmsg(ctx: &ProbeContext) -> Result<(), i64>{

}

#[kretprobe]
pub fn kretprobe_tcp_recv_msg(ctx: ProbeContext) -> u32{
    match try_tcp_recvmsg(&ctx){

    }
}

unsafe fn try_tcp_recvmsg(ctx: &ProbeContext) -> Result<(), i64>{

}


#[kretprobe]
pub fn kretprobe_tcp_recv_msg(ctx: ProbeContext) -> u32{
    match try_tcp_recvmsg(&ctx){

    }
}

unsafe fn try_tcp_recvmsg(ctx: &ProbeContext) -> Result<(), i64>{
    
}




#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(link_section = "license")]
#[unsafe(no_mangle)]
static LICENSE: [u8; 13] = *b"Dual MIT/GPL\0";
