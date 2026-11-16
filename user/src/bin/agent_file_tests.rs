#![no_std]
#![no_main]
extern crate user_lib;
extern crate alloc;
use user_lib::*;
use alloc::{format,vec::Vec,string::String};
use serde_json::Value;

fn execute(req: ToolRequest) -> Value {
    let mut out=ToolResponse::default();
    assert_eq!(tool_call(&req,&mut out),0);
    serde_json::from_str(out.result().unwrap()).unwrap()
}
fn attr(path: &str,key: &str,value: &str) -> Value {
    execute(ToolRequest::new("set_file_attr").unwrap().text("path",path).unwrap().text("key",key).unwrap().text("value",value).unwrap())
}
fn summary(path: &str) -> Value {execute(ToolRequest::new("read_file_summary").unwrap().text("path",path).unwrap())}
fn query(filter: &str,keyword: &str,mode: &str,cursor: i64) -> Value {
    execute(ToolRequest::new("query_files").unwrap().text("filter",filter).unwrap().text("keyword",keyword).unwrap().text("mode",mode).unwrap().number("limit",4).unwrap().number("cursor",cursor).unwrap())
}
fn create(path: &str,body: &str) {
    let path=format!("{}\0",path);
    let fd=open(&path,OpenFlags::CREATE|OpenFlags::WRONLY);
    assert!(fd>=0);assert_eq!(write(fd as usize,body.as_bytes()),body.len() as isize);close(fd as usize);
}
fn paths(filter: &str,mode: &str) -> Vec<String> {
    let mut cursor=0;let mut result=Vec::new();
    loop {
        let value=query(filter,"",mode,cursor);
        for row in value["files"].as_array().unwrap() {result.push(String::from(row["path"].as_str().unwrap()));}
        let Some(next)=value["next_cursor"].as_i64() else {break;};
        assert!(next>cursor);cursor=next;
    }
    result
}
#[unsafe(no_mangle)]
pub fn main(argc: usize,argv: &[&str]) -> i32 {
    assert!(agent_create(&AgentCreateArgs::new(2,8,0))>0);
    if argc>1 {
        let info=execute(ToolRequest::new("file_index_status").unwrap());
        assert_eq!(paths("type=diagnostic,owner=42","index"),[String::from("attr-proof")]);
        assert!(summary("attr-proof")["summary"].as_str().unwrap().contains("更新"));
        if argv[1]=="corrupt" {assert_eq!(unsafe {agent_raw(4094,[0;3])},0);}
        if argv[1]=="recover" {
            assert_eq!(info["corrupt_tail"],true);
            attr("attr-proof","recovered","yes");
            let after=execute(ToolRequest::new("file_index_status").unwrap());
            assert_eq!(after["corrupt_tail"],false);
            assert!(after["epoch"].as_u64()>info["epoch"].as_u64());
        }
        println!("[PASS] file_persistence {}",argv[1]);
        return 0;
    }
    create("attr-proof","初始内容 original");
    let first=attr("attr-proof","type","log");
    attr("attr-proof","owner","42");
    let info=execute(ToolRequest::new("get_file_attrs").unwrap().text("path","attr-proof").unwrap());
    assert_eq!(info["attrs"]["owner"],"42");
    let a=summary("attr-proof");assert_eq!(a["cache_hit"],false);
    let b=summary("attr-proof");assert_eq!(b["cache_hit"],true);assert_eq!(a["summary"],b["summary"]);
    create("attr-proof","更新后的内容 new evidence");
    let changed=summary("attr-proof");assert_eq!(changed["cache_hit"],false);assert!(changed["version"].as_u64()>a["version"].as_u64());
    assert!(changed["summary"].as_str().unwrap().contains("更新"));
    let updated=attr("attr-proof","type","diagnostic");assert_eq!(first["id"],updated["id"]);
    assert_eq!(query("type=log,owner=42","","index",0)["matched"],0);
    assert_eq!(query("type=diagnostic,owner=42","更新","index",0)["matched"],1);
    assert_eq!(query("type=diagnostic","missing","index",0)["matched"],0);
    attr("attr-proof","temp","remove");
    execute(ToolRequest::new("delete_file_attr").unwrap().text("path","attr-proof").unwrap().text("key","temp").unwrap());
    assert_eq!(query("temp=remove","","index",0)["matched"],0);
    println!("[PASS] F01 F04 attributes overwrite delete, summary keyword and version cache invalidation");
    for i in 0..32 {
        let path=format!("attr-{:02}",i);create(&path,"test file for AND index");
        attr(&path,"type","test");attr(&path,"owner",if i%2==0 {"even"} else {"odd"});
        attr(&path,"tag",if i%8==0 {"rare"} else {"common"});
    }
    for filter in ["type=test","type=test,owner=even","owner=even,tag=rare","owner=odd,tag=rare","type=absent"] {
        assert_eq!(paths(filter,"index"),paths(filter,"scan"));
    }
    let indexed=query("type=test,tag=rare","","index",0);
    let scanned=query("type=test,tag=rare","","scan",0);
    assert!(indexed["checked"].as_u64()<scanned["checked"].as_u64());
    println!("[PASS] F02 F03 AND results, pagination, empty results and index-scan equality");
    let mut out=ToolResponse::default();
    for req in [ToolRequest::new("set_file_attr").unwrap().text("path",".agent-meta-a").unwrap().text("key","type").unwrap().text("value","bad").unwrap(),
        ToolRequest::new("query_files").unwrap().text("filter","bad").unwrap(),
        ToolRequest::new("set_file_attr").unwrap().text("path","attr-proof").unwrap().text("key","bad=key").unwrap().text("value","bad").unwrap()] {
        assert!(tool_call(&req,&mut out)<0);
    }
    assert!(open(".agent-meta-a\0",OpenFlags::RDONLY)<0);
    assert!(open(".agent-meta-b\0",OpenFlags::CREATE|OpenFlags::WRONLY)<0);
    for i in 0..6 {attr("attr-proof",&format!("extra{}",i),"value");}
    let extra=ToolRequest::new("set_file_attr").unwrap().text("path","attr-proof").unwrap().text("key","ninth").unwrap().text("value","value").unwrap();
    assert_eq!(tool_call(&extra,&mut out),ENOSPC);
    for i in 0..6 {execute(ToolRequest::new("delete_file_attr").unwrap().text("path","attr-proof").unwrap().text("key",&format!("extra{}",i)).unwrap());}
    println!("[PASS] F05 protected journal and invalid attributes");
    println!("[PASS] agent_file_tests");
    0
}
