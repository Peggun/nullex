pub const uFRAME: usize = 125; // micro seconds

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TFrameSchedulerState {
    StateStartSplit = 0,
    StateStartSplitComplete = 1,
    StateCompleteSplit = 2,
    StateCompleteRetry = 3,
    StateCompleteSplitComplete = 4,
    StateCompleteSplitFailed = 5,
    StateUnknown = 6,
}

pub trait TDWHCIFrameScheduler {
 	fn start_split(&mut self);
 	fn complete_split(&mut self) -> bool;
 	fn transaction_complete(&mut self, status: u32);
 	fn wait_for_frame(&mut self);
 	fn is_odd_frame(&self) -> bool;
}