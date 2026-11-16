#![no_std]
#![no_main]
extern crate user_lib;
extern crate alloc;
use user_lib::*;
use user_lib::agent_runtime::{emit, tool_specs, request_from_json};
use alloc::{collections::VecDeque, format, string::{String, ToString}, vec::Vec};
use serde_json::{Value, json};

fn spawn(app: &str, goal: &str, rounds: usize) -> isize {
    let parent_pid = getpid();
    let parent_instance = unsafe { mapped_info() }.instance;
    let child = fork();
    if child == 0 {
        let app = format!("{}\0", app);
        let pid = format!("{}\0", parent_pid);
        let instance = format!("{}\0", parent_instance);
        let goal = format!("{}\0", goal);
        let rounds = format!("{}\0", rounds);
        exec(&app, &[app.as_ptr(),pid.as_ptr(),instance.as_ptr(),goal.as_ptr(),rounds.as_ptr(),core::ptr::null()]);
        exit(99);
    }
    child
}

fn dispatch(command: &Value, children: &mut Vec<isize>) -> Result<Value, isize> {
    if command["v"].as_u64() != Some(VERSION) { return Err(EINVAL); }
    let mut code = 0;
    children.retain(|pid| waitpid_nb(*pid as usize, &mut code) == -2);
    match command["command"].as_str().ok_or(EINVAL)? {
        "start" | "workload" | "scenario" => {
            if children.len() >= 6 { return Err(ENOSPC); }
            let is_worker = command["command"] == "workload";
            let scenario = command["command"] == "scenario";
            let goal = if scenario { command["scenario"].as_str().ok_or(EINVAL)? }
                else { command["goal"].as_str().unwrap_or("检查当前系统状态并引用证据") };
            if scenario && !["context","events"].contains(&goal) { return Err(EINVAL); }
            if goal.len() > 768 || goal.as_bytes().contains(&0) { return Err(EMSGSIZE); }
            let rounds = command["max_rounds"].as_u64().unwrap_or(8) as usize;
            if !(1..=16).contains(&rounds) { return Err(EINVAL); }
            let pid = spawn(if is_worker { "agent_worker" } else if scenario { "agent_scenarios" } else { "agent_admin" }, goal, rounds);
            if pid < 0 { return Err(pid); }
            children.push(pid);
            Ok(json!({"child_pid":pid}))
        }
        "deliver" => {
            let payload = command["payload"].to_string();
            if payload.len() > MESSAGE_BYTES { return Err(EMSGSIZE); }
            let mut msg = Message { target_pid:command["target_pid"].as_u64().ok_or(EINVAL)?,
                target_instance:command["target_instance"].as_u64().ok_or(EINVAL)?,
                kind:command["kind"].as_u64().ok_or(EINVAL)?,len:payload.len() as u64,..Message::default() };
            msg.data[..payload.len()].copy_from_slice(payload.as_bytes());
            let status = agent_send(&msg);
            if status < 0 { Err(status) } else { Ok(json!({"delivered":true})) }
        }
        "snapshot" => {
            let mut out = ToolResponse::default();
            let req = ToolRequest::new("get_system_status")?;
            let status = tool_call(&req, &mut out);
            if status < 0 { return Err(status); }
            let system: Value = serde_json::from_str(out.result()?).map_err(|_| EINVAL)?;
            let mut rows = Vec::new();
            let mut cursor = 0;
            let clock_hz = system["clock_hz"].clone();
            loop {
                let req = ToolRequest::new("query_process")?.number("limit", 6)?.number("cursor", cursor)?;
                let status = tool_call(&req, &mut out);
                if status < 0 { return Err(status); }
                let page: Value = serde_json::from_str(out.result()?).map_err(|_| EINVAL)?;
                rows.extend(page["processes"].as_array().ok_or(EINVAL)?.iter().cloned());
                let Some(next) = page["next_cursor"].as_i64() else { break; };
                if next <= cursor || rows.len() >= 24 { return Err(ENOSPC); }
                cursor = next;
            }
            let processes = json!({"processes":rows,"next_cursor":null,"clock_hz":clock_hz});
            Ok(json!({"system":system,"processes":processes}))
        }
        "tool" => {
            let req = request_from_json(command, command["id"].as_u64().unwrap_or(0))?;
            let mut out = ToolResponse::default();
            let status = tool_call(&req, &mut out);
            let result: Value = serde_json::from_str(out.result().unwrap_or("null")).unwrap_or(Value::Null);
            Ok(json!({"status":status,"result":result}))
        }
        _ => Err(ENOSYS),
    }
}

fn shallow_json(bytes: &[u8]) -> bool {
    let (mut depth, mut quoted, mut escaped) = (0usize, false, false);
    for &b in bytes {
        if quoted {
            if escaped { escaped = false; }
            else if b == b'\\' { escaped = true; }
            else if b == b'"' { quoted = false; }
        } else {
            match b {
                b'"' => quoted = true,
                b'{' | b'[' => { depth += 1; if depth > 8 { return false; } },
                b'}' | b']' => { if depth == 0 { return false; } depth -= 1; },
                _ => (),
            }
        }
    }
    !quoted && depth == 0
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    assert!(agent_create(&AgentCreateArgs::new(3, 16, 0)) > 0);
    emit("bridge_ready", json!({"tools":tool_specs()}));
    let mut line = Vec::new();
    let mut overflow = false;
    let mut children = Vec::new();
    let mut cache: VecDeque<(u64, String, String)> = VecDeque::new();
    let mut last_id = 0u64;
    loop {
        let c = console::getchar();
        if c != b'\n' {
            if c != b'\r' {
                if line.len() < 8192 { line.push(c); } else { overflow = true; }
            }
            continue;
        }
        if overflow {
            emit("protocol_error", json!({"error":"inbound_frame_too_large"}));
        } else if shallow_json(&line) && let Ok(command) = serde_json::from_slice::<Value>(&line) {
            let id = command["id"].as_u64().unwrap_or(0);
            let request = command.to_string();
            if let Some((_, previous, reply)) = cache.iter().find(|(n,_,_)| *n == id) {
                let reply = if *previous == request { serde_json::from_str(reply).unwrap() }
                    else { json!({"id":id,"status":EINVAL,"error":"id_reused_with_different_request"}) };
                emit("command_result", reply);
            } else if id == 0 || id <= last_id {
                emit("command_result", json!({"id":id,"status":ESTALE}));
            } else {
                last_id = id;
                let reply = match dispatch(&command, &mut children) {
                    Ok(result) => json!({"id":id,"status":0,"result":result}),
                    Err(code) => json!({"id":id,"status":code}),
                };
                emit("command_result", reply.clone());
                let reply = reply.to_string();
                while cache.len() >= 8 || cache.iter().map(|(_,r,s)| r.len()+s.len()).sum::<usize>() + request.len()+reply.len() > 32768 {
                    cache.pop_front();
                }
                cache.push_back((id, request, reply));
            }
        } else if !line.is_empty() {
            emit("protocol_error", json!({"error":"invalid_json","bytes":line.len()}));
        }
        line.clear();
        overflow = false;
    }
}
