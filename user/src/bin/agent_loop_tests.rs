#![no_std]
#![no_main]
extern crate user_lib;
use user_lib::*;
use serde_json::Value;

fn message(pid: u64, instance: u64, kind: u64, seq: u64) -> Message {
    let mut msg = Message { target_pid:pid,target_instance:instance,kind,len:8,..Message::default() };
    msg.data[..8].copy_from_slice(&seq.to_le_bytes());
    msg
}
fn recv() -> Message {
    let mut m = Message::default();
    for _ in 0..200 {
        if agent_receive(&mut m) == 0 { return m; }
        let events = agent_wait(1000);
        assert!(events > 0);
        assert_eq!(events as u64 & EVENT_TIMEOUT, 0);
    }
    panic!("message timeout");
}
fn process(pid: isize) -> Value {
    let req = ToolRequest::new("query_process").unwrap().number("pid", pid as i64).unwrap();
    let mut out = ToolResponse::default();
    assert_eq!(tool_call(&req, &mut out), 0);
    let v: Value = serde_json::from_str(out.result().unwrap()).unwrap();
    v["processes"][0].clone()
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let mut channel=[0usize;2];let mut ack_channel=[0usize;2];
    assert_eq!(pipe(&mut channel),0);assert_eq!(pipe(&mut ack_channel),0);
    let outsider=fork();
    if outsider==0 {
        assert!(agent_create(&AgentCreateArgs::new(2,4,0))>0);
        let id=unsafe {mapped_info()}.instance.to_le_bytes();
        assert_eq!(write(channel[1],&id),8);
        let mut ack=[0u8;1];assert_eq!(read(ack_channel[0],&mut ack),1);
        exit(0);
    }
    let mut id=[0u8;8];assert_eq!(read(channel[0],&mut id),8);
    assert!(agent_create(&AgentCreateArgs::new(1,4,0))>0);
    assert_eq!(agent_send(&message(outsider as u64,u64::from_le_bytes(id),MSG_EVENT,0)),EPERM);
    assert_eq!(write(ack_channel[1],&[1]),1);
    let mut code=0;waitpid(outsider as usize,&mut code);assert_eq!(code,0);
    for fd in [channel[0],channel[1],ack_channel[0],ack_channel[1]] {close(fd);}
    assert_eq!(agent_finish(),0);
    println!("[PASS] T05 cross-domain IPC permission denied");
    assert!(agent_create(&AgentCreateArgs::new(1, 8, 0)) > 0);
    assert_eq!(agent_watch(0), EINVAL);
    assert_eq!(agent_watch(0x100), EINVAL);
    assert_eq!(agent_heartbeat_set(1), EINVAL);
    assert_eq!(agent_heartbeat_set(30), 0);
    let begin = get_time();
    for _ in 0..3 { assert_ne!(agent_wait(300) as u64 & EVENT_HEARTBEAT, 0); }
    assert!(get_time() - begin >= 70);
    assert_eq!(agent_heartbeat_set(10), 0);
    assert_ne!(agent_wait(300) as u64 & EVENT_HEARTBEAT, 0);
    assert_eq!(agent_heartbeat_stop(), 0);
    assert_eq!(agent_wait(40), EVENT_TIMEOUT as isize);
    println!("[PASS] L01 L02 heartbeat, dynamic period, stop and timeout");

    let me = getpid() as u64;
    let instance = unsafe { mapped_info() }.instance;
    for seq in 0..MAX_MESSAGES { assert_eq!(agent_send(&message(me, instance, MSG_EVENT, seq as u64)), 0); }
    assert_eq!(agent_send(&message(me, instance, MSG_EVENT, 9)), EAGAIN);
    assert_eq!(agent_send(&message(me, instance+999, MSG_EVENT, 9)), ESTALE);
    for seq in 0..MAX_MESSAGES {
        let m = recv();
        assert_eq!(m.source_pid, me);
        assert_eq!(m.source_instance, instance);
        assert_eq!(u64::from_le_bytes(m.data[..8].try_into().unwrap()), seq as u64);
    }
    assert_eq!(agent_unwatch(EVENT_MESSAGE), 0);
    assert_eq!(agent_send(&message(me, instance, MSG_EVENT, 10)), 0);
    assert_eq!(agent_wait(20), EVENT_TIMEOUT as isize);
    assert_eq!(agent_watch(EVENT_MESSAGE), 0);
    assert_eq!(agent_wait(20), EVENT_MESSAGE as isize);
    recv();
    println!("[PASS] L03 T05 bounded IPC, instance validation and watch/unwatch");

    let child = fork();
    if child == 0 {
        assert_eq!(agent_send(&message(me, instance, MSG_HELLO, 0)), 0);
        let request = recv();
        assert_eq!(agent_send(&message(me, instance, MSG_EVENT, request.kind)), 0);
        assert_eq!(agent_finish(), 0);
        exit(0);
    }
    let hello = recv();
    sleep(30);
    let before = process(child);
    assert_eq!(before["state"].as_str(), Some("blocked"));
    sleep(50);
    let after = process(child);
    assert_eq!(after["state"].as_str(), Some("blocked"));
    assert_eq!(before["runtime_ticks"], after["runtime_ticks"]);
    assert_eq!(before["dispatches"], after["dispatches"]);
    assert_eq!(agent_send(&message(child as u64, hello.source_instance, MSG_CANCEL, 0)), 0);
    let ack = recv();
    assert_eq!(u64::from_le_bytes(ack.data[..8].try_into().unwrap()), MSG_CANCEL);
    let mut code = 0;
    waitpid(child as usize, &mut code);
    assert_eq!(code, 0);
    assert_eq!(agent_send(&message(child as u64, hello.source_instance, MSG_EVENT, 0)), ESRCH);
    println!("[PASS] L03 L05 blocked task consumes no runtime; cancel wakes and exit cleans up");

    let child = fork();
    if child == 0 {
        assert_eq!(agent_heartbeat_set(10), 0);
        assert_eq!(agent_send(&message(me, instance, MSG_HELLO, 0)), 0);
        for seq in 0..1000 {
            let request = recv();
            assert_eq!(u64::from_le_bytes(request.data[..8].try_into().unwrap()), seq);
            assert_eq!(agent_send(&message(me, instance, MSG_EVENT, seq)), 0);
        }
        assert_eq!(agent_finish(), 0);
        exit(0);
    }
    let next = recv();
    if child as u64 == hello.source_pid {
        assert_eq!(agent_send(&message(child as u64, hello.source_instance, MSG_EVENT, 0)), ESTALE);
    }
    assert_eq!(agent_heartbeat_set(10), 0);
    for seq in 0..1000 {
        assert_eq!(agent_send(&message(child as u64, next.source_instance, MSG_EVENT, seq)), 0);
        let answer = recv();
        assert_eq!(u64::from_le_bytes(answer.data[..8].try_into().unwrap()), seq);
    }
    waitpid(child as usize, &mut code);
    assert_eq!(code, 0);
    assert_eq!(agent_heartbeat_stop(), 0);
    assert_eq!(agent_wait(30), EVENT_TIMEOUT as isize);
    println!("[PASS] L04 L06 1000 IPC round trips with heartbeats and PID reuse");
    assert_eq!(agent_finish(), 0);
    println!("[PASS] agent_loop_tests");
    0
}
