use super::__switch;
use super::{ProcessControlBlock, TaskContext, TaskControlBlock};
use super::{TaskStatus, fetch_task};
use crate::sync::UPIntrFreeCell;
use crate::trap::TrapContext;
use alloc::sync::Arc;
use core::arch::asm;
use lazy_static::*;

pub struct Processor {
    current: Option<Arc<TaskControlBlock>>,
    idle_task_cx: TaskContext,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::zero_init(),
        }
    }
    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.as_ref().map(Arc::clone)
    }
}

lazy_static! {
    pub static ref PROCESSOR: UPIntrFreeCell<Processor> =
        unsafe { UPIntrFreeCell::new(Processor::new()) };
}

pub fn run_tasks() {
    loop {
        // Check Agent deadlines at scheduler boundaries. Avoid allocation in a
        // timer ISR that could interrupt a kernel heap allocation.
        crate::agent::poll_events();
        let mut processor = PROCESSOR.exclusive_access();
        if let Some(task) = fetch_task() {
            let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
            // access coming task TCB exclusively
            let next_task_cx_ptr = task.inner.exclusive_session(|task_inner| {
                task_inner.task_status = TaskStatus::Running;
                task_inner.run_since = crate::timer::get_time() as u64;
                task_inner.dispatches += 1;
                &task_inner.task_cx as *const TaskContext
            });
            processor.current = Some(task);
            // release processor manually
            drop(processor);
            unsafe {
                __switch(idle_task_cx_ptr, next_task_cx_ptr);
            }
        } else {
            println!("no tasks available in run_tasks");
        }
    }
}

pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    let task = PROCESSOR.exclusive_access().take_current();
    if let Some(ref task) = task {
        let mut inner = task.inner_exclusive_access();
        inner.run_ticks += (crate::timer::get_time() as u64).saturating_sub(inner.run_since);
        inner.run_since = 0;
    }
    task
}

pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().current()
}

pub fn current_process() -> Arc<ProcessControlBlock> {
    current_task().unwrap().process.upgrade().unwrap()
}

pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    task.get_user_token()
}

pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .get_trap_cx()
}

pub fn current_trap_cx_user_va() -> usize {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .trap_cx_user_va()
}

pub fn current_kstack_top() -> usize {
    if let Some(task) = current_task() {
        task.kstack.get_top()
    } else {
        let mut boot_stack_top;
        unsafe { asm!("la {},boot_stack_top",out(reg) boot_stack_top) };
        boot_stack_top
    }
    // current_task().unwrap().kstack.get_top()
}

pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let idle_task_cx_ptr =
        PROCESSOR.exclusive_session(|processor| processor.get_idle_task_cx_ptr());
    unsafe {
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }
}
