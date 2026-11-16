#![no_std]
#![no_main]
extern crate user_lib;
extern crate alloc;
use alloc::{format,string::String};
use user_lib::*;
use user_lib::agent_runtime::emit;
use serde_json::{Value,json};

fn diagnostic(path: &str, state: &str) {
    let file=format!("{}\0",path);
    let fd=open(&file,OpenFlags::CREATE|OpenFlags::WRONLY);
    if fd>=0 {
        let info=unsafe {mapped_info()};
        let body=format!("Controlled CPU worker pid={} instance={} state={} guest_ms={}. Pause via MSG_WORKLOAD {{\"busy\":false}}; cancel via MSG_CANCEL.\n",getpid(),info.instance,state,get_time());
        write(fd as usize,body.as_bytes());close(fd as usize);
    }
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    assert!(agent_create(&AgentCreateArgs::new(4, 4, 0)) > 0);
    let path: String=format!("worker-{}-{}.log",getpid(),unsafe {mapped_info()}.instance);
    diagnostic(&path,"busy");
    for (key,value) in [("type",String::from("log")),("tag",String::from("diagnostic")),("owner",format!("{}",getpid())),("instance",format!("{}",unsafe {mapped_info()}.instance))] {
        let req=ToolRequest::new("set_file_attr").unwrap().text("path",&path).unwrap().text("key",key).unwrap().text("value",&value).unwrap();
        let mut out=ToolResponse::default();
        assert_eq!(tool_call(&req,&mut out),0);
    }
    emit("worker_ready", json!({"state":"busy","description":"controlled integer-computation workload"}));
    let mut value = 1u64;
    let mut busy = true;
    loop {
        if busy {
            for _ in 0..100000 { value = value.wrapping_mul(6364136223846793005).wrapping_add(1); core::hint::black_box(value); }
        } else { agent_wait(1000); }
        let mut msg = Message::default();
        while agent_receive(&mut msg) == 0 {
            if msg.kind == MSG_CANCEL {
                diagnostic(&path,"stopped");
                emit("worker_stopped", json!({"checksum":value}));
                agent_finish();
                return 0;
            }
            if msg.kind == MSG_WORKLOAD {
                let payload: Value = serde_json::from_slice(&msg.data[..msg.len as usize]).unwrap_or(Value::Null);
                let Some(value)=payload["busy"].as_bool() else {continue;};
                busy = value;
                diagnostic(&path,if busy {"busy"} else {"waiting"});
                emit("worker_state", json!({"state":if busy {"busy"} else {"waiting"}}));
            }
        }
    }
}
