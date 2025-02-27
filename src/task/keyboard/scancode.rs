// src/keyboard/scancode.rs
extern crate alloc;

use core::task::Poll;
use alloc::string::{String, ToString};
use conquer_once::spin::OnceCell;
use crossbeam::queue::ArrayQueue;
use futures_util::{
    task::AtomicWaker,
    Stream,
    StreamExt,
};
use embassy_futures::yield_now;
use lazy_static::lazy_static;
use pc_keyboard::{layouts, HandleControl, KeyCode, Keyboard, ScancodeSet1};
use spin::Mutex;

use crate::{
    print, println, serial_println,
    vga_buffer::{console_backspace, WRITER},
};

lazy_static! {
    pub static ref CWD: Mutex<String> = Mutex::new("/".to_string());
}

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Err(_) = queue.push(scancode) {
            println!(
                "WARNING: scancode queue full; dropping keyboard input {}",
                scancode
            );
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
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once");

        Self { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("SCANCODE_QUEUE not initialized");

        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(&cx.waker());

        match queue.pop() {
            Some(c) => {
                WAKER.take();
                Poll::Ready(Some(c))
            }
            None => Poll::Pending,
        }
    }
}

/// The async function that reads scancodes and processes keypresses.
/// Notice that when a full command line is ready (on newline),
/// we yield before calling run_command so that any locks (e.g. the VGA writer lock)
/// used during key echoing have been released.
pub async fn print_keypresses() -> i32 {
    let mut scancodes = ScancodeStream::new();

    let mut keyboard = Keyboard::new(
        ScancodeSet1::new(),
        layouts::Us104Key,
        HandleControl::Ignore,
    );

    let mut line = String::new();

    print!("test@nullex: {} $ ", *CWD.lock());
    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    pc_keyboard::DecodedKey::RawKey(key) => {
                        if key == KeyCode::LControl
                            || key == KeyCode::RControl
                            || key == KeyCode::RControl2
                        {
                            print!("^C\ntest@nullex: {} $ ", *CWD.lock());
                            line.clear();
                        } else {
                            serial_println!("unhandled key {:?}", key);
                        }
                    }
                    pc_keyboard::DecodedKey::Unicode(c) => {
                        // Backspace (ASCII 8)
                        if c as u8 == 8 {
                            if !line.is_empty() {
                                line.pop();
                                console_backspace();
                            }
                            continue;
                        // Escape (ASCII 27): clear screen
                        } else if c as u8 == 27 {
                            WRITER.lock().clear_everything();
                            print!("test@nullex: {} $ ", *CWD.lock());
                            continue;
                        }
                        // Echo the character.
                        print!("{}", c);
                        if c == '\n' && !line.is_empty() {
                            // Clone the full line before clearing it.
                            let command_line = line.clone();
                            line.clear();
                            // Yield to ensure that any temporary locks (e.g. from print! calls)
                            // are released before processing the command.
                            yield_now().await;
                            crate::task::keyboard::commands::run_command(&command_line);
                            print!("test@nullex: {} $ ", *CWD.lock());
                        } else {
                            line.push(c);
                        }
                    }
                }
            }
        }
    }
    0 // Return an exit code when input stops.
}
