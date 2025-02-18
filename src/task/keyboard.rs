// keyboard.rs

extern crate alloc;

use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;

use futures_util::stream::StreamExt;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1, KeyCode};
use crate::{fs, print, printnl};
use alloc::{borrow::ToOwned, string::{String, ToString}, vec::Vec};

// use crate::task::keyboard::DecodedKey::RawKey; // Remove this line

use core::{pin::Pin, task::{Poll, Context}};
use futures_util::stream::Stream;

use crate::println;
use crate::vga_buffer::clear_screen; // Import clear_screen function

use futures_util::task::AtomicWaker;

static WAKER: AtomicWaker = AtomicWaker::new();
static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();

use lazy_static::lazy_static;
use spin::Mutex;
lazy_static! {
    static ref CWD: Mutex<String> = Mutex::new("/".to_string());
}
// Inside the print_keypresses function, after the input is echoed:
fn process_command(input: &str) {
    let command_line = input.to_string();
    printnl!();

    // Then process without holding the lock
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }
    let command = parts[0];
    let args = &parts[1..];

    match command {
        "echo" => echo(args),
        "clear" => clear(),
        "help" => help(),
        "ls" => ls(args),
        "cat" => cat(args),
        "cd" => cd(args),
        _ => println!("Command not found: {}", command),
    }
}

fn echo(args: &[&str]) {
    println!("{}", args.join(" "));
}

fn clear() {
    clear_screen();
}

fn help() {
    println!("Available commands:");
    println!("echo <args>   - Print arguments");
    println!("clear         - Clear the screen");
    println!("help          - Show this help");
    println!("ls [dir]      - List directory contents");
    println!("cat <file>    - Display file content");
    println!("cd <dir>      - Change directory");
}

fn ls(args: &[&str]) {
    let path = resolve_path(if args.is_empty() { "." } else { args[0] });
    fs::with_fs(|fs| {
        match fs.list_dir(&path) {
            Ok(entries) => {
                for entry in entries {
                    print!("{} ", entry);
                }
                println!();
            }
            Err(_) => println!("ls: cannot access '{}'", path),
        }
    });
}

fn cat(args: &[&str]) {
    if args.is_empty() {
        println!("cat: missing file operand");
        return;
    }
    let path = resolve_path(args[0]);
    fs::with_fs(|fs| {
        match fs.read_file(&path) {
            Ok(content) => println!("{:#?}", content),
            Err(_) => println!("cat: {}: No such file", path),
        }
    });
}

fn cd(args: &[&str]) {
    let path = if args.is_empty() {
        "/".to_string()
    } else {
        resolve_path(args[0])
    };

    fs::with_fs(|fs| {
        if fs.is_dir(&path) {
            *CWD.lock() = path;
        } else {
            println!("cd: no such directory: {}", args[0]);
        }
    });
}

fn resolve_path(path: &str) -> String {
    let mut cwd = CWD.lock().clone();
    let mut result = if path.starts_with('/') {
        String::new()
    } else {
        cwd.push('/');
        cwd
    };
    result.push_str(path);
    normalize_path(&result)
}

fn normalize_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').filter(|&p| !p.is_empty() && p != ".").collect();
    let mut stack = Vec::new();
    for part in parts {
        if part == ".." {
            if !stack.is_empty() {
                stack.pop();
            }
        } else {
            stack.push(part);
        }
    }
    if stack.is_empty() {
        "/".to_string()
    } else {
        format!("/{}/", stack.join("/"))
    }
}

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
    use core::fmt::Write;

    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(ScancodeSet1::new(), layouts::Us104Key, HandleControl::Ignore);
    let mut input_buffer = String::new();

    // Print initial prompt
    {
        let mut writer = WRITER.lock();
        write!(writer, "test@nullex: $ ").unwrap();
        writer.input_start_column = writer.column_position;
        writer.input_start_row = writer.row_position;
    }

    //println!("Keyboard task started");
    while let Some(scancode) = scancodes.next().await {
        //println!("Received scancode: {:x}", scancode);
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => {
                        if character == '\n' {
                            let input_copy = input_buffer.clone();
                            input_buffer.clear();
                            
                            process_command(&input_copy);
                            
                            // Print new prompt
                            print!("test@nullex: $ "); 
                        }
                        else {
                            // Store & print character
                            input_buffer.push(character);
                            let mut writer = WRITER.lock();
                            writer.write_byte(character as u8);
                        }
                    }
                    DecodedKey::RawKey(raw_key) => match raw_key {
                        KeyCode::Backspace => {
                            if !input_buffer.is_empty() {
                                input_buffer.pop();
                                let mut writer = WRITER.lock();
                                writer.backspace();
                                // Ensure buffer is cleared if empty (optional)
                                if input_buffer.is_empty() {
                                    input_buffer.clear();
                                }
                            }
                        }
                        KeyCode::F1 => {
                            // Clear screen (optional feature)
                            {
                                let mut writer = WRITER.lock();
                                writer.clear_screen();
                            }
                            input_buffer.clear();
                            let mut writer = WRITER.lock();
                            write!(writer, "test@nullex: $ ").unwrap();
                            writer.input_start_column = writer.column_position;
                        }
                        _ => {}
                    },
                }
            }
        }
    }
}