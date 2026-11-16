#![no_std]
#![no_main]
extern crate user_lib;
extern crate alloc;
use user_lib::*;
use serde_json::{json,Value};
use agent_runtime::{emit,context_value};

fn pause(ms:usize) {
    agent_wait(ms);
    let mut msg=Message::default();
    while agent_receive(&mut msg)==0 {
        if msg.kind==MSG_CANCEL {emit("task_cancelled",json!({"context":context_value()}));agent_finish();exit(0);}
    }
}
fn execute(name:&str) {
    let mut req=ToolRequest::new(name).unwrap();req.request_id=unsafe {mapped_info()}.next_seq+1;
    let mut out=ToolResponse::default();
    emit("tool_start",json!({"name":name,"params":{},"note":"内核机制验证场景"}));
    assert_eq!(tool_call_recorded(&req,&mut out),0);
    let value:Value=serde_json::from_str(out.result().unwrap()).unwrap();
    emit("tool_result",json!({"observation":{"name":name,"source":"kernel","result":value,"status":0},"context":context_value()}));
}
fn send(pid:u64,instance:u64,kind:u64) {
    assert_eq!(agent_send(&Message{target_pid:pid,target_instance:instance,kind,..Message::default()}),0);
}
fn receive()->Message {loop {let mut m=Message::default();if agent_receive(&mut m)==0{return m;}assert_ne!(agent_wait(2000) as u64&EVENT_MESSAGE,0);}}
#[unsafe(no_mangle)]
pub fn main(argc:usize,argv:&[&str])->i32 {
    let scenario=if argc>3 {argv[3]} else {"context"};
    assert!(agent_create(&AgentCreateArgs::new(2,3,0))>0);
    emit("agent_ready",json!({"goal":if scenario=="context" {"Context：回绕、回溯、追加与清空"} else {"事件：心跳修改、停止与 IPC 唤醒"},"scenario":scenario,"context":context_value()}));
    if scenario=="context" {
        for _ in 0..6 {execute("get_system_status");pause(250);}
        assert_eq!(unsafe {mapped_info()}.evictions,3);
        assert_eq!(context_rollback(1),0);
        emit("context_changed",json!({"operation":"rollback_keep_1","context":context_value()}));pause(500);
        execute("read_context");pause(500);
        assert_eq!(context_clear(),0);
        emit("context_changed",json!({"operation":"clear","context":context_value()}));
    } else {
        for period in [60usize,20] {
            assert_eq!(agent_heartbeat_set(period),0);
            for _ in 0..3 {
                let start=get_time();let event=agent_wait(500);
                assert_ne!(event as u64&EVENT_HEARTBEAT,0);
                emit("loop_event",json!({"kind":"heartbeat","period_ms":period,"observed_ms":get_time()-start,"context":context_value()}));
            }
        }
        agent_heartbeat_stop();assert_eq!(agent_wait(40),EVENT_TIMEOUT as isize);
        emit("loop_event",json!({"kind":"heartbeat_stopped","timeout_ms":40,"context":context_value()}));
        let pid=getpid() as u64;let instance=unsafe {mapped_info()}.instance;
        let child=fork();
        if child==0 {
            send(pid,instance,MSG_HELLO);let message=receive();assert_eq!(message.kind,MSG_EVENT);
            emit("loop_event",json!({"kind":"ipc_wakeup","source_pid":message.source_pid,"context":context_value()}));
            send(pid,instance,MSG_EVENT);agent_finish();exit(0);
        }
        let hello=receive();pause(100);
        let req=ToolRequest::new("query_process").unwrap().number("pid",child as i64).unwrap();
        let mut out=ToolResponse::default();assert_eq!(tool_call_recorded(&req,&mut out),0);
        let value:Value=serde_json::from_str(out.result().unwrap()).unwrap();
        assert_eq!(value["processes"][0]["state"],"blocked");
        emit("loop_event",json!({"kind":"blocked_observed","observation":value,"context":context_value()}));
        send(child as u64,hello.source_instance,MSG_EVENT);receive();
        let mut code=0;waitpid(child as usize,&mut code);assert_eq!(code,0);
    }
    emit("task_final",json!({"text":"机制场景验证完成。各步观察保存在调用轨迹；该场景不调用模型。","scenario":scenario,"complete":true,"evidence":[],"context":context_value()}));
    agent_finish();0
}
