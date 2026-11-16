//! Agent mechanisms. Strategy and model inference remain outside the kernel.
pub use agent_abi::*;
mod tools;
mod events;
pub use events::{poll_events, wait, watch, unwatch, send, receive};

use alloc::collections::VecDeque;
use core::sync::atomic::{AtomicU64, Ordering};
use crate::mm::{MemorySet, MapPermission, VirtAddr, copy_from_user, copy_to_user, validate_user_buffer};
use crate::task::{current_process, current_user_token};
use crate::timer::get_time_ms;

static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(1);
pub struct AgentKernelState {
    pub info: AgentInfo,
    pub domain: u64,
    pub inbox: VecDeque<Message>,
    pub watched: u64,
    pub heartbeat_due: u64,
    pub wait_deadline: u64,
}
impl AgentKernelState {
    pub fn new() -> Self {
        Self { info: AgentInfo::default(), domain: 0, inbox: VecDeque::new(),
            watched: EVENT_MESSAGE | EVENT_CANCEL, heartbeat_due: 0, wait_deadline: 0 }
    }
    pub fn is_agent(&self) -> bool { self.info.agent_type != 0 }
    pub fn live(&self) -> bool { self.is_agent() && self.info.loop_state != DONE }
    pub fn forked(&self) -> Self {
        let mut state = Self::new();
        if self.live() {
            state.info = self.info;
            state.info.instance = NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed);
            state.info.heartbeat_ms = 0;
            state.info.loop_state = REGISTERED;
            state.info.pending_events = 0;
            state.info.inbox_len = 0;
            state.info.wakeups = 0;
            state.info.generation += 2;
            state.domain = self.domain;
        }
        state
    }
}

pub fn sync_control(memory: &MemorySet, agent: &AgentKernelState) {
    if !agent.live() || agent.info.context_base == 0 { return; }
    let Some(pte) = memory.translate(VirtAddr::from(CONTEXT_BASE).floor()) else { return; };
    if !pte.is_valid() { return; }
    let page = pte.ppn().get_bytes_array();
    let offset = core::mem::offset_of!(AgentInfo, generation);
    let generation = agent.info.generation;
    page[offset..offset+8].copy_from_slice(&(generation | 1).to_le_bytes());
    core::sync::atomic::compiler_fence(Ordering::Release);
    let bytes = wire_bytes(&agent.info);
    page[..offset].copy_from_slice(&bytes[..offset]);
    page[offset+8..bytes.len()].copy_from_slice(&bytes[offset+8..]);
    core::sync::atomic::compiler_fence(Ordering::Release);
    page[offset..offset+8].copy_from_slice(&generation.to_le_bytes());
}

pub fn create(args_ptr: *const AgentCreateArgs) -> isize {
    let token = current_user_token();
    let args = match read_user(token, args_ptr) { Ok(v) => v, Err(e) => return e };
    if args.version != VERSION || !(1..=4).contains(&args.agent_type)
        || args.quota == 0 || args.quota as usize > MAX_CONTEXT_NODES
        || !valid_heartbeat(args.heartbeat_ms) { return EINVAL; }
    let process = current_process();
    let parent = process.inner_exclusive_access().parent.as_ref().and_then(|p| p.upgrade());
    let parent_domain = parent.map(|p| p.inner_exclusive_access().agent.domain).unwrap_or(0);
    let mut inner = process.inner_exclusive_access();
    if inner.agent.live() { return EEXIST; }
    if inner.tasks.iter().flatten().count() != 1 { return EINVAL; }
    let bytes = (args.quota as usize * core::mem::size_of::<ContextNode>()).div_ceil(PAGE_SIZE) * PAGE_SIZE;
    if let Err(e) = inner.memory_set.try_insert_framed_area(CONTEXT_BASE,
        CONTEXT_BASE + PAGE_SIZE, MapPermission::R | MapPermission::U) { return e; }
    if let Err(e) = inner.memory_set.try_insert_framed_area(CONTEXT_BASE + PAGE_SIZE,
        CONTEXT_BASE + PAGE_SIZE + bytes, MapPermission::R | MapPermission::W | MapPermission::U) {
        inner.memory_set.remove_area_with_start_vpn(VirtAddr::from(CONTEXT_BASE).floor());
        return e;
    }
    let instance = NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed);
    let mut state = AgentKernelState::new();
    state.info = AgentInfo { magic: MAGIC, version: VERSION, instance,
        agent_type: args.agent_type, loop_state: REGISTERED, heartbeat_ms: args.heartbeat_ms,
        quota: args.quota, context_base: (CONTEXT_BASE + PAGE_SIZE) as u64,
        context_bytes: (PAGE_SIZE + bytes) as u64,
        node_size: core::mem::size_of::<ContextNode>() as u64, generation: 2,
        ..AgentInfo::default() };
    state.domain = if parent_domain != 0 { parent_domain } else { instance };
    state.heartbeat_due = if args.heartbeat_ms == 0 { 0 } else { get_time_ms() as u64 + args.heartbeat_ms };
    inner.agent = state;
    sync_control(&inner.memory_set, &inner.agent);
    inner.agent.info.context_base as isize
}

pub fn info(out: *mut AgentInfo) -> isize {
    let process = current_process();
    let inner = process.inner_exclusive_access();
    if !inner.agent.is_agent() { return EPERM; }
    write_user(inner.memory_set.token(), out, &inner.agent.info).map_or_else(|e| e, |_| 0)
}

#[cfg(feature="agent-test")]
pub fn test_context_collision(map: usize) -> isize {
    let process=current_process();let mut inner=process.inner_exclusive_access();
    if inner.agent.live() {return EPERM;}
    if map==1 {
        inner.memory_set.try_insert_framed_area(CONTEXT_BASE,CONTEXT_BASE+PAGE_SIZE,MapPermission::R|MapPermission::W|MapPermission::U).map_or_else(|e|e, |_|0)
    } else {
        inner.memory_set.remove_area_with_start_vpn(VirtAddr::from(CONTEXT_BASE).floor());0
    }
}

pub fn context_push(ptr: *const ContextNode) -> isize {
    let token = current_user_token();
    let mut node = match read_user(token, ptr) { Ok(v) => v, Err(e) => return e };
    if text(&node.tool).is_err() || text(&node.request).is_err() || text(&node.result).is_err() { return EINVAL; }
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    if !inner.agent.live() { return EPERM; }
    let a = &mut inner.agent.info;
    let addr = a.context_base as usize + a.context_head as usize * core::mem::size_of::<ContextNode>();
    node.seq = a.next_seq;
    node.timestamp_ms = get_time_ms() as u64;
    if let Err(e) = write_user(token, addr as *mut ContextNode, &node) { return e; }
    a.next_seq += 1;
    a.context_head = (a.context_head + 1) % a.quota;
    if a.context_len < a.quota { a.context_len += 1; } else { a.evictions += 1; }
    a.generation += 2;
    sync_control(&inner.memory_set, &inner.agent);
    0
}

pub fn context_query(out: *mut ContextNode, max: usize) -> isize {
    let process = current_process();
    let inner = process.inner_exclusive_access();
    if !inner.agent.live() { return EPERM; }
    let a = &inner.agent.info;
    let n = max.min(a.context_len as usize);
    let token = inner.memory_set.token();
    if validate_user_buffer(token, out as usize, n * core::mem::size_of::<ContextNode>(), true).is_err() { return EFAULT; }
    for i in 0..n {
        let slot = (a.context_head as usize + a.quota as usize - n + i) % a.quota as usize;
        let src = (a.context_base as usize + slot * core::mem::size_of::<ContextNode>()) as *const ContextNode;
        let node = match read_user(token, src) { Ok(v) => v, Err(e) => return e };
        if let Err(e) = write_user(token, unsafe { out.add(i) }, &node) { return e; }
    }
    n as isize
}

pub fn context_rollback(keep: usize) -> isize {
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    if !inner.agent.live() { return EPERM; }
    let a = &mut inner.agent.info;
    if keep > a.context_len as usize { return EINVAL; }
    let oldest = (a.context_head + a.quota - a.context_len) % a.quota;
    a.context_len = keep as u64;
    a.context_head = (oldest + keep as u64) % a.quota;
    a.generation += 2;
    sync_control(&inner.memory_set, &inner.agent);
    0
}

pub fn context_clear() -> isize {
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    if !inner.agent.live() { return EPERM; }
    inner.agent.info.context_len = 0;
    inner.agent.info.context_head = 0;
    inner.agent.info.generation += 2;
    sync_control(&inner.memory_set, &inner.agent);
    0
}

pub fn valid_heartbeat(ms: u64) -> bool { ms == 0 || (10..=60_000).contains(&ms) }
pub fn heartbeat_set(ms: usize) -> isize {
    if !valid_heartbeat(ms as u64) { return EINVAL; }
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    if !inner.agent.live() { return EPERM; }
    inner.agent.info.heartbeat_ms = ms as u64;
    inner.agent.heartbeat_due = if ms == 0 { 0 } else { get_time_ms() as u64 + ms as u64 };
    inner.agent.info.pending_events &= !EVENT_HEARTBEAT;
    inner.agent.info.generation += 2;
    sync_control(&inner.memory_set, &inner.agent);
    0
}

pub fn finish() -> isize {
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    if !inner.agent.live() { return EPERM; }
    inner.memory_set.remove_area_with_start_vpn(VirtAddr::from(CONTEXT_BASE + PAGE_SIZE).floor());
    inner.memory_set.remove_area_with_start_vpn(VirtAddr::from(CONTEXT_BASE).floor());
    inner.agent.inbox.clear();
    inner.agent.info.loop_state = DONE;
    inner.agent.info.heartbeat_ms = 0;
    inner.agent.info.context_base = 0;
    inner.agent.info.context_bytes = 0;
    inner.agent.info.context_len = 0;
    inner.agent.info.quota = 0;
    inner.agent.info.inbox_len = 0;
    inner.agent.info.pending_events = 0;
    inner.agent.heartbeat_due = 0;
    inner.agent.wait_deadline = 0;
    inner.agent.watched = 0;
    0
}

pub fn tool_call(req: *const ToolRequest, out: *mut ToolResponse) -> isize { tools::call(req, out) }
pub fn tool_list(out: *mut ToolDescriptor, len: usize) -> isize { tools::list(out, len) }

pub fn read_user<T: Wire>(token: usize, ptr: *const T) -> Result<T, isize> {
    let mut value = core::mem::MaybeUninit::<T>::uninit();
    let dst = unsafe { core::slice::from_raw_parts_mut(value.as_mut_ptr().cast::<u8>(), core::mem::size_of::<T>()) };
    copy_from_user(token, ptr.cast::<u8>(), dst).map_err(|_| EFAULT)?;
    Ok(unsafe { value.assume_init() })
}
pub fn wire_bytes<T: Wire>(value: &T) -> &[u8] {
    unsafe { core::slice::from_raw_parts((value as *const T).cast::<u8>(), core::mem::size_of::<T>()) }
}
pub fn write_user<T: Wire>(token: usize, ptr: *mut T, value: &T) -> Result<(), isize> {
    copy_to_user(token, ptr.cast::<u8>(), wire_bytes(value)).map_err(|_| EFAULT)
}
