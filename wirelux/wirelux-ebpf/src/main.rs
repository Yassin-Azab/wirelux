#![no_std]
#![no_main]

use aya_ebpf::{
    cty::c_int, helpers::{bpf_get_current_comm,bpf_get_current_pid_tgid,bpf_ktime_get_ns,bpf_printk}, macros::{fexit, map}, maps::RingBuf, programs::FExitContext,
};
use wirelux_common::AppBytes;

#[repr(C)]
struct SockCommon{
    skc_daddr:u32,
    skc_rcv_saddr: u32,
    skc_hash: u32,
    skc_dport: u16,
    skc_num: u16
}

#[inline(always)]
unsafe fn read_sock(sk: *const SockCommon)-> (u32,u16){
    let remote_addr=(*sk).skc_daddr;
    let local_port: u16= (*sk).skc_num;
    return (remote_addr,local_port)
}

const RING_BUF_SIZE: u32= 1024*1024*4;
    
    #[map]
    static EVENTS: RingBuf=RingBuf::with_byte_size(RING_BUF_SIZE, 0);


unsafe fn write_event(bytes:u64,addr:u32,pid:u32,port:u16,direction:u8,protocol:u8,comm: [u8;16]) -> Result<(),i64>{
let timestamp_now=unsafe { bpf_ktime_get_ns() };
let mut entry= match EVENTS.reserve::<AppBytes>(0){
Some(e)=>e,
None=>return Err(-1)
};
let ptr=entry.as_mut_ptr();
unsafe{
    (*ptr).timestamp_ns=timestamp_now;
    (*ptr).size=bytes;
    (*ptr).remote_addr=addr;
    (*ptr).pid=pid;
    (*ptr).local_port=port;
    (*ptr).protocol=protocol;
    (*ptr).direction=direction;
    (*ptr).comm=comm;
    (*ptr)._pad = [0 as u8;4];
};
entry.submit(0);
Ok(())
}

#[fexit(function="tcp_sendmsg")]
pub fn fexit_tcp_send_msg(ctx: FExitContext) -> u32{
    match unsafe {try_tcp_sendmsg(&ctx)}{
        Ok(_) | Err(_) => 0,

    }
}

unsafe fn try_tcp_sendmsg(ctx: &FExitContext) -> Result<(), i64>{
let actual: i32        = ctx.arg(3);
if actual<=0 {return Err(-1i64)};

let sock: *const SockCommon=ctx.arg(0);
let (remoteaddr,localport)=read_sock(sock);

let total_id:u64=bpf_get_current_pid_tgid();
let pid: u32 = (total_id >> 32) as u32;
let comm:[u8;16]=bpf_get_current_comm().map_err(|e|e as i64)?;
write_event(actual as u64, remoteaddr, pid, localport, 1, 6, comm)
}


#[fexit(function="udp_sendmsg")]
pub fn fexit_udp_send_msg(ctx: FExitContext) -> u32{
    match unsafe {try_udp_sendmsg(&ctx)}{
        Ok(_) | Err(_) => 0,
    }
}

unsafe fn try_udp_sendmsg(ctx: &FExitContext) -> Result<(), i64>{
let size:c_int=ctx.arg(3);

if size<=0 {return Err(-1i64)};

let sock: *const SockCommon=ctx.arg(0);
let (remoteaddr,localport)=read_sock(sock);

let total_id:u64=bpf_get_current_pid_tgid();
let pid: u32 = (total_id >> 32) as u32;
let comm:[u8;16]=bpf_get_current_comm().map_err(|e|e as i64)?;
write_event(size as u64, remoteaddr, pid, localport, 1, 17, comm)
}

#[fexit(function="tcp_recvmsg")]
pub fn fexit_tcp_recv_msg(ctx: FExitContext) -> u32{
    match unsafe{ try_tcp_recvmsg(&ctx)}{
    Ok(_)| Err(_)=> 0,
    }
}

unsafe fn try_tcp_recvmsg(ctx: &FExitContext) ->  Result<(), i64>{
let size:i32 =ctx.arg(5);
if size<=0 {return Err(-1i64)};

let sock: *const SockCommon=ctx.arg(0);
let (remoteaddr,localport)=read_sock(sock);

let total_id:u64=bpf_get_current_pid_tgid();
let pid: u32 = (total_id >> 32) as u32;
let comm:[u8;16]=bpf_get_current_comm().map_err(|e|e as i64)?;
write_event(size as u64, remoteaddr, pid, localport, 0, 6, comm)

}


#[fexit(function="udp_recvmsg")]
pub fn fexit_udp_recv_msg(ctx: FExitContext) -> u32{
    match unsafe{try_udp_recvmsg(&ctx)}{
        Ok(_)| Err(_)=> 0,
    }
}

unsafe fn try_udp_recvmsg(ctx: &FExitContext) -> Result<(), i64>{
let size:i32 =ctx.arg(5);
if size<=0 {return Err(-1i64)};

let sock: *const SockCommon=ctx.arg(0);
let (remoteaddr,localport)=read_sock(sock);

let total_id:u64=bpf_get_current_pid_tgid();
let pid: u32 = (total_id >> 32) as u32;
let comm:[u8;16]=bpf_get_current_comm().map_err(|e: i32|e as i64)?;
write_event(size as u64, remoteaddr, pid, localport, 0, 17, comm)  
}



#[unsafe(link_section = "license")]
#[unsafe(no_mangle)]
static LICENSE: [u8; 13] = *b"Dual MIT/GPL\0";

#[cfg(target_arch = "bpf")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}