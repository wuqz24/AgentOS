#![no_std]
#![no_main]
extern crate user_lib;
extern crate alloc;
use user_lib::*;
use alloc::vec;

fn call(name: &str) -> ToolResponse {
    let req = ToolRequest::new(name).unwrap();
    let mut out = ToolResponse::default();
    assert_eq!(tool_call(&req, &mut out), 0);
    out
}
fn free_pages() -> u64 {
    let out = call("get_system_status");
    let value: serde_json::Value = serde_json::from_str(out.result().unwrap()).unwrap();
    value["physical_pages_free"].as_u64().unwrap()
}

#[unsafe(no_mangle)]
pub fn main(argc: usize, argv: &[&str]) -> i32 {
    let mut info = AgentInfo::default();
    if argc > 1 && argv[1] == "exec-check" {
        assert_eq!(agent_info(&mut info), EPERM);
        println!("[PASS] A06 exec cleared Agent state");
        return 0;
    }
    assert_eq!(agent_info(&mut info), EPERM);
    assert_eq!(agent_create(&AgentCreateArgs::new(1, 0, 0)), EINVAL);
    assert_eq!(agent_create(&AgentCreateArgs::new(1, MAX_CONTEXT_NODES as u64 + 1, 0)), EINVAL);
    assert_eq!(agent_create(&AgentCreateArgs::new(1, 4, 1)), EINVAL);
    assert_eq!(agent_create(&AgentCreateArgs::new(1, 4, 0)), (CONTEXT_BASE + PAGE_SIZE) as isize);
    assert_eq!(agent_create(&AgentCreateArgs::new(1, 4, 0)), EEXIST);
    assert_eq!(agent_info(&mut info), 0);
    assert_eq!(info.context_bytes, 8192);
    assert_eq!(info.node_size, 320);
    println!("[PASS] A01 A02 registration and bounded mapping");

    let mut desc = [ToolDescriptor::default(); 10];
    assert!(tool_list(&mut desc) >= 3);
    for name in ["get_system_status", "query_process", "read_context"] {
        let r = call(name);
        assert!(serde_json::from_str::<serde_json::Value>(r.result().unwrap()).is_ok());
    }
    let mut out = ToolResponse::default();
    assert_eq!(tool_call(&ToolRequest::new("missing_tool").unwrap(), &mut out), ENOSYS);
    assert_eq!(out.status, ENOSYS as i64);
    let bad = ToolRequest::new("query_process").unwrap().text("pid", "bad").unwrap();
    assert_eq!(tool_call(&bad, &mut out), EINVAL);
    let bad = ToolRequest::new("query_process").unwrap().number("unknown", 1).unwrap();
    assert_eq!(tool_call(&bad, &mut out), EINVAL);
    let bad = ToolRequest::new("query_process").unwrap().number("pid", 1).unwrap().number("pid", 2).unwrap();
    assert_eq!(tool_call(&bad, &mut out), EINVAL);
    let bad = ToolRequest::new("query_process").unwrap().number("pid", 999999).unwrap();
    assert_eq!(tool_call(&bad, &mut out), ESRCH);
    println!("[PASS] T01 T02 T03 T06 discovery, typed tools and errors");

    let req = ToolRequest::new("get_system_status").unwrap();
    let req_ptr = &req as *const ToolRequest as usize;
    let out_ptr = &mut out as *mut ToolResponse as usize;
    assert_eq!(unsafe { agent_raw(SYS_TOOL_CALL, [0, out_ptr, 0]) }, EFAULT);
    assert_eq!(unsafe { agent_raw(SYS_TOOL_CALL, [req_ptr | (1usize << 39), out_ptr, 0]) }, EFAULT);
    assert_eq!(unsafe { agent_raw(SYS_TOOL_CALL, [req_ptr, CONTEXT_BASE, 0]) }, EFAULT);
    let mut bytes = vec![0u8; PAGE_SIZE * 3];
    let aligned = (bytes.as_mut_ptr() as usize + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    let crossing = aligned + PAGE_SIZE - 24;
    unsafe { core::ptr::copy_nonoverlapping(req_ptr as *const u8, crossing as *mut u8, core::mem::size_of::<ToolRequest>()); }
    assert_eq!(unsafe { agent_raw(SYS_TOOL_CALL, [crossing, out_ptr, 0]) }, 0);
    let end = info.context_base as usize + PAGE_SIZE - 16;
    unsafe { core::ptr::write_bytes(end as *mut u8, 0xab, 16); }
    assert_eq!(unsafe { agent_raw(SYS_TOOL_CALL, [req_ptr, end, 0]) }, EFAULT);
    for i in 0..16 { assert_eq!(unsafe { ((end+i) as *const u8).read_volatile() }, 0xab); }
    println!("[PASS] A04 T04 canonical, cross-page and atomic output validation");

    for round in 0..8 {
        let mut req = ToolRequest::new("get_system_status").unwrap();
        req.request_id = round;
        assert_eq!(tool_call_recorded(&req, &mut out), 0);
    }
    let mut direct = [ContextNode::default(); 4];
    let mut copied = [ContextNode::default(); 4];
    assert_eq!(unsafe { context_snapshot(&mut direct) }, 4);
    assert_eq!(context_query(&mut copied), 4);
    for i in 0..4 { assert_eq!(direct[i].seq, i as u64 + 4); assert_eq!(direct[i].seq, copied[i].seq); }
    assert_eq!(unsafe { mapped_info() }.evictions, 4);
    assert_eq!(context_rollback(5), EINVAL);
    assert_eq!(context_rollback(2), 0);
    assert_eq!(context_query(&mut copied), 2);
    assert_eq!(copied[0].seq, 4);
    assert_eq!(copied[1].seq, 5);
    assert_eq!(tool_call_recorded(&req, &mut out), 0);
    assert_eq!(context_query(&mut copied), 3);
    assert_eq!(copied[2].seq, 8);
    println!("[PASS] C01 C02 C03 C04 history, FIFO, direct reads and rollback");

    let parent_instance = unsafe { mapped_info() }.instance;
    let child = fork();
    assert!(child >= 0);
    if child == 0 {
        assert_ne!(unsafe { mapped_info() }.instance, parent_instance);
        assert_eq!(context_clear(), 0);
        assert_eq!(agent_finish(), 0);
        exit(0);
    }
    let mut code = 0;
    assert_eq!(waitpid(child as usize, &mut code), child);
    assert_eq!(code, 0);
    assert_eq!(unsafe { mapped_info() }.context_len, 3);
    println!("[PASS] A06 fork independent Context and instance");

    let child = fork();
    if child == 0 {
        let name = "agent_tests\0";
        let mode = "exec-check\0";
        exec(name, &[name.as_ptr(), mode.as_ptr(), core::ptr::null()]);
        exit(99);
    }
    waitpid(child as usize, &mut code);
    assert_eq!(code, 0);
    let child = fork();
    if child == 0 {
        unsafe { (CONTEXT_BASE as *mut u64).write_volatile(0); }
        exit(99);
    }
    waitpid(child as usize, &mut code);
    assert_eq!(code, -11);
    println!("[PASS] A04 control page rejects direct user writes");

    assert_eq!(context_clear(), 0);
    let before = free_pages();
    if unsafe { agent_raw(4095, [usize::MAX, 0, 0]) } == 0 {
        for fail_after in 0..4 {
            assert_eq!(agent_finish(), 0);
            assert_eq!(unsafe { agent_raw(4095, [fail_after, 0, 0]) }, 0);
            assert_eq!(agent_create(&AgentCreateArgs::new(1, 4, 0)), ENOMEM);
            assert_eq!(unsafe { agent_raw(4095, [usize::MAX, 0, 0]) }, 0);
            assert!(agent_create(&AgentCreateArgs::new(1, 4, 0)) > 0);
            assert_eq!(free_pages(), before);
        }
        println!("[PASS] A05 allocation failures rolled back at every physical allocation");
        assert_eq!(agent_finish(),0);
        assert_eq!(unsafe {agent_raw(4093,[1,0,0])},0);
        assert_eq!(agent_create(&AgentCreateArgs::new(1,4,0)),EEXIST);
        assert_eq!(unsafe {agent_raw(4093,[0,0,0])},0);
        assert!(agent_create(&AgentCreateArgs::new(1,4,0))>0);
        assert_eq!(free_pages(),before);
        println!("[PASS] A05 existing mapping collision preserved and safely rejected");
    } else {
        assert_eq!(unsafe {agent_raw(4093,[1,0,0])},ENOSYS);
        assert_eq!(unsafe {agent_raw(4094,[0;3])},ENOSYS);
        println!("[PASS] production kernel excludes all test injection syscalls");
        println!("[SKIP] A05 fault injection disabled in production kernel");
    }
    let mut node = ContextNode::default();
    put_text(&mut node.tool, "stress").unwrap();
    for _ in 0..10000 { assert_eq!(context_push(&node), 0); }
    assert_eq!(free_pages(), before);
    assert_eq!(unsafe { mapped_info() }.context_len, 4);
    println!("[PASS] C05 10000 appends with constant physical pages");

    for _ in 0..100 {
        assert_eq!(agent_finish(), 0);
        assert_eq!(agent_info(&mut info), 0);
        assert_eq!(info.loop_state, DONE);
        assert_eq!(info.context_bytes, 0);
        assert!(agent_create(&AgentCreateArgs::new(1, 4, 0)) > 0);
    }
    assert_eq!(free_pages(), before);
    for _ in 0..100 {
        let child=fork();
        assert!(child>=0);
        if child==0 {assert_eq!(context_push(&node),0);exit(0);}
        waitpid(child as usize,&mut code);assert_eq!(code,0);
    }
    assert_eq!(free_pages(), before);
    println!("[PASS] A07 100 Agent fork/exit/reap cycles with constant physical pages");
    assert_eq!(agent_finish(), 0);
    println!("[PASS] A07 100 create/finish cycles with reclaimed page tables");
    for app in ["hello_world\0", "filetest_simple\0", "forktest_simple\0", "sleep_simple\0", "condsync_condvar\0"] {
        let child = fork();
        if child == 0 { exec(app, &[app.as_ptr(), core::ptr::null()]); exit(99); }
        waitpid(child as usize, &mut code);
        assert_eq!(code, 0);
    }
    println!("[PASS] A03 R01 ordinary process, file, fork, sleep and condition-variable regressions");
    println!("[PASS] agent_tests");
    0
}
