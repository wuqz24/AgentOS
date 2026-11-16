//! Guest-owned Agent loop and bounded serial event formatting.
use crate::*;
use alloc::{format, vec, vec::Vec};
use serde_json::{Value, json};

pub const FRAME_LIMIT: usize = 16 * 1024;

pub fn emit(event: &str, mut value: Value) {
    let a = unsafe { mapped_info() };
    value["v"] = json!(VERSION);
    value["event"] = json!(event);
    value["pid"] = json!(getpid());
    value["instance"] = json!(a.instance);
    value["time_ms"] = json!(get_time());
    let frame = format!("\n@AGENTOS {}\n", value);
    if frame.len() <= FRAME_LIMIT {
        // One syscall per frame: user tasks cannot interleave partial prints.
        write(1, frame.as_bytes());
    } else {
        let small = format!("\n@AGENTOS {{\"v\":1,\"event\":\"protocol_error\",\"pid\":{},\"error\":\"outbound_frame_too_large\"}}\n", getpid());
        write(1, small.as_bytes());
    }
}

pub fn tool_specs() -> Value {
    let mut descriptors = vec![ToolDescriptor::default(); 16];
    let count = tool_list(&mut descriptors);
    let rows: Vec<Value> = descriptors.iter().take(count.max(0) as usize).map(|d|
        json!({"name":text(&d.name).unwrap_or(""),"params":text(&d.params).unwrap_or(""),
               "description":text(&d.description).unwrap_or("")})).collect();
    json!(rows)
}

pub fn context_value() -> Value {
    let a = unsafe { mapped_info() };
    let mut nodes = vec![ContextNode::default(); (a.context_len as usize).min(8)];
    let n = unsafe { context_snapshot(&mut nodes) };
    let rows: Vec<Value> = nodes[..n].iter().map(|node| json!({
        "seq":node.seq,"request_id":node.request_id,"tool":text(&node.tool).unwrap_or("invalid"),
        "request":text(&node.request).unwrap_or("invalid"),"result":text(&node.result).unwrap_or("invalid"),
        "status":node.status,"time_ms":node.timestamp_ms})).collect();
    json!({"length":a.context_len,"capacity":a.quota,"head":a.context_head,"evictions":a.evictions,
        "next_seq":a.next_seq,"data_base":a.context_base,"control_base":CONTEXT_BASE,
        "mapped_bytes":a.context_bytes,"node_bytes":a.node_size,"generation":a.generation,
        "loop_state":a.loop_state,"wakeups":a.wakeups,"nodes":rows})
}

pub fn request_from_json(value: &Value, id: u64) -> Result<ToolRequest, isize> {
    let name = value["name"].as_str().ok_or(EINVAL)?;
    let mut req = ToolRequest::new(name)?;
    req.request_id = id;
    if let Some(params) = value.get("params") {
        let params = params.as_object().ok_or(EINVAL)?;
        if params.len() > MAX_PARAMS { return Err(EMSGSIZE); }
        for (key, value) in params {
            req = if let Some(number) = value.as_i64() { req.number(key, number)? }
                else if let Some(text) = value.as_str() { req.text(key, text)? }
                else { return Err(EINVAL); };
        }
    }
    Ok(req)
}

pub fn run(goal: &str, bridge_pid: u64, bridge_instance: u64, max_rounds: usize) -> i32 {
    emit("agent_ready", json!({"goal":goal,"context":context_value()}));
    let mut latest = Value::Null;
    for round in 0..=max_rounds {
        let id = (unsafe { mapped_info() }.instance << 32) | (round as u64 + 1);
        emit("model_request", json!({"request_id":id,"round":round,"max_rounds":max_rounds,
            "goal":goal,"latest":latest,"context":context_value()}));
        let deadline = get_time() + 120_000;
        let decision = loop {
            let mut msg = Message::default();
            let received = agent_receive(&mut msg);
            if received == EAGAIN {
                let remaining = deadline - get_time();
                if remaining <= 0 {
                    emit("task_error", json!({"error":"model_timeout","context":context_value()}));
                    agent_finish();
                    return 1;
                }
                agent_wait(remaining as usize);
                continue;
            }
            if received < 0 {
                emit("task_error", json!({"error":"receive_failed","code":received,"context":context_value()}));
                agent_finish();
                return 1;
            }
            if msg.source_pid != bridge_pid || msg.source_instance != bridge_instance {
                emit("decision_rejected", json!({"reason":"unexpected_sender"}));
                continue;
            }
            if msg.kind == MSG_CANCEL {
                emit("task_cancelled", json!({"context":context_value()}));
                agent_finish();
                return 0;
            }
            if msg.kind != MSG_MODEL_RESPONSE { continue; }
            let Ok(value) = serde_json::from_slice::<Value>(&msg.data[..msg.len as usize]) else {
                emit("decision_rejected", json!({"reason":"invalid_json"}));
                continue;
            };
            if value["request_id"].as_u64() != Some(id) {
                emit("decision_rejected", json!({"reason":"stale_request_id"}));
                continue;
            }
            break value;
        };
        if decision["action"] == "error" {
            emit("task_error", json!({"error":decision["text"],"context":context_value()}));
            agent_finish();
            return 1;
        }
        let ctx = context_value();
        let valid_evidence = decision["evidence"].as_array().is_some_and(|refs|
            !refs.is_empty() && refs.iter().all(|r| r.as_u64().is_some_and(|seq|
                ctx["nodes"].as_array().unwrap().iter().any(|n| n["seq"].as_u64() == Some(seq)))));
        if decision["action"] == "final" && valid_evidence && decision["text"].as_str().is_some_and(|s| !s.is_empty()) {
            emit("task_final", json!({"text":decision["text"],"evidence":decision["evidence"],
                "context":context_value(),"complete":true}));
            agent_finish();
            return 0;
        }
        if round == max_rounds {
            emit("task_final", json!({"text":"达到工具调用上限，保留已取得的证据；本次调查尚未完成。",
                "context":context_value(),"complete":false}));
            agent_finish();
            return 0;
        }
        if decision["action"] != "tool" {
            latest = json!({"source":"userspace_validation","status":EINVAL,
                "error":"A final answer requires actual observations; choose one listed tool."});
            continue;
        }
        let req = match request_from_json(&decision, id) {
            Ok(req) => req,
            Err(code) => {
                latest = json!({"source":"userspace_validation","status":code,"error":"Invalid tool decision"});
                emit("decision_rejected", latest.clone());
                continue;
            }
        };
        emit("tool_start", json!({"request_id":id,"name":decision["name"],"params":decision["params"],"note":decision["note"]}));
        let start = agent_clock();
        let mut result = ToolResponse::default();
        let status = tool_call_recorded(&req, &mut result);
        let ticks = agent_clock() - start;
        let data = serde_json::from_str::<Value>(result.result().unwrap_or("null")).unwrap_or(Value::Null);
        latest = json!({"source":"kernel","request_id":id,"name":decision["name"],
            "params":decision["params"],"status":status,"result":data,
            "context_seq":unsafe { mapped_info() }.next_seq - 1});
        emit("tool_result", json!({"observation":latest,"elapsed_ticks":ticks,"context":context_value()}));
    }
    agent_finish();
    0
}
