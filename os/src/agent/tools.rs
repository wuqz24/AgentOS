use super::*;
use alloc::{string::ToString, vec::Vec};
use serde_json::{Value, json};
use crate::task::{TaskStatus, process_snapshot};

struct Spec { name: &'static str, params: &'static str, description: &'static str }
const SPECS: &[Spec] = &[
    Spec { name: "get_system_status", params: "", description: "Guest process counts, physical frames and clock; no host CPU metrics." },
    Spec { name: "query_process", params: "pid:number?,type:text?,status:text?,limit:number?,cursor:number?", description: "Filter real guest processes; runtime_ticks are cumulative guest clock ticks." },
    Spec { name: "read_context", params: "", description: "Current Agent Context metadata and bounded history window." },
    Spec { name: "send_message", params: "target_pid:number,target_instance:number,kind:number?,message:text", description: "Send a bounded message within this Agent family; instance prevents PID reuse." },
    Spec { name: "set_file_attr", params: "path:text,key:text,value:text", description: "Set an attribute on an existing flat file; key <=23 bytes, value <=63 bytes, at most 8 attributes." },
    Spec { name: "delete_file_attr", params: "path:text,key:text", description: "Delete an attribute and update the inverted index and persistent journal." },
    Spec { name: "get_file_attrs", params: "path:text", description: "Read all attributes and the stable catalog identity of a file." },
    Spec { name: "query_files", params: "filter:text,keyword:text?,mode:text?,limit:number?,cursor:number?", description: "AND query, e.g. type=log,owner=4. Optional summary keyword. mode=index|scan, limit 1..4, next_cursor pagination." },
    Spec { name: "read_file_summary", params: "path:text", description: "Read the first 127 UTF-8 bytes of file content; summary cache invalidates on ordinary writes." },
    Spec { name: "file_index_status", params: "", description: "Attribute catalog capacity, journal checkpoint epoch and detected recovery damage." },
];

pub struct Params<'a>(&'a ToolRequest);
impl<'a> Params<'a> {
    fn get(&self, key: &str) -> Option<&Param> {
        self.0.params[..self.0.param_count as usize].iter().find(|p| text(&p.key) == Ok(key))
    }
    pub fn number(&self, key: &str, fallback: i64) -> i64 { self.get(key).map_or(fallback, |p| p.number) }
    pub fn text(&self, key: &str) -> &str { self.get(key).and_then(|p| text(&p.text).ok()).unwrap_or("") }
}

fn validate<'a>(req: &'a ToolRequest, spec: &Spec) -> Result<Params<'a>, isize> {
    if req.version != VERSION || req.param_count as usize > MAX_PARAMS { return Err(EINVAL); }
    for (i, p) in req.params[..req.param_count as usize].iter().enumerate() {
        let key = text(&p.key)?;
        if req.params[..i].iter().any(|v| text(&v.key) == Ok(key)) { return Err(EINVAL); }
        let rule = spec.params.split(',').find_map(|r| r.split_once(':').filter(|(k,_)| *k == key)).ok_or(EINVAL)?;
        match rule.1.trim_end_matches('?') {
            "number" if p.kind == KIND_NUMBER && p.text.iter().all(|x| *x == 0) => (),
            "text" if p.kind == KIND_TEXT && p.number == 0 => { text(&p.text)?; },
            _ => return Err(EINVAL),
        }
    }
    let params = Params(req);
    for rule in spec.params.split(',').filter(|r| !r.is_empty()) {
        let (key, kind) = rule.split_once(':').ok_or(EINVAL)?;
        if !kind.ends_with('?') && params.get(key).is_none() { return Err(EINVAL); }
    }
    Ok(params)
}

pub fn list(out: *mut ToolDescriptor, capacity: usize) -> isize {
    let n = capacity.min(SPECS.len());
    let token = current_user_token();
    if validate_user_buffer(token, out as usize, n * core::mem::size_of::<ToolDescriptor>(), true).is_err() { return EFAULT; }
    for (i, s) in SPECS[..n].iter().enumerate() {
        let mut d = ToolDescriptor::default();
        put_text(&mut d.name, s.name).unwrap();
        put_text(&mut d.params, s.params).unwrap();
        put_text(&mut d.description, s.description).unwrap();
        if let Err(e) = write_user(token, unsafe { out.add(i) }, &d) { return e; }
    }
    n as isize
}

pub fn call(ptr: *const ToolRequest, out: *mut ToolResponse) -> isize {
    let token = current_user_token();
    if validate_user_buffer(token, out as usize, core::mem::size_of::<ToolResponse>(), true).is_err() { return EFAULT; }
    let req = match read_user(token, ptr) { Ok(v) => v, Err(e) => return e };
    {
        let process = current_process();
        let mut inner = process.inner_exclusive_access();
        if !inner.agent.live() { return EPERM; }
        inner.agent.info.loop_state = ACTIVE;
        inner.agent.info.generation += 2;
        sync_control(&inner.memory_set, &inner.agent);
    }
    let result = (|| {
        let name = text(&req.name)?;
        let spec = SPECS.iter().find(|s| s.name == name).ok_or(ENOSYS)?;
        let params = validate(&req, spec)?;
        match name {
            "get_system_status" => system_status(),
            "query_process" => query_process(&params),
            "read_context" => read_context(),
            "send_message" => send_message(&params),
            "set_file_attr" => crate::fs::agent_attrs::set(params.text("path"),params.text("key"),Some(params.text("value"))),
            "delete_file_attr" => crate::fs::agent_attrs::set(params.text("path"),params.text("key"),None),
            "get_file_attrs" => crate::fs::agent_attrs::attributes(params.text("path")),
            "query_files" => crate::fs::agent_attrs::query(params.text("filter"),params.text("keyword"),params.text("mode"),params.number("limit",4),params.number("cursor",0)),
            "read_file_summary" => crate::fs::agent_attrs::read_summary(params.text("path")),
            "file_index_status" => Ok(crate::fs::agent_attrs::status()),
            _ => Err(ENOSYS),
        }
    })();
    let (status, value) = match result { Ok(v) => (0, v), Err(e) => (e, json!({"error": error_name(e), "code": e})) };
    let mut serialized = value.to_string();
    let mut status = status;
    if serialized.len() > RESULT_BYTES {
        status = ENOSPC;
        serialized = json!({"error":"result_too_large", "capacity":RESULT_BYTES}).to_string();
    }
    let mut response = ToolResponse { status: status as i64, request_id: req.request_id,
        timestamp_ms: get_time_ms() as u64, len: serialized.len() as u64, ..ToolResponse::default() };
    response.json[..serialized.len()].copy_from_slice(serialized.as_bytes());
    write_user(token, out, &response).map_or_else(|e| e, |_| status)
}

fn error_name(e: isize) -> &'static str {
    match e { ENOSYS => "unknown_tool", EINVAL => "invalid_parameter", EPERM => "permission_denied",
        ESRCH => "process_not_found", ENOSPC => "capacity_exceeded", ESTALE => "stale_instance",
        EFAULT => "invalid_user_buffer", _ => "tool_error" }
}

fn system_status() -> Result<Value, isize> {
    let list = process_snapshot();
    let mut agents = 0;
    let mut ready = 0;
    let mut blocked = 0;
    for p in &list {
        let inner = p.inner_exclusive_access();
        if inner.agent.live() { agents += 1; }
        for task in inner.tasks.iter().flatten() {
            match task.inner_exclusive_access().task_status {
                TaskStatus::Blocked => blocked += 1,
                _ => ready += 1,
            }
        }
    }
    let (total, free) = crate::mm::frame_stats();
    Ok(json!({"processes":list.len(), "agents":agents, "runnable_tasks":ready,
        "blocked_tasks":blocked, "physical_pages_total":total, "physical_pages_free":free,
        "page_bytes":PAGE_SIZE, "clock_hz":crate::device_tree::clock_freq(),
        "uptime_ms":get_time_ms()}))
}

fn query_process(params: &Params) -> Result<Value, isize> {
    let pid_filter = params.number("pid", -1);
    let cursor = params.number("cursor", 0);
    let limit = params.number("limit", 4);
    if pid_filter < -1 || cursor < 0 || !(1..=6).contains(&limit)
        || !["", "agent", "normal"].contains(&params.text("type"))
        || !["", "ready", "running", "blocked"].contains(&params.text("status")) { return Err(EINVAL); }
    let now = crate::timer::get_time() as u64;
    let mut rows = Vec::new();
    let mut next = Value::Null;
    for p in process_snapshot() {
        let pid = p.getpid();
        if pid < cursor as usize || (pid_filter >= 0 && pid != pid_filter as usize) { continue; }
        let inner = p.inner_exclusive_access();
        let is_agent = inner.agent.live();
        if (params.text("type") == "agent" && !is_agent) || (params.text("type") == "normal" && is_agent) { continue; }
        let mut state = "blocked";
        let mut ticks = 0;
        let mut dispatches = 0;
        for task in inner.tasks.iter().flatten() {
            let t = task.inner_exclusive_access();
            ticks += t.run_ticks;
            dispatches += t.dispatches;
            match t.task_status {
                TaskStatus::Running => { state = "running"; ticks += now.saturating_sub(t.run_since); }
                TaskStatus::Ready if state != "running" => state = "ready",
                _ => (),
            }
        }
        if !params.text("status").is_empty() && params.text("status") != state { continue; }
        if rows.len() == limit as usize { next = json!(pid); break; }
        rows.push(json!({"pid":pid, "name":inner.name, "agent":is_agent,
            "instance":inner.agent.info.instance, "state":state, "runtime_ticks":ticks,
            "dispatches":dispatches, "mapped_data_pages":inner.memory_set.resident_data_pages(),
            "context_len":inner.agent.info.context_len, "loop_state":inner.agent.info.loop_state}));
    }
    if pid_filter >= 0 && rows.is_empty() { return Err(ESRCH); }
    Ok(json!({"processes":rows, "next_cursor":next, "clock_hz":crate::device_tree::clock_freq()}))
}

fn read_context() -> Result<Value, isize> {
    let process = current_process();
    let inner = process.inner_exclusive_access();
    let a = &inner.agent.info;
    Ok(json!({"length":a.context_len,"capacity":a.quota,"head":a.context_head,
        "evictions":a.evictions,"next_seq":a.next_seq,"data_base":a.context_base,
        "mapped_bytes":a.context_bytes,"generation":a.generation,"node_bytes":a.node_size}))
}

fn send_message(params: &Params) -> Result<Value, isize> {
    let pid = params.number("target_pid", -1);
    let instance = params.number("target_instance", 0);
    let kind = params.number("kind", MSG_EVENT as i64);
    if pid < 0 || instance <= 0 || !(1..=32).contains(&kind) { return Err(EINVAL); }
    let text = params.text("message").as_bytes();
    let mut msg = Message { target_pid: pid as u64, target_instance: instance as u64,
        kind: kind as u64, len: text.len() as u64, ..Message::default() };
    msg.data[..text.len()].copy_from_slice(text);
    let result = super::events::send_message(msg);
    if result < 0 { Err(result) } else { Ok(json!({"delivered":true,"target_pid":pid,"target_instance":instance})) }
}
