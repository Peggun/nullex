// executor.rs

/*
Task executor module for the kernel.
*/

extern crate alloc;

use crate::{println, serial_println};

use super::{Process, ProcessId, ProcessState}; // Renamed Task to Process and TaskId to ProcessId
use alloc::task::Wake;
use alloc::{collections::BTreeMap, sync::Arc};
use spin::mutex::SpinMutex;
use core::task::{Context, Poll, Waker};
use crossbeam_queue::ArrayQueue;
use lazy_static::lazy_static;

lazy_static! {
    // Now CURRENT_PROCESS holds a reference to the immutable ProcessState.
    pub static ref CURRENT_PROCESS: SpinMutex<Option<Arc<ProcessState>>> = SpinMutex::new(None);
}


pub struct Executor {
    pub processes: BTreeMap<ProcessId, Arc<SpinMutex<Process>>>,
    pub process_queue: Arc<ArrayQueue<ProcessId>>,
    pub waker_cache: BTreeMap<ProcessId, Waker>,
    pub next_pid: ProcessId,
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            processes: BTreeMap::new(),
            process_queue: Arc::new(ArrayQueue::new(100)),
            waker_cache: BTreeMap::new(),
            next_pid: ProcessId::new(0),
        }
    }

    pub fn spawn_process(&mut self, process: Process) {
        let pid = process.state.id;
        let process_arc = Arc::new(SpinMutex::new(process)); // Wrap in Arc<SpinMutex>
        if self.processes.insert(pid, process_arc).is_some() {
            panic!("process with same ID already in processes");
        }
        self.process_queue.push(pid).expect("queue full");
    }

    // Remove run_cycle since we'll handle it in main.rs
    pub fn sleep_if_idle(&self) {
        use x86_64::instructions::interrupts;
        interrupts::disable();
        if self.process_queue.is_empty() {
            interrupts::enable_and_hlt();
        } else {
            interrupts::enable();
        }
    }

    pub fn create_pid(&mut self) -> ProcessId {
        let pid = self.next_pid;
        self.next_pid = ProcessId::new(pid.0 + 1);
        pid
    }

    pub fn list_processes(&self) {
        println!("Running processes:");
        for (pid, _) in &self.processes {
            println!("  Process {}", pid.0);
        }
    }
}

pub struct ProcessWaker {
    // Renamed TaskWaker to ProcessWaker
    pub pid: ProcessId,                            // Renamed task_id to pid
    pub process_queue: Arc<ArrayQueue<ProcessId>>, // Renamed task_queue to process_queue and TaskId to ProcessId
}

impl ProcessWaker {
    // Renamed TaskWaker to ProcessWaker
    pub fn wake_process(&self) {
        // Renamed wake_task to wake_process
        self.process_queue
            .push(self.pid)
            .expect("process_queue full"); // Renamed task_queue to process_queue and task_id to pid
    }

    pub fn new(pid: ProcessId, process_queue: Arc<ArrayQueue<ProcessId>>) -> Waker {
        // Renamed TaskWaker to ProcessWaker, task_id to pid, and task_queue to process_queue and TaskId to ProcessId
        Waker::from(Arc::new(ProcessWaker {
            // Renamed TaskWaker to ProcessWaker
            pid,           // Renamed task_id to pid
            process_queue, // Renamed task_queue to process_queue
        }))
    }
}

impl Wake for ProcessWaker {
    // Renamed TaskWaker to ProcessWaker
    fn wake(self: Arc<Self>) {
        self.wake_process(); // Renamed wake_task to wake_process
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_process(); // Renamed wake_task to wake_process
    }
}

lazy_static! {
    pub static ref EXECUTOR: spin::Mutex<Executor> = spin::Mutex::new(Executor::new());
}