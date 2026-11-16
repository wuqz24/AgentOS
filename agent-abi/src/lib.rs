//! Versioned, bounded RV64 little-endian wire records shared by kernel and userspace.
#![no_std]

pub const VERSION: u64 = 1;
pub const MAGIC: u64 = 0x4147_454e_544f_5331;
pub const CONTEXT_BASE: usize = 0x4000_0000;
pub const PAGE_SIZE: usize = 4096;
pub const MAX_CONTEXT_BYTES: usize = 16 * PAGE_SIZE;
pub const MAX_PARAMS: usize = 6;
pub const RESULT_BYTES: usize = 2048;
pub const MESSAGE_BYTES: usize = 1536;
pub const MAX_MESSAGES: usize = 4;

pub const EPERM: isize = -1;
pub const ESRCH: isize = -3;
pub const EAGAIN: isize = -11;
pub const ENOMEM: isize = -12;
pub const EFAULT: isize = -14;
pub const EEXIST: isize = -17;
pub const EINVAL: isize = -22;
pub const ENOSPC: isize = -28;
pub const ENAMETOOLONG: isize = -36;
pub const ENOSYS: isize = -38;
pub const EMSGSIZE: isize = -90;
pub const ETIMEDOUT: isize = -110;
pub const ESTALE: isize = -116;

pub const REGISTERED: u64 = 1;
pub const ACTIVE: u64 = 2;
pub const WAITING: u64 = 3;
pub const DONE: u64 = 4;
pub const EVENT_HEARTBEAT: u64 = 1;
pub const EVENT_MESSAGE: u64 = 2;
pub const EVENT_TIMEOUT: u64 = 4;
pub const EVENT_CANCEL: u64 = 8;
pub const ALL_EVENTS: u64 = 15;
pub const MSG_MODEL_REQUEST: u64 = 1;
pub const MSG_MODEL_RESPONSE: u64 = 2;
pub const MSG_OBSERVATION: u64 = 3;
pub const MSG_GOAL: u64 = 4;
pub const MSG_FINAL: u64 = 5;
pub const MSG_CANCEL: u64 = 6;
pub const MSG_WORKLOAD: u64 = 7;
pub const MSG_EVENT: u64 = 8;
pub const MSG_HELLO: u64 = 9;
pub const KIND_NUMBER: u64 = 1;
pub const KIND_TEXT: u64 = 2;
pub const KIND_BOOL: u64 = 3;

pub const SYS_CREATE: usize = 4000;
pub const SYS_INFO: usize = 4001;
pub const SYS_TOOL_CALL: usize = 4002;
pub const SYS_TOOL_LIST: usize = 4003;
pub const SYS_CONTEXT_PUSH: usize = 4004;
pub const SYS_CONTEXT_QUERY: usize = 4005;
pub const SYS_CONTEXT_ROLLBACK: usize = 4006;
pub const SYS_CONTEXT_CLEAR: usize = 4007;
pub const SYS_HEARTBEAT_SET: usize = 4008;
pub const SYS_HEARTBEAT_STOP: usize = 4009;
pub const SYS_WATCH: usize = 4010;
pub const SYS_WAIT: usize = 4011;
pub const SYS_UNWATCH: usize = 4012;
pub const SYS_FINISH: usize = 4013;
pub const SYS_RECEIVE: usize = 4014;
pub const SYS_SEND: usize = 4015;
pub const SYS_CLOCK: usize = 4016;

/// # Safety
/// Implementors have no padding, references, or invalid bit patterns. All fields
/// must be integers or byte arrays, and every byte must be initialized.
pub unsafe trait Wire: Copy {}
macro_rules! wire {
    ($($name:ty),+ $(,)?) => {$(
        unsafe impl Wire for $name {}
        impl Default for $name {
            fn default() -> Self {
                // SAFETY: these records contain only integers and byte arrays.
                unsafe { core::mem::zeroed() }
            }
        }
    )+};
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AgentCreateArgs {
    pub version: u64,
    pub agent_type: u64,
    /// Number of ContextNode slots, excluding the read-only control page.
    pub quota: u64,
    pub heartbeat_ms: u64,
}
impl AgentCreateArgs {
    pub fn new(agent_type: u64, quota: u64, heartbeat_ms: u64) -> Self {
        Self { version: VERSION, agent_type, quota, heartbeat_ms }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AgentInfo {
    pub magic: u64,
    pub version: u64,
    pub instance: u64,
    pub agent_type: u64,
    pub loop_state: u64,
    pub heartbeat_ms: u64,
    pub quota: u64,
    pub context_len: u64,
    pub context_head: u64,
    pub evictions: u64,
    pub context_base: u64,
    pub context_bytes: u64,
    pub node_size: u64,
    pub next_seq: u64,
    pub generation: u64,
    pub pending_events: u64,
    pub inbox_len: u64,
    pub wakeups: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Param {
    pub key: [u8; 24],
    pub kind: u64,
    pub number: i64,
    pub text: [u8; 96],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ToolRequest {
    pub version: u64,
    pub request_id: u64,
    pub name: [u8; 32],
    pub param_count: u64,
    pub params: [Param; MAX_PARAMS],
}
impl ToolRequest {
    pub fn new(name: &str) -> Result<Self, isize> {
        let mut r = Self { version: VERSION, ..Self::default() };
        put_text(&mut r.name, name)?;
        Ok(r)
    }
    pub fn number(mut self, key: &str, value: i64) -> Result<Self, isize> {
        let mut p = Param { kind: KIND_NUMBER, number: value, ..Param::default() };
        put_text(&mut p.key, key)?;
        self.add(p)?;
        Ok(self)
    }
    pub fn text(mut self, key: &str, value: &str) -> Result<Self, isize> {
        let mut p = Param { kind: KIND_TEXT, ..Param::default() };
        put_text(&mut p.key, key)?;
        put_text(&mut p.text, value)?;
        self.add(p)?;
        Ok(self)
    }
    fn add(&mut self, p: Param) -> Result<(), isize> {
        if self.param_count as usize >= MAX_PARAMS { return EMSGSIZE_ERR; }
        self.params[self.param_count as usize] = p;
        self.param_count += 1;
        Ok(())
    }
}
const EMSGSIZE_ERR: Result<(), isize> = Err(EMSGSIZE);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ToolResponse {
    pub status: i64,
    pub request_id: u64,
    pub timestamp_ms: u64,
    pub len: u64,
    /// UTF-8 JSON result, with named fields and documented units.
    pub json: [u8; RESULT_BYTES],
}
impl ToolResponse {
    pub fn result(&self) -> Result<&str, isize> {
        if self.len as usize > RESULT_BYTES { return Err(EMSGSIZE); }
        core::str::from_utf8(&self.json[..self.len as usize]).map_err(|_| EINVAL)
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ToolDescriptor {
    pub name: [u8; 32],
    /// Compact schema: key:type pairs, ? for optional fields.
    pub params: [u8; 192],
    pub description: [u8; 160],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ContextNode {
    pub seq: u64,
    pub request_id: u64,
    pub timestamp_ms: u64,
    pub status: i64,
    pub tool: [u8; 32],
    pub request: [u8; 96],
    pub result: [u8; 160],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Message {
    pub source_pid: u64,
    pub source_instance: u64,
    pub target_pid: u64,
    pub target_instance: u64,
    pub kind: u64,
    pub len: u64,
    pub data: [u8; MESSAGE_BYTES],
}
wire!(AgentCreateArgs, AgentInfo, Param, ToolRequest, ToolResponse, ToolDescriptor, ContextNode, Message);

pub const MAX_CONTEXT_NODES: usize = (MAX_CONTEXT_BYTES - PAGE_SIZE) / core::mem::size_of::<ContextNode>();

pub fn text(bytes: &[u8]) -> Result<&str, isize> {
    let n = bytes.iter().position(|b| *b == 0).ok_or(ENAMETOOLONG)?;
    if bytes[n..].iter().any(|b| *b != 0) { return Err(EINVAL); }
    core::str::from_utf8(&bytes[..n]).map_err(|_| EINVAL)
}
pub fn put_text(dst: &mut [u8], value: &str) -> Result<(), isize> {
    if value.len() >= dst.len() || value.as_bytes().contains(&0) { return Err(ENAMETOOLONG); }
    dst.fill(0);
    dst[..value.len()].copy_from_slice(value.as_bytes());
    Ok(())
}
pub fn summarize(dst: &mut [u8], value: &str) {
    dst.fill(0);
    let mut n = value.len().min(dst.len().saturating_sub(1));
    while !value.is_char_boundary(n) { n -= 1; }
    dst[..n].copy_from_slice(&value.as_bytes()[..n]);
}

const _: () = assert!(core::mem::size_of::<AgentCreateArgs>() == 32);
const _: () = assert!(core::mem::size_of::<AgentInfo>() == 144);
const _: () = assert!(core::mem::size_of::<Param>() == 136);
const _: () = assert!(core::mem::size_of::<ToolRequest>() == 872);
const _: () = assert!(core::mem::size_of::<ToolResponse>() == 2080);
const _: () = assert!(core::mem::size_of::<ToolDescriptor>() == 384);
const _: () = assert!(core::mem::size_of::<ContextNode>() == 320);
const _: () = assert!(core::mem::size_of::<Message>() == 1584);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_unicode_and_arguments() {
        let mut dst = [0u8; 8];
        summarize(&mut dst, "上下文路径");
        assert_eq!(text(&dst), Ok("上下"));
        assert_eq!(put_text(&mut dst, "12345678"), Err(ENAMETOOLONG));
        let mut req = ToolRequest::new("query_process").unwrap();
        for _ in 0..MAX_PARAMS { req = req.number("pid", 3).unwrap(); }
        assert_eq!(req.number("pid", 4).err(), Some(EMSGSIZE));
    }
    #[test]
    fn malformed_text_and_result_lengths_are_rejected() {
        assert_eq!(text(&[0xff,0]), Err(EINVAL));
        assert_eq!(text(&[b'a',0,b'b']), Err(EINVAL));
        let r = ToolResponse { len: RESULT_BYTES as u64 + 1, ..ToolResponse::default() };
        assert_eq!(r.result(), Err(EMSGSIZE));
    }
}
