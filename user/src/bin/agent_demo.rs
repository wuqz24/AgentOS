#![no_std]
#![no_main]
extern crate user_lib;
use user_lib::*;
#[unsafe(no_mangle)]
pub fn main() -> i32 {
    assert!(agent_create(&AgentCreateArgs::new(1, 4, 0)) > 0);
    for round in 0..8 {
        let name = ["get_system_status", "query_process", "read_context"][round % 3];
        let mut req = ToolRequest::new(name).unwrap();
        req.request_id = round as u64;
        let mut out = ToolResponse::default();
        assert_eq!(tool_call_recorded(&req, &mut out), 0);
        println!("[AgentAdmin] round={} tool={} result={}", round, name, out.result().unwrap());
    }
    let mut nodes = [ContextNode::default(); 4];
    assert_eq!(unsafe { context_snapshot(&mut nodes) }, 4);
    assert_eq!(nodes[0].seq, 4);
    assert_eq!(nodes[3].seq, 7);
    assert_eq!(unsafe { mapped_info() }.evictions, 4);
    assert_eq!(context_rollback(2), 0);
    assert_eq!(context_query(&mut nodes), 2);
    assert_eq!(context_clear(), 0);
    assert_eq!(agent_finish(), 0);
    println!("AgentOS demo passed.");
    0
}
