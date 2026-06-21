use crate::{drivers::usb::uspi::dwhci::{DWHCI_HOST_FRM_NUM, DWHCI_MAX_FRAME_NUMBER, dwhci_host_frm_num_number, frame_scheduler::TDWHCIFrameScheduler, register::TDWHCIRegister}, kassert};

pub const FRAME_UNSET: usize = DWHCI_MAX_FRAME_NUMBER + 1;

pub struct TDWHCIFrameSchedulerNoSplit {
    pub is_periodic: bool,
    pub next_frame: u32,
}

impl TDWHCIFrameSchedulerNoSplit {
    pub fn new(periodic: bool) -> Self {
        Self {
            is_periodic: periodic,
            next_frame: FRAME_UNSET as u32,
        }
    }
}

impl TDWHCIFrameScheduler for TDWHCIFrameSchedulerNoSplit {
    fn start_split(&mut self) {
        kassert!(false, "TDWHCIFrameSchedulerNoSplit has no split functions.");
    }

    fn complete_split(&mut self) -> bool {
        kassert!(false, "TDWHCIFrameSchedulerNoSplit has no split functions.");
        return false;
    }

    fn transaction_complete(&mut self, n_status: u32) {
        kassert!(false, "TDWHCIFrameSchedulerNoSplit has no transaction functions.")
    }

    fn wait_for_frame(&mut self) {
        let mut frame_number: TDWHCIRegister = TDWHCIRegister::new(DWHCI_HOST_FRM_NUM);
        self.next_frame = ((dwhci_host_frm_num_number(frame_number.read() as usize) + 1) & DWHCI_MAX_FRAME_NUMBER) as u32;

        if self.is_periodic {
            while ((dwhci_host_frm_num_number(frame_number.read() as usize)) & DWHCI_MAX_FRAME_NUMBER) as u32 != self.next_frame {
                // do nothing
            }
        }

        frame_number.invalidate();
    }

    fn is_odd_frame(&self) -> bool {
        (self.next_frame & 1) != 0
    }
}