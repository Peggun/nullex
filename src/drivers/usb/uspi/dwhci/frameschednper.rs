use crate::{drivers::usb::uspi::{dwhci::{DWHCI_HOST_CHAN_INT_ACK, DWHCI_HOST_CHAN_INT_NAK, DWHCI_HOST_CHAN_INT_NYET, DWHCI_HOST_CHAN_INT_XFER_COMPLETE, frame_scheduler::{TDWHCIFrameScheduler, TFrameSchedulerState, uFRAME}, frameschednper::TFrameSchedulerState::StateUnknown}, os::us_delay}, kassert, serial_println};

#[repr(C)]
pub struct TDWHCIFrameSchedulerNonPeriodic {
	pub state: TFrameSchedulerState,
	pub tries: u32
}

impl TDWHCIFrameSchedulerNonPeriodic {
	pub fn new() -> Self {
		Self {
			state: TFrameSchedulerState::StateUnknown,
			tries: 0, // change here maybe?
		}
	}
}

impl TDWHCIFrameScheduler for TDWHCIFrameSchedulerNonPeriodic {
	fn start_split(&mut self) {
		self.state = TFrameSchedulerState::StateStartSplit;
	}

	fn complete_split(&mut self) -> bool {
		let mut result = false;

		match self.state {
			TFrameSchedulerState::StateStartSplitComplete => {
				self.state = TFrameSchedulerState::StateStartSplitComplete;
				self.tries = 3;
				result = true;
			},
			TFrameSchedulerState::StateStartSplit => {},
			TFrameSchedulerState::StateCompleteRetry => {
				us_delay(5 * uFRAME as u32);
				result = true;
			},
			TFrameSchedulerState::StateCompleteSplitComplete | TFrameSchedulerState::StateCompleteSplitFailed => {},
			_ => {
				kassert!(false, "Unknown USB Frame Scheduler State. {:#?}", self.state);
			}
		}

		result
	}

	fn transaction_complete(&mut self, status: u32) {
		match self.state {
			TFrameSchedulerState::StateStartSplit => {
				kassert!((status & DWHCI_HOST_CHAN_INT_ACK as u32) != 0,
					"Expected DWHCI_HOST_CHAN_INT_ACK in status when starting split. status=0x{:X}", status);
				self.state = TFrameSchedulerState::StateStartSplitComplete;
			},
			TFrameSchedulerState::StateCompleteSplit | TFrameSchedulerState::StateCompleteRetry => {
				if status as usize & DWHCI_HOST_CHAN_INT_XFER_COMPLETE != 0 {
					self.state = TFrameSchedulerState::StateStartSplitComplete;
				}
				else if status as usize & (DWHCI_HOST_CHAN_INT_NYET | DWHCI_HOST_CHAN_INT_ACK) != 0 {
					if self.tries - 1 == 0 {
						self.state = TFrameSchedulerState::StateCompleteSplitFailed;
					} else {
						self.state = TFrameSchedulerState::StateCompleteRetry;
					}
				}
				else if status as usize & DWHCI_HOST_CHAN_INT_NAK != 0 {
					if self.tries - 1 == 0 {
						us_delay(5 * uFRAME as u32);
						self.state = TFrameSchedulerState::StateCompleteSplitFailed;
					} else {
						self.state = TFrameSchedulerState::StateCompleteRetry;
					}
				} else {
					serial_println!("[dwsched] Invalid status 0x{:X}", status);
					kassert!(false, "Invalid status.");
				}
			},
			_ => {
				kassert!(false, "Invalid state.");
			}
		}
	}

	fn wait_for_frame(&mut self) {}
	
	fn is_odd_frame(&self) -> bool {
		false
	}
}