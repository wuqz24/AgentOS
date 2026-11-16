const SYSCALL_DUP: usize = 24;
const SYSCALL_CONNECT: usize = 29;
const SYSCALL_LISTEN: usize = 30;
const SYSCALL_ACCEPT: usize = 31;
const SYSCALL_OPEN: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYSCALL_PIPE: usize = 59;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_SLEEP: usize = 101;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_KILL: usize = 129;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_FORK: usize = 220;
const SYSCALL_EXEC: usize = 221;
const SYSCALL_WAITPID: usize = 260;
const SYSCALL_THREAD_CREATE: usize = 1000;
const SYSCALL_GETTID: usize = 1001;
const SYSCALL_WAITTID: usize = 1002;
const SYSCALL_MUTEX_CREATE: usize = 1010;
const SYSCALL_MUTEX_LOCK: usize = 1011;
const SYSCALL_MUTEX_UNLOCK: usize = 1012;
const SYSCALL_SEMAPHORE_CREATE: usize = 1020;
const SYSCALL_SEMAPHORE_UP: usize = 1021;
const SYSCALL_SEMAPHORE_DOWN: usize = 1022;
const SYSCALL_CONDVAR_CREATE: usize = 1030;
const SYSCALL_CONDVAR_SIGNAL: usize = 1031;
const SYSCALL_CONDVAR_WAIT: usize = 1032;
const SYSCALL_FRAMEBUFFER: usize = 2000;
const SYSCALL_FRAMEBUFFER_FLUSH: usize = 2001;
const SYSCALL_EVENT_GET: usize = 3000;
const SYSCALL_KEY_PRESSED: usize = 3001;
const SYSCALL_AGENT_CREATE: usize = 4000;
const SYSCALL_AGENT_INFO: usize = 4001;
const SYSCALL_TOOL_CALL: usize = 4002;
const SYSCALL_TOOL_LIST: usize = 4003;
const SYSCALL_CONTEXT_PUSH: usize = 4004;
const SYSCALL_CONTEXT_QUERY: usize = 4005;
const SYSCALL_CONTEXT_ROLLBACK: usize = 4006;
const SYSCALL_CONTEXT_CLEAR: usize = 4007;
const SYSCALL_AGENT_HEARTBEAT_SET: usize = 4008;

mod fs;
mod gui;
mod input;
mod net;
mod process;
mod sync;
mod thread;
use crate::agent;

use fs::*;
use gui::*;
use input::*;
use net::*;
use process::*;
use sync::*;
use thread::*;

pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    match syscall_id {
        SYSCALL_DUP => sys_dup(args[0]),
        SYSCALL_CONNECT => sys_connect(args[0] as _, args[1] as _, args[2] as _),
        SYSCALL_LISTEN => sys_listen(args[0] as _),
        SYSCALL_ACCEPT => sys_accept(args[0] as _),
        SYSCALL_OPEN => sys_open(args[0] as *const u8, args[1] as u32),
        SYSCALL_CLOSE => sys_close(args[0]),
        SYSCALL_PIPE => sys_pipe(args[0] as *mut usize),
        SYSCALL_READ => sys_read(args[0], args[1] as *const u8, args[2]),
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_SLEEP => sys_sleep(args[0]),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_KILL => sys_kill(args[0], args[1] as u32),
        SYSCALL_GET_TIME => sys_get_time(),
        SYSCALL_GETPID => sys_getpid(),
        SYSCALL_FORK => sys_fork(),
        SYSCALL_EXEC => sys_exec(args[0] as *const u8, args[1] as *const usize),
        SYSCALL_WAITPID => sys_waitpid(args[0] as isize, args[1] as *mut i32),
        SYSCALL_THREAD_CREATE => sys_thread_create(args[0], args[1]),
        SYSCALL_GETTID => sys_gettid(),
        SYSCALL_WAITTID => sys_waittid(args[0]) as isize,
        SYSCALL_MUTEX_CREATE => sys_mutex_create(args[0] == 1),
        SYSCALL_MUTEX_LOCK => sys_mutex_lock(args[0]),
        SYSCALL_MUTEX_UNLOCK => sys_mutex_unlock(args[0]),
        SYSCALL_SEMAPHORE_CREATE => sys_semaphore_create(args[0]),
        SYSCALL_SEMAPHORE_UP => sys_semaphore_up(args[0]),
        SYSCALL_SEMAPHORE_DOWN => sys_semaphore_down(args[0]),
        SYSCALL_CONDVAR_CREATE => sys_condvar_create(),
        SYSCALL_CONDVAR_SIGNAL => sys_condvar_signal(args[0]),
        SYSCALL_CONDVAR_WAIT => sys_condvar_wait(args[0], args[1]),
        SYSCALL_FRAMEBUFFER => sys_framebuffer(),
        SYSCALL_FRAMEBUFFER_FLUSH => sys_framebuffer_flush(),
        SYSCALL_EVENT_GET => sys_event_get(),
        SYSCALL_KEY_PRESSED => sys_key_pressed(),
        SYSCALL_AGENT_CREATE => agent::create(args[0] as *const agent::AgentCreateArgs),
        SYSCALL_AGENT_INFO => agent::info(args[0] as *mut agent::AgentInfo),
        SYSCALL_TOOL_CALL => agent::tool_call(args[0] as *const agent::ToolRequest, args[1] as *mut agent::ToolResponse),
        SYSCALL_TOOL_LIST => agent::tool_list(args[0] as *mut agent::ToolDescriptor, args[1]),
        SYSCALL_CONTEXT_PUSH => agent::context_push(args[0] as *const agent::ContextNode),
        SYSCALL_CONTEXT_QUERY => agent::context_query(args[0] as *mut agent::ContextNode, args[1]),
        SYSCALL_CONTEXT_ROLLBACK => agent::context_rollback(args[0]),
        SYSCALL_CONTEXT_CLEAR => agent::context_clear(),
        SYSCALL_AGENT_HEARTBEAT_SET => agent::heartbeat_set(args[0]),
        agent_abi::SYS_HEARTBEAT_STOP => agent::heartbeat_set(0),
        agent_abi::SYS_FINISH => agent::finish(),
        agent_abi::SYS_CLOCK => crate::timer::get_time() as isize,
        agent_abi::SYS_WATCH => agent::watch(args[0]),
        agent_abi::SYS_WAIT => agent::wait(args[0]),
        agent_abi::SYS_UNWATCH => agent::unwatch(args[0]),
        agent_abi::SYS_SEND => agent::send(args[0] as *const agent::Message),
        agent_abi::SYS_RECEIVE => agent::receive(args[0] as *mut agent::Message),
        #[cfg(feature = "agent-test")]
        4095 => crate::mm::set_agent_allocation_failure(args[0] as isize),
        #[cfg(feature = "agent-test")]
        4094 => crate::fs::agent_attrs::corrupt_tail_for_test(),
        #[cfg(feature = "agent-test")]
        4093 => agent::test_context_collision(args[0]),
        _ => -38,
    }
}
