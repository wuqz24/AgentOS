#![no_std]
#![no_main]
extern crate user_lib;
use user_lib::*;
#[unsafe(no_mangle)]
pub fn main() -> i32 {
    assert!(agent_create(&AgentCreateArgs::new(2, 8, 0)) > 0);
    let mut out = ToolResponse::default();
    assert_eq!(tool_call(&ToolRequest::new("unknown").unwrap(), &mut out), ENOSYS);
    println!("AgentProbe: unknown-tool error path passed, code={}", out.status);
    assert_eq!(agent_finish(), 0);
    0
}
