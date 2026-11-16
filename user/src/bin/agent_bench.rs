#![no_std]
#![no_main]
extern crate user_lib;
extern crate alloc;
use user_lib::*;
use alloc::{format,vec,vec::Vec};
use core::hint::black_box;
use serde_json::{Value,json};

fn tool(req: ToolRequest) -> Value {
    let mut out=ToolResponse::default();assert_eq!(tool_call(&req,&mut out),0);
    serde_json::from_str(out.result().unwrap()).unwrap()
}
fn record(value: Value) {println!("@BENCH {}",value);}
fn process() -> Value {tool(ToolRequest::new("query_process").unwrap().number("pid",getpid() as i64).unwrap())["processes"][0].clone()}
fn context_bench() {
    for count in [4usize,16,64] {
        agent_finish();assert!(agent_create(&AgentCreateArgs::new(2,count as u64,0))>0);
        for i in 0..count {assert_eq!(context_push(&ContextNode {request_id:i as u64,..ContextNode::default()}),0);}
        let mut direct=vec![ContextNode::default();count];let mut copied=direct.clone();
        assert_eq!(unsafe {context_snapshot(&mut direct)},count);assert_eq!(context_query(&mut copied),count as isize);
        for (a,b) in direct.iter().zip(&copied) {assert_eq!(a.seq,b.seq);assert_eq!(a.request_id,b.request_id);}
        for sample in 0..11 {
            for mode in if sample%2==0 {["direct","syscall"]} else {["syscall","direct"]} {
                let mut checksum=0u64;let start=agent_clock();
                for _ in 0..1000 {
                    if mode=="direct" {black_box(unsafe {context_snapshot(&mut direct)});} else {assert_eq!(context_query(&mut direct),count as isize);}
                    for node in &direct {checksum=checksum.wrapping_add(black_box(node.seq)^black_box(node.request_id));}
                    black_box(&direct);
                }
                let ticks=agent_clock()-start;black_box(checksum);
                record(json!({"experiment":"context","mode":mode,"nodes":count,"node_bytes":320,"iterations":1000,"sample":sample,"ticks":ticks}));
            }
        }
    }
}
fn attr(path: &str,key: &str,value: &str) {tool(ToolRequest::new("set_file_attr").unwrap().text("path",path).unwrap().text("key",key).unwrap().text("value",value).unwrap());}
fn query(filter: &str,mode: &str,cursor: i64) -> Value {tool(ToolRequest::new("query_files").unwrap().text("filter",filter).unwrap().text("mode",mode).unwrap().number("limit",4).unwrap().number("cursor",cursor).unwrap())}
fn all_ids(filter: &str,mode: &str) -> Vec<u64> {
    let mut ids=Vec::new();let mut cursor=0;
    loop {let page=query(filter,mode,cursor);for row in page["files"].as_array().unwrap() {ids.push(row["id"].as_u64().unwrap());}
        let Some(next)=page["next_cursor"].as_i64() else {break;};assert!(next>cursor);cursor=next;}
    ids
}
fn index_bench() {
    let mut created=0;
    for count in [32,128,512] {
        let start=agent_clock();
        for i in created..count {
            let path=format!("bench-{:03}",i);let file=format!("{}\0",path);
            let fd=open(&file,OpenFlags::CREATE|OpenFlags::WRONLY);assert!(fd>=0);
            write(fd as usize,b"Index benchmark dataset: real easy-fs file.\n");close(fd as usize);
            attr(&path,"bench","yes");attr(&path,"tag",if i%32==0 {"rare"} else {"common"});attr(&path,"owner",if i%2==0 {"even"} else {"odd"});
        }
        record(json!({"experiment":"index_update","files":count,"added":count-created,"ticks":agent_clock()-start}));created=count;
        for (case,filter) in [("selective","bench=yes,tag=rare"),("broad","bench=yes"),("empty","bench=yes,tag=missing"),("and","bench=yes,tag=common,owner=odd")] {
            let indexed=all_ids(filter,"index");let scanned=all_ids(filter,"scan");assert_eq!(indexed,scanned);
            for sample in 0..11 {
                for mode in if sample%2==0 {["index","scan"]} else {["scan","index"]} {
                    let result=query(filter,mode,0);
                    record(json!({"experiment":"index","files":count,"mode":mode,"case":case,"sample":sample,
                        "ticks":result["elapsed_ticks"],"matched":result["matched"],"checked":result["checked"],
                        "index_pairs":result["index_pairs"],"index_allocated_bytes":result["index_allocated_bytes"],"index_payload_bytes":result["index_payload_bytes"],"cache":"warm summary cache; no query-result cache"}));
                }
            }
        }
    }
}
fn send(pid:u64,instance:u64,value:u64) {
    let mut m=Message{target_pid:pid,target_instance:instance,kind:MSG_EVENT,len:8,..Message::default()};
    m.data[..8].copy_from_slice(&value.to_le_bytes());assert_eq!(agent_send(&m),0);
}
fn receive() -> Message {loop {let mut m=Message::default();if agent_receive(&mut m)==0{return m;}assert!(agent_wait(2000)>0);}}
fn wait_bench() {
    agent_finish();assert!(agent_create(&AgentCreateArgs::new(2,4,0))>0);
    let me=getpid() as u64;let instance=unsafe {mapped_info()}.instance;
    for sample in 0..11 {
        for mode in if sample%2==0 {["wait","poll"]} else {["poll","wait"]} {
            let child=fork();assert!(child>=0);
            if child==0 {
                let before=process();send(me,instance,0);
                let start=agent_clock();let mut calls=0u64;let mut waits=0u64;
                let sent=loop {
                    let mut m=Message::default();calls+=1;
                    if agent_receive(&mut m)==0 {break u64::from_le_bytes(m.data[..8].try_into().unwrap());}
                    if mode=="wait" {waits+=1;assert!(agent_wait(2000)>0);}
                };
                let received=agent_clock();let after=process();
                record(json!({"experiment":"wait","mode":mode,"sample":sample,"interval_ms":50,
                    "elapsed_ticks":received-start,"runtime_ticks":after["runtime_ticks"].as_u64().unwrap()-before["runtime_ticks"].as_u64().unwrap(),
                    "dispatches":after["dispatches"].as_u64().unwrap()-before["dispatches"].as_u64().unwrap(),
                    "receive_calls":calls,"wait_calls":waits,"latency_ticks":received-sent}));
                exit(0);
            }
            let hello=receive();sleep(50);send(child as u64,hello.source_instance,agent_clock());
            let mut code=0;waitpid(child as usize,&mut code);assert_eq!(code,0);
        }
    }
}
#[unsafe(no_mangle)]
pub fn main(argc:usize,argv:&[&str]) -> i32 {
    assert!(agent_create(&AgentCreateArgs::new(2,8,0))>0);
    let status=tool(ToolRequest::new("get_system_status").unwrap());
    record(json!({"experiment":"environment","clock_hz":status["clock_hz"],"samples":11,"single_cpu":true}));
    let mode=if argc>1 {argv[1]} else {"all"};
    if mode=="all" || mode=="context" {context_bench();}
    if mode=="all" || mode=="index" {index_bench();}
    if mode=="all" || mode=="wait" {wait_bench();}
    println!("[PASS] agent_bench");0
}
