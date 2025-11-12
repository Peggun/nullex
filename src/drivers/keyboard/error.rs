// code from https://github.com/rust-embedded-community/pc-keyboard
// license in THIRD_PARTY_LICENSE


#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum KeyboardError {
    BadStartBit,
    BadStopBit,
    ParityError,
    UnknownKeyCode,
    #[doc(hidden)]
    InvalidState,
}
