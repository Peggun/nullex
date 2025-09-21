use core::pin::Pin;

use alloc::{boxed::Box, string::ToString, sync::Arc};
use crossbeam_queue::ArrayQueue;
use lazy_static::lazy_static;
use pc_keyboard::{layouts, HandleControl, KeyEvent, Keyboard, ScancodeSet1};
use spin::Mutex;

use crate::{print, println, serial, serial_println, task::{keyboard::ScancodeStream, ProcessState}, utils::process::spawn_process, vga_buffer::{BufferEntry, WRITER}};

lazy_static! {
    pub static ref PREV_BUFFER: Mutex<[[BufferEntry; 80]; 25]> =
        Mutex::new([[BufferEntry { character: 0, colour_code: 0 }; 80]; 25]);
}
 
pub fn nedit_app(args: &[&str]) {
    let main = spawn_process(|state| { Box::pin(nedit_main(state)) as Pin<Box<dyn Future<Output = i32>>> }, false);
}

pub async fn nedit_main(state: Arc<ProcessState>) -> i32 {
    // setup scancode queue and keyboard inputs
    // like the keyboard driver
    state.scancode_queue.try_init_once(|| ArrayQueue::new(100))
        .expect("Scancode stream should only be called once. NEdit.");

    let mut keyboard = Keyboard::new(
		ScancodeSet1::new(),
		layouts::Us104Key,
		HandleControl::Ignore
	);

    let snapshot = {
        let writer = WRITER.lock();           
        writer.copy_vga_buffer()
    };

    {
        let mut prev = PREV_BUFFER.lock();
        *prev = snapshot;
    } 

    serial_println!("prev_buffer[0][0]: {:#?}", snapshot[0][0]);

    {
        let mut writer = WRITER.lock();
        writer.clear_everything();
    }

    println!("Hello from nedit!");

    // main app loop
    loop {
        // print keypress for now
        // damn i gotta fix this.
        while let Some(queue) = state.scancode_queue.get().iter().next() {
            match queue.pop() {
                Some(c) => {
                    if let Ok(Some(key_event)) = keyboard.add_byte(c) {
                        if let Some(key) = keyboard.process_keyevent(key_event) {
                            match key {
                                pc_keyboard::DecodedKey::Unicode(ch) => {
                                    print!("{}", ch);
                                },
                                _ => {} // ignore rest.
                            }
                        }
                    }
                },
                None => {} // scanqueue empty, do nothing
            }
        }
        
    }
}