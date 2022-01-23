use crate::TimeVal;

const SYSCALL_DUP: usize = 24;
const SYSCALL_OPEN: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYSCALL_PIPE: usize = 59;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_FORK: usize = 220;
const SYSCALL_EXEC: usize = 221;
const SYSCALL_WAITPID: usize = 260;
const SYSCALL_SPAWN: usize = 400;
const SYSCALL_INIT_USER_TRAP: usize = 600;
const SYSCALL_SEND_MSG: usize = 601;
const SYSCALL_SET_TIMER: usize = 602;
const SYSCALL_CLAIM_EXT_INT: usize = 603;
const SYSCALL_SET_EXT_INT_ENABLE: usize = 604;
const SYSCALL_UIPI_SENDER_CTL: usize = 700;
const SYSCALL_UIPI_RECEIVER_CTL: usize = 701;
const SYSCALL_UIPI_CONNECTION_CTL: usize = 702;

fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        llvm_asm!("ecall"
            : "={x10}" (ret)
            : "{x10}" (args[0]), "{x11}" (args[1]), "{x12}" (args[2]), "{x17}" (id)
            : "memory"
            : "volatile"
        );
    }
    ret
}

pub fn sys_dup(fd: usize) -> isize {
    syscall(SYSCALL_DUP, [fd, 0, 0])
}

pub fn sys_open(path: &str, flags: u32) -> isize {
    syscall(SYSCALL_OPEN, [path.as_ptr() as usize, flags as usize, 0])
}

pub fn sys_close(fd: usize) -> isize {
    syscall(SYSCALL_CLOSE, [fd, 0, 0])
}

pub fn sys_pipe(pipe: &mut [usize]) -> isize {
    syscall(SYSCALL_PIPE, [pipe.as_mut_ptr() as usize, 0, 0])
}

pub fn sys_read(fd: usize, buffer: &mut [u8]) -> isize {
    syscall(
        SYSCALL_READ,
        [fd, buffer.as_mut_ptr() as usize, buffer.len()],
    )
}

pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

pub fn sys_exit(exit_code: i32) -> ! {
    syscall(SYSCALL_EXIT, [exit_code as usize, 0, 0]);
    panic!("sys_exit never returns!");
}

pub fn sys_yield() -> isize {
    syscall(SYSCALL_YIELD, [0, 0, 0])
}

#[allow(unused_variables)]
pub fn sys_get_time(time: &TimeVal, tz: usize) -> isize {
    syscall(SYSCALL_GET_TIME, [time as *const _ as usize, tz, 0])
}

pub fn sys_getpid() -> isize {
    syscall(SYSCALL_GETPID, [0, 0, 0])
}

pub fn sys_fork() -> isize {
    syscall(SYSCALL_FORK, [0, 0, 0])
}

pub fn sys_exec(path: &str, args: &[*const u8]) -> isize {
    syscall(
        SYSCALL_EXEC,
        [path.as_ptr() as usize, args.as_ptr() as usize, 0],
    )
}

pub fn sys_spawn(path: &str) -> isize {
    syscall(SYSCALL_SPAWN, [path.as_ptr() as usize, 0, 0])
}

pub fn sys_waitpid(pid: isize, exit_code: *mut i32) -> isize {
    syscall(SYSCALL_WAITPID, [pid as usize, exit_code as usize, 0])
}

pub fn sys_init_user_trap() -> isize {
    syscall(SYSCALL_INIT_USER_TRAP, [0, 0, 0])
}

pub fn sys_send_msg(pid: usize, msg: usize) -> isize {
    syscall(SYSCALL_SEND_MSG, [pid as usize, msg as usize, 0])
}

pub fn sys_set_timer(time_us: isize) -> isize {
    syscall(SYSCALL_SET_TIMER, [time_us as usize, 0, 0])
}

pub fn sys_claim_ext_int(device_id: usize) -> isize {
    syscall(SYSCALL_CLAIM_EXT_INT, [device_id as usize, 0, 0])
}

pub fn sys_set_ext_int_enable(device_id: usize, enable: usize) -> isize {
    syscall(SYSCALL_SET_EXT_INT_ENABLE, [device_id as usize, enable, 0])
}

pub fn sys_uipi_sender_ctl(flags: usize, sender_id: u32, buf: *mut u8) -> isize {
    syscall(
        SYSCALL_UIPI_SENDER_CTL,
        [flags, sender_id as usize, buf as usize],
    )
}

pub fn sys_uipi_receiver_ctl(flags: usize, receiver_id: u32, buf: *mut u8) -> isize {
    syscall(
        SYSCALL_UIPI_RECEIVER_CTL,
        [flags, receiver_id as usize, buf as usize],
    )
}

pub fn sys_uipi_connection_ctl(sender_id: u32, receiver_id: u32, connected: bool) -> isize {
    syscall(
        SYSCALL_UIPI_CONNECTION_CTL,
        [sender_id as usize, receiver_id as usize, connected as usize],
    )
}
