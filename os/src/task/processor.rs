use core::{arch::asm, intrinsics::drop_in_place};

use alloc::sync::Arc;
use lazy_static::lazy_static;

use crate::{
    mm::{
        page_table::{self, PageTable},
        KERNEL_SPACE,
    },
    mutex::SpinNoIrqLock,
    task::{context::check_task_context_in_kernel_stack, current_task, processor},
};

use super::{
    switch::{self, IDLE_TASK},
    Task, TaskStatus,
};

///Processor management structure
pub struct Processor {
    ///The task currently executing on the current processor
    current: Arc<Task>,
}

impl Processor {
    /// Create a empty Processor
    pub fn new() -> Self {
        Self {
            current: IDLE_TASK.clone(),
        }
    }
    pub fn current_task(&self) -> Arc<Task> {
        self.current.clone()
    }
    pub fn switch_to(&mut self, task: Arc<Task>) {
        self.current = task;
    }
}

lazy_static! {
    ///Processor management structure
    pub static ref PROCESSOR: SpinNoIrqLock<Processor> = SpinNoIrqLock::new(Processor::new());
}

pub fn run_tasks() {
    loop {
        if let Some(next_task) = crate::task::scheduler::fetch_task() {
            let mut next_task_inner = next_task.inner.lock();
            let idle_task = IDLE_TASK.clone();
            let mut current_task_inner = idle_task.inner.lock();

            current_task_inner.task_status = TaskStatus::Ready;
            next_task_inner.task_status = TaskStatus::Running;
            let next_task_kernel_stack = next_task.kstack.0;
            let next_tp = Arc::as_ptr(&next_task) as usize;
            log::error!("next_tp: {:#x}", next_tp);
            log::info!("next_task_kernel_stack: {:#x}", next_task_kernel_stack);

            log::info!("switch to task {}", next_task.tid);
            // 注意这里要主动drop, 否则会造成死锁
            drop(current_task_inner);
            drop(next_task_inner);

            let mut processor = PROCESSOR.lock();
            processor.current = next_task.clone();
            drop(processor);
            drop(next_task);

            unsafe {
                switch::__switch(next_task_kernel_stack);
            }
            unreachable!("Unreachable in run_tasks");
        }
    }
}
