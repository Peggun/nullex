// keyboard.rs
use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;

use futures_util::stream::StreamExt;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1, KeyCode};
use crate::print;

// use crate::task::keyboard::DecodedKey::RawKey; // Remove this line

use core::{pin::Pin, task::{Poll, Context}};
use futures_util::stream::Stream;

use crate::println;
use crate::vga_buffer::clear_screen; // Import clear_screen function

use futures_util::task::AtomicWaker;

static WAKER: AtomicWaker = AtomicWaker::new();
static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();

/// Called by the keyboard interrupt handler
///
/// Must not block or allocate.
pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Err(_) = queue.push(scancode) {
            println!("WARNING: scancode queue full; dropping keyboard input");
        } else {
            WAKER.wake();
        }
    } else {
        println!("WARNING: scancode queue uninitialized");
    }
}

pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE.try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once");
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("scancode queue not initialized");

        // fast path
        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(&cx.waker());
        match queue.pop() {
            Some(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            None => Poll::Pending,
        }
    }
}

pub async fn print_keypresses() {
    use crate::vga_buffer::WRITER;
    // use pc_keyboard::KeyCode; // Import KeyCode for comparison - already imported

    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(ScancodeSet1::new(),
        layouts::Us104Key, HandleControl::Ignore);

    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => print!("{}", character),
                    DecodedKey::RawKey(raw_key) => {
                        match raw_key {
                            KeyCode::Backspace => {
                                if WRITER.lock().column_position > WRITER.lock().input_start_column {
                                    WRITER.lock().write_byte(8); // ASCII code for backspace
                                }
                            }
                            KeyCode::F1 => { // Example: F1 key to clear screen
                                clear_screen();
                            }
                            _ => print!("{:?}", raw_key), // For other raw keys, print debug info
                        }
                    }
                }
            }
        }
    }
}