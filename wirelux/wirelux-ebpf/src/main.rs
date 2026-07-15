#![no_std]
#![no_main]

use aya_ebpf::{
    macros::{kprobe,kretprobe, map},
    programs::{ProbeContext, RetProbeContext},
    maps::RingBuf,
    helpers::{bpf_get_current_comm,bpf_get_current_pid_tgid,bpf_ktime_get_ns}
};
use wirelux_common::AppBytes;

const RING_BUF_SIZE: u32= 1024*1024*4;
    
    #[map]
    static EVENTS: RingBuf=RingBuf::with_byte_size(RING_BUF_SIZE, 0);


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
pub fn kprobe_tcp_send_msg(ctx: ProbeContext) -> u32{
    match unsafe {try_tcp_sendmsg(&ctx)}{
        Ok(_) | Err(_) => 0,

    }
}

unsafe fn try_tcp_sendmsg(ctx: &ProbeContext) -> Result<(), i64>{
let arg: usize=ctx.arg(2).ok_or(-1i64)?;
if arg ==0 { return Err((-1i64)); }

let total_id:u64=bpf_get_current_pid_tgid();
let pid: u32 = (total_id >> 32) as u32;

let comm:[u8;16]=bpf_get_current_comm().map_err(|e|e as i64)?;
write_event(pid, comm, arg as u64, 1, 6)

}


#[kprobe]
pub fn kprobe_udp_send_msg(ctx: ProbeContext) -> u32{
    match unsafe {try_udp_sendmsg(&ctx)}{
        Ok(_) | Err(_) => 0,
    }
}

unsafe fn try_udp_sendmsg(ctx: &ProbeContext) -> Result<(), i64>{
let arg: usize=ctx.arg(2).ok_or(-1i64)?;
if arg ==0 { return Err((-1i64)); }

let total_id:u64=bpf_get_current_pid_tgid();
let pid: u32 = (total_id >> 32) as u32;

let comm:[u8;16]=bpf_get_current_comm().map_err(|e|e as i64)?;
write_event(pid, comm, arg as u64, 1, 17)

}

#[kretprobe]
pub fn kretprobe_tcp_recv_msg(ctx: RetProbeContext) -> u32{
    match unsafe{ try_tcp_recvmsg(&ctx)}{
    Ok(_)| Err(_)=> 0,
    }
}

unsafe fn try_tcp_recvmsg(ctx: &RetProbeContext) ->  Result<(), i64>{
let arg:i64 =ctx.ret() as i64;
if arg < 0 {return Err((-1i64))}
let total_id:u64=bpf_get_current_pid_tgid();
let pid: u32 = (total_id >> 32) as u32;

let comm:[u8;16]=bpf_get_current_comm().map_err(|e|e as i64)?;
write_event(pid, comm, arg as u64, 0, 17)

}


#[kretprobe]
pub fn kretprobe_udp_recv_msg(ctx: RetProbeContext) -> u32{
    match unsafe{try_udp_recvmsg(&ctx)}{
        Ok(_)| Err(_)=> 0,
    }
}

unsafe fn try_udp_recvmsg(ctx: &RetProbeContext) -> Result<(), i64>{
let arg:i64 =ctx.ret() as i64;
if arg < 0 {return Err((-1i64))}
let total_id:u64=bpf_get_current_pid_tgid();
let pid: u32 = (total_id >> 32) as u32;

let comm:[u8;16]=bpf_get_current_comm().map_err(|e|e as i64)?;
write_event(pid, comm, arg as u64, 0, 7)   
}



#[unsafe(link_section = "license")]
#[unsafe(no_mangle)]
static LICENSE: [u8; 13] = *b"Dual MIT/GPL\0";


#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}