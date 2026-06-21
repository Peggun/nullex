use crate::{drivers::usb::uspi::{dwhci::{DWHCI_HOST_CHAN_INT_ACK, DWHCI_HOST_CHAN_INT_NAK, DWHCI_HOST_CHAN_INT_NYET, DWHCI_HOST_CHAN_INT_XFER_COMPLETE, DWHCI_HOST_FRM_NUM, dwhci_host_frm_num_number, frame_scheduler::{TDWHCIFrameScheduler, TFrameSchedulerState, uFRAME}, register::TDWHCIRegister}, os::us_delay}, kassert, serial_println};

pub const FRAME_UNSET: usize = 8;

pub struct TDWHCIFrameSchedulerPeriodic {
    pub state: TFrameSchedulerState,
    pub tries: u32,
    pub next_frame: u32,
}

impl TDWHCIFrameSchedulerPeriodic {
    pub fn new() -> Self {
        Self {
            state: TFrameSchedulerState::StateUnknown,
            tries: 0,
            next_frame: FRAME_UNSET as u32,
        }
    }
}

impl TDWHCIFrameScheduler for TDWHCIFrameSchedulerPeriodic {
    fn start_split(&mut self) {
        self.state = TFrameSchedulerState::StateStartSplit;
        self.next_frame = FRAME_UNSET as u32;
    }

    fn complete_split(&mut self) -> bool {
        let mut result = false;

        match self.state {
            TFrameSchedulerState::StateStartSplitComplete => {
                self.state = TFrameSchedulerState::StateStartSplitComplete;
                self.tries = if self.next_frame != 5 { 3 } else { 2 };
                self.next_frame = (self.next_frame + 2) & 7;
                result = true;
            },
            TFrameSchedulerState::StateCompleteRetry => {
                result = true;
                self.next_frame = (self.next_frame + 1) & 7;
            },
            TFrameSchedulerState::StateCompleteSplitComplete => {},
            TFrameSchedulerState::StateCompleteSplitFailed => {},
            _ => {
                kassert!(false, "Unknown or invalid state in complete_split");
            }
        }

        result
    }

    fn transaction_complete(&mut self, status: u32) {
        match self.state {
            TFrameSchedulerState::StateStartSplit => {
                kassert!(status & DWHCI_HOST_CHAN_INT_ACK as u32 != 0, "Expected DWHCI_HOST_CHAN_INT_ACK set for StartSplit completion");
                self.state = TFrameSchedulerState::StateStartSplitComplete;
            },
            TFrameSchedulerState::StateCompleteSplit | TFrameSchedulerState::StateCompleteRetry => {
                if (status & DWHCI_HOST_CHAN_INT_XFER_COMPLETE as u32) != 0 {
                    self.state = TFrameSchedulerState::StateCompleteSplitComplete;
                } else if (status & (DWHCI_HOST_CHAN_INT_NYET | DWHCI_HOST_CHAN_INT_ACK) as u32) != 0 {
                    if self.tries == 0 {
                        self.state = TFrameSchedulerState::StateCompleteSplitFailed;
                        us_delay(8 * uFRAME as u32);
                    } else {
                        self.tries -= 1;
                        self.state = TFrameSchedulerState::StateCompleteRetry;
                    }
                } else if (status & DWHCI_HOST_CHAN_INT_NAK as u32) != 0 {
                    us_delay(5 * uFRAME as u32);
                    self.state = TFrameSchedulerState::StateCompleteSplitFailed;
                } else {
                    serial_println!("Unexpected status: {:#X}", status);
                    kassert!(false, "Unexpected status in transaction_complete");
                }
            }
            _ => {
                kassert!(false, "Unknown or invalid state in transaction_complete");
            }
        }
    }

    fn wait_for_frame(&mut self) {
        let mut frame_number = TDWHCIRegister::new(DWHCI_HOST_FRM_NUM);

        if self.next_frame == FRAME_UNSET as u32 {
            self.next_frame = ((dwhci_host_frm_num_number(frame_number.read() as usize) + 1) & 7) as u32;
            if self.next_frame == 6 {
                self.next_frame += 1;
            }
        }

        while (dwhci_host_frm_num_number(frame_number.read() as usize) & 7) != self.next_frame as usize {
            // do nothing
        }

        frame_number.invalidate();
    }

    fn is_odd_frame(&self) -> bool {
        (self.next_frame & 1) != 0
    }
}