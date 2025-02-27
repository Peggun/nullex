// task/executor.rs
extern crate alloc;

use crate::println;

use super::{Process, ProcessId}; // Renamed Task to Process and TaskId to ProcessId
use alloc::{collections::BTreeMap, sync::Arc};
use lazy_static::lazy_static;
use core::task::{Context, Poll, Waker};
use crossbeam_queue::ArrayQueue;
use alloc::task::Wake;

pub struct Executor {
    processes: BTreeMap<ProcessId, Process>, // Renamed tasks to processes
    process_queue: Arc<ArrayQueue<ProcessId>>, // Renamed task_queue to process_queue
    waker_cache: BTreeMap<ProcessId, Waker>, // Keep waker_cache, but now for ProcessId
    next_pid: ProcessId, // For assigning process IDs
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            processes: BTreeMap::new(), // Renamed tasks to processes
            process_queue: Arc::new(ArrayQueue::new(100)), // Renamed task_queue to process_queue
            waker_cache: BTreeMap::new(),
            next_pid: ProcessId::new(0), // Initialize PID counter
        }
    }

    pub fn spawn_process(&mut self, process: Process) { // Renamed spawn to spawn_process
        let pid = process.id; // Use process ID (pid)
        if self.processes.insert(process.id, process).is_some() { // Renamed tasks to processes
            panic!("process with same ID already in processes"); // Renamed task to process and tasks to processes
        }
        self.process_queue.push(pid).expect("queue full"); // Renamed task_queue to process_queue
    }

    fn run_ready_processes(&mut self) { // Renamed run_ready_tasks to run_ready_processes
        //println!("Running processes..."); // Changed tasks to processes
        // destructure `self` to avoid borrow checker errors
        let Self {
            processes, // Renamed tasks to processes
            process_queue, // Renamed task_queue to process_queue
            waker_cache,
            .. // Ignore next_pid for now in this destructuring
        } = self;

        while let Some(pid) = process_queue.pop() { // Renamed task_queue to process_queue and task_id to pid
            let process = match processes.get_mut(&pid) { // Renamed tasks to processes and task_id to pid
                Some(process) => process, // Renamed task to process
                None => continue, // process no longer exists // Renamed task to process
            };
            let waker = waker_cache
                .entry(pid) // Use pid
                .or_insert_with(|| ProcessWaker::new(pid, process_queue.clone())); // Renamed TaskWaker to ProcessWaker and task_queue to process_queue
            let mut context = Context::from_waker(waker);
            match process.poll(&mut context) { // Renamed task to process
                Poll::Ready(exit_code) => { // Process can now return an exit code
                    // process done -> remove it and its cached waker // Renamed task to process
                    processes.remove(&pid); // Renamed tasks to processes and task_id to pid
                    waker_cache.remove(&pid); // Renamed task_id to pid
                    println!("Process {} exited with code: {}", pid.0, exit_code); // Indicate process exit
                }
                Poll::Pending => {}
            }
        }
    }

    pub fn run(&mut self) -> ! {
        loop {
            self.run_ready_processes(); // Renamed run_ready_tasks to run_ready_processes
            self.sleep_if_idle();
        }
    }

    fn sleep_if_idle(&self) {
        use x86_64::instructions::interrupts;

        // Atomically check queue state
        interrupts::disable();
        if self.process_queue.is_empty() { // Renamed task_queue to process_queue
            interrupts::enable_and_hlt();
        } else {
            interrupts::enable();
        }
    }

    pub fn create_pid(&mut self) -> ProcessId {
        let pid = self.next_pid;
        self.next_pid = ProcessId::new(pid.0 + 1); // Simple increment for PID
        pid
    }
}

struct ProcessWaker { // Renamed TaskWaker to ProcessWaker
    pid: ProcessId, // Renamed task_id to pid
    process_queue: Arc<ArrayQueue<ProcessId>>, // Renamed task_queue to process_queue and TaskId to ProcessId
}

impl ProcessWaker { // Renamed TaskWaker to ProcessWaker
    fn wake_process(&self) { // Renamed wake_task to wake_process
        self.process_queue.push(self.pid).expect("process_queue full"); // Renamed task_queue to process_queue and task_id to pid
    }

    fn new(pid: ProcessId, process_queue: Arc<ArrayQueue<ProcessId>>) -> Waker { // Renamed TaskWaker to ProcessWaker, task_id to pid, and task_queue to process_queue and TaskId to ProcessId
        Waker::from(Arc::new(ProcessWaker { // Renamed TaskWaker to ProcessWaker
            pid, // Renamed task_id to pid
            process_queue, // Renamed task_queue to process_queue
        }))
    }
}

impl Wake for ProcessWaker { // Renamed TaskWaker to ProcessWaker
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