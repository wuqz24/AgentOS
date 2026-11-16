//! Single-writer Agent SDK. Raw mapped reads require a live Agent registration.
pub use agent_abi::*;
use crate::syscall::syscall;

pub fn agent_create(args: &AgentCreateArgs) -> isize { syscall(SYS_CREATE, [args as *const _ as usize, 0, 0]) }
pub fn agent_info(out: &mut AgentInfo) -> isize { syscall(SYS_INFO, [out as *mut _ as usize, 0, 0]) }
pub fn tool_call(req: &ToolRequest, out: &mut ToolResponse) -> isize { syscall(SYS_TOOL_CALL, [req as *const _ as usize, out as *mut _ as usize, 0]) }
pub fn tool_list(out: &mut [ToolDescriptor]) -> isize { syscall(SYS_TOOL_LIST, [out.as_mut_ptr() as usize, out.len(), 0]) }
pub fn context_push(node: &ContextNode) -> isize { syscall(SYS_CONTEXT_PUSH, [node as *const _ as usize, 0, 0]) }
pub fn context_query(out: &mut [ContextNode]) -> isize { syscall(SYS_CONTEXT_QUERY, [out.as_mut_ptr() as usize, out.len(), 0]) }
pub fn context_rollback(keep: usize) -> isize { syscall(SYS_CONTEXT_ROLLBACK, [keep, 0, 0]) }
pub fn context_clear() -> isize { syscall(SYS_CONTEXT_CLEAR, [0; 3]) }
pub fn agent_heartbeat_set(ms: usize) -> isize { syscall(SYS_HEARTBEAT_SET, [ms, 0, 0]) }
pub fn agent_heartbeat_stop() -> isize { syscall(SYS_HEARTBEAT_STOP, [0; 3]) }
pub fn agent_finish() -> isize { syscall(SYS_FINISH, [0; 3]) }
pub fn agent_clock() -> u64 { syscall(SYS_CLOCK, [0; 3]) as u64 }
pub fn agent_watch(mask: u64) -> isize { syscall(SYS_WATCH, [mask as usize, 0, 0]) }
pub fn agent_unwatch(mask: u64) -> isize { syscall(SYS_UNWATCH, [mask as usize, 0, 0]) }
pub fn agent_wait(timeout_ms: usize) -> isize { syscall(SYS_WAIT, [timeout_ms, 0, 0]) }
pub fn agent_send(message: &Message) -> isize { syscall(SYS_SEND, [message as *const _ as usize, 0, 0]) }
pub fn agent_receive(message: &mut Message) -> isize { syscall(SYS_RECEIVE, [message as *mut _ as usize, 0, 0]) }

/// # Safety
/// Pointer arguments must follow the selected ABI. Intended for negative tests.
pub unsafe fn agent_raw(id: usize, args: [usize; 3]) -> isize { syscall(id, args) }

/// # Safety
/// The current process must have a live Agent mapping, with one Context writer.
pub unsafe fn mapped_info() -> AgentInfo {
    let ptr = CONTEXT_BASE as *const AgentInfo;
    loop {
        let before = unsafe { core::ptr::addr_of!((*ptr).generation).read_volatile() };
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Acquire);
        let info = unsafe { ptr.read_volatile() };
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Acquire);
        let after = unsafe { core::ptr::addr_of!((*ptr).generation).read_volatile() };
        if before == after && after & 1 == 0 { return info; }
    }
}

/// # Safety
/// Same requirements as mapped_info. User code must not alter ring metadata.
pub unsafe fn context_snapshot(out: &mut [ContextNode]) -> usize {
    loop {
        let info = unsafe { mapped_info() };
        let n = out.len().min(info.context_len as usize);
        for (i, node) in out[..n].iter_mut().enumerate() {
            let slot = (info.context_head as usize + info.quota as usize - n + i) % info.quota as usize;
            *node = unsafe { (info.context_base as *const ContextNode).add(slot).read_volatile() };
        }
        if info.generation == unsafe { mapped_info() }.generation { return n; }
    }
}

pub fn tool_call_recorded(req: &ToolRequest, out: &mut ToolResponse) -> isize {
    let status = tool_call(req, out);
    let mut node = ContextNode { request_id: req.request_id, status: status as i64,
        tool: req.name, ..ContextNode::default() };
    let mut args = alloc::string::String::new();
    use core::fmt::Write;
    for p in req.params.iter().take((req.param_count as usize).min(MAX_PARAMS)) {
        let key = text(&p.key).unwrap_or("?");
        if p.kind == KIND_TEXT { let _ = write!(args, "{}={} ", key, text(&p.text).unwrap_or("?")); }
        else { let _ = write!(args, "{}={} ", key, p.number); }
    }
    summarize(&mut node.request, &args);
    summarize(&mut node.result, out.result().unwrap_or("invalid response"));
    let recorded = context_push(&node);
    if recorded < 0 { recorded } else { status }
}
