use super::*;
use crate::task::{ProcessControlBlockInner, block_current_task, for_each_process,
                  pid2process, schedule, wakeup_task};

fn ready_events(inner: &ProcessControlBlockInner) -> u64 {
    let a = &inner.agent;
    let heartbeat = if a.info.heartbeat_ms > 0 { EVENT_HEARTBEAT } else { 0 };
    a.info.pending_events & (a.watched | heartbeat | EVENT_TIMEOUT | EVENT_CANCEL)
}

fn wake_if_ready(inner: &mut ProcessControlBlockInner) {
    if inner.agent.info.loop_state == WAITING && ready_events(inner) != 0 {
        inner.agent.info.loop_state = ACTIVE;
        inner.agent.info.wakeups += 1;
        inner.agent.wait_deadline = 0;
        if let Some(Some(task)) = inner.tasks.first() { wakeup_task(task.clone()); }
    }
}

/// Deadlines live in PCB state, so resetting/stopping an Agent leaves no stale
/// callback or strong task reference in a second timer queue.
pub fn poll_events() {
    let now = get_time_ms() as u64;
    for_each_process(|process| {
        let mut inner = process.inner_exclusive_access();
        if !inner.agent.live() { return; }
        let mut changed = false;
        let a = &mut inner.agent;
        if a.heartbeat_due != 0 && now >= a.heartbeat_due {
            a.info.pending_events |= EVENT_HEARTBEAT;
            let elapsed = (now - a.heartbeat_due) / a.info.heartbeat_ms + 1;
            a.heartbeat_due += elapsed * a.info.heartbeat_ms;
            changed = true;
        }
        if a.info.loop_state == WAITING && a.wait_deadline != 0 && now >= a.wait_deadline {
            a.info.pending_events |= EVENT_TIMEOUT;
            changed = true;
        }
        if changed {
            wake_if_ready(&mut inner);
            inner.agent.info.generation += 2;
            sync_control(&inner.memory_set, &inner.agent);
        }
    });
}

pub fn watch(mask: usize) -> isize { update_watch(mask as u64, true) }
pub fn unwatch(mask: usize) -> isize { update_watch(mask as u64, false) }
fn update_watch(mask: u64, add: bool) -> isize {
    if mask == 0 || mask & !ALL_EVENTS != 0 { return EINVAL; }
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    if !inner.agent.live() { return EPERM; }
    if add { inner.agent.watched |= mask; } else { inner.agent.watched &= !mask; }
    inner.agent.info.generation += 2;
    sync_control(&inner.memory_set, &inner.agent);
    0
}

pub fn wait(timeout_ms: usize) -> isize {
    if timeout_ms > 300_000 { return EINVAL; }
    let process = current_process();
    let deadline = if timeout_ms == 0 { 0 } else { get_time_ms() as u64 + timeout_ms as u64 };
    loop {
        let mut inner = process.inner_exclusive_access();
        if !inner.agent.live() { return EPERM; }
        if !inner.agent.inbox.is_empty() { inner.agent.info.pending_events |= EVENT_MESSAGE; }
        if deadline != 0 && get_time_ms() as u64 >= deadline { inner.agent.info.pending_events |= EVENT_TIMEOUT; }
        let ready = ready_events(&inner);
        if ready != 0 {
            inner.agent.info.pending_events &= !ready;
            inner.agent.info.loop_state = ACTIVE;
            inner.agent.wait_deadline = 0;
            inner.agent.info.generation += 2;
            sync_control(&inner.memory_set, &inner.agent);
            return ready as isize;
        }
        inner.agent.wait_deadline = deadline;
        inner.agent.info.loop_state = WAITING;
        inner.agent.info.generation += 2;
        sync_control(&inner.memory_set, &inner.agent);
        // PCB guard masks interrupts across pending-event check and enrollment.
        let cx = block_current_task();
        drop(inner);
        schedule(cx);
    }
}

pub fn send(ptr: *const Message) -> isize {
    let token = current_user_token();
    let message = match read_user(token, ptr) { Ok(m) => m, Err(e) => return e };
    send_message(message)
}

pub(super) fn send_message(mut message: Message) -> isize {
    if message.len as usize > MESSAGE_BYTES || message.kind == 0 || message.kind > 32 { return EINVAL; }
    let source = current_process();
    let (instance, domain) = {
        let inner = source.inner_exclusive_access();
        if !inner.agent.live() { return EPERM; }
        (inner.agent.info.instance, inner.agent.domain)
    };
    let Some(target) = pid2process(message.target_pid as usize) else { return ESRCH; };
    let mut inner = target.inner_exclusive_access();
    if !inner.agent.live() { return ESRCH; }
    if inner.agent.info.instance != message.target_instance { return ESTALE; }
    if inner.agent.domain != domain { return EPERM; }
    if inner.agent.inbox.len() >= MAX_MESSAGES { return EAGAIN; }
    if inner.agent.inbox.try_reserve(1).is_err() { return ENOMEM; }
    message.source_pid = source.getpid() as u64;
    message.source_instance = instance;
    let cancelled = message.kind == MSG_CANCEL;
    inner.agent.inbox.push_back(message);
    inner.agent.info.inbox_len = inner.agent.inbox.len() as u64;
    inner.agent.info.pending_events |= EVENT_MESSAGE;
    if cancelled { inner.agent.info.pending_events |= EVENT_CANCEL; }
    wake_if_ready(&mut inner);
    inner.agent.info.generation += 2;
    sync_control(&inner.memory_set, &inner.agent);
    0
}

pub fn receive(out: *mut Message) -> isize {
    let token = current_user_token();
    if validate_user_buffer(token, out as usize, core::mem::size_of::<Message>(), true).is_err() { return EFAULT; }
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    if !inner.agent.live() { return EPERM; }
    let Some(message) = inner.agent.inbox.front() else { return EAGAIN; };
    if let Err(e) = write_user(token, out, message) { return e; }
    inner.agent.inbox.pop_front();
    inner.agent.info.inbox_len = inner.agent.inbox.len() as u64;
    if inner.agent.inbox.is_empty() { inner.agent.info.pending_events &= !EVENT_MESSAGE; }
    inner.agent.info.generation += 2;
    sync_control(&inner.memory_set, &inner.agent);
    0
}
