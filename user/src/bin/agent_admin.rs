#![no_std]
#![no_main]
extern crate user_lib;
use user_lib::*;
#[unsafe(no_mangle)]
pub fn main(argc: usize, argv: &[&str]) -> i32 {
    if argc < 5 {
        println!("Start agent_admin through agent_bridge (host/server.py).");
        return 1;
    }
    let parent_pid = argv[1].parse::<u64>().unwrap_or(0);
    let parent_instance = argv[2].parse::<u64>().unwrap_or(0);
    let max_rounds = argv[4].parse::<usize>().unwrap_or(8).clamp(1,16);
    assert!(agent_create(&AgentCreateArgs::new(1, 16, 0)) > 0);
    agent_runtime::run(argv[3], parent_pid, parent_instance, max_rounds)
}
