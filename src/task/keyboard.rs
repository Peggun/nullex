extern crate alloc;

use core::task::Poll;

use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use conquer_once::spin::OnceCell;
use crossbeam::{epoch::Pointable, queue::ArrayQueue};
use futures_util::{task::AtomicWaker, Stream, StreamExt};
use lazy_static::lazy_static;
use pc_keyboard::{layouts, HandleControl, KeyCode, Keyboard, ScancodeSet1};
use spin::Mutex;
use vga::{
    colors::Color16,
    writers::{  
        Graphics1280x800x256, Graphics320x200x256, Graphics320x240x256, GraphicsWriter, Text80x25,
        TextWriter,
    },
};

use crate::{
    fs::{self, ramfs::Permission, FS}, print, println, serial_println, vga_buffer::{console_backspace, string_to_color, Color, WRITER}
};

lazy_static! {
    static ref CWD: Mutex<String> = Mutex::new("/".to_string());
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

pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();

    let mut keyboard = Keyboard::new(
        ScancodeSet1::new(),
        layouts::Us104Key,
        HandleControl::Ignore,
    );

    let mut line = String::new();
    let mut current_dir = String::from("/".to_string());

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
                            print!("^C\ntest@nullex: {} $", *CWD.lock());
                            line.clear();
                        } else {
                            serial_println!("unhandled key {:?}", key);
                        }
                    }
                    pc_keyboard::DecodedKey::Unicode(c) => {
                        // backspace
                        if c as u8 == 8 {
                            line.pop();
                            console_backspace();
                            continue;
                        } else if c as u8 == 27 {
                            // let text = Text80x25::new();
                            // text.set_mode();
                            // text.clear_screen();
                            WRITER.lock().clear_everything();
                            print!("test@nullex: {} $ ", *CWD.lock());
                            continue;
                        }

                        print!("{}", c);
                        if c == '\n' && !line.is_empty() {
                            process_command(&line);
                            line.clear();
                            print!("test@nullex: {} $ ", *CWD.lock());
                        } else {
                            line.push(c);
                        }
                    }
                }
            }
        }
    }
}

pub enum MemoryFile {
    Static(&'static [u8]),
    Dynamic(Vec<u8>),
}

impl AsRef<[u8]> for MemoryFile {
    fn as_ref(&self) -> &[u8] {
        match self {
            MemoryFile::Static(r) => r,
            MemoryFile::Dynamic(r) => r.as_slice(),
        }
    }
}


const FS_SEP: char = '/';

fn process_command(input: &str) {
    let command_line = input.to_string();
    //println!();

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
        "touch" => touch(args),   // Add touch
        "mkdir" => mkdir(args),   // Add mkdir
        "rm" => rm(args),         // Add rm
        "write" => write_file(args),
        _ => println!("Command not found: {}", command),
    }
}

fn write_file(args: &[&str]) {
    if args.len() < 2 {
        println!("Usage: write <file> <content>");
        return;
    }
    let path = resolve_path(args[0]);
    let content = args[1..].join(" ");
    fs::with_fs(|fs| {
        if let Err(_) = fs.write_file(&path, content.as_bytes()) {
            println!("write: failed to write to '{}'", args[0]);
        }
    });
}

fn mkdir(args: &[&str]) {
    if args.is_empty() {
        println!("mkdir: missing operand");
        return;
    }
    for dir in args {
        let path = resolve_path(dir);
        fs::with_fs(|fs| {
            if let Err(_) = fs.create_dir(&path, Permission::all()) {
                println!("mkdir: cannot create directory '{}'", dir);
            }
        });
    }
}

fn touch(args: &[&str]) {
    if args.is_empty() {
        println!("touch: missing file operand");
        return;
    }
    for file in args {
        let path = resolve_path(file);
        fs::with_fs(|fs| {
            if fs.read_file(&path).is_err() {
                // Create empty file if it doesn't exist
                if let Err(_) = fs.create_file(&path, Permission::all()) {
                    println!("touch: cannot create file '{}'", file);
                }
            }
        });
    }
}

fn rm(args: &[&str]) {
    if args.is_empty() {
        println!("rm: missing operand");
        return;
    }
    for arg in args {
        let path = resolve_path(arg);
        fs::with_fs(|fs| {
            if fs.is_dir(&path) {
                println!("rm: cannot remove '{}': Is a directory", arg);
            } else {
                match fs.remove(&path) {
                    Ok(_) => {},
                    Err(_) => println!("rm: cannot remove '{}': No such file", arg),
                }
            }
        });
    }
}

fn echo(args: &[&str]) {
    println!("{}", args.join(" "));
}

fn clear() {
    WRITER.lock().clear_everything(); // Access clear_everything through WRITER
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
            
            Ok(content) => {
                let s = String::from_utf8_lossy(content);
                println!("{:#?}", s)
            },
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

pub fn join_paths(path: &str, next: &str, out: &mut String) {
    out.clear();
    if !next.starts_with(FS_SEP) {
        out.push_str(path);
        if !path.ends_with(FS_SEP) {
            out.push(FS_SEP);
        }
    }
    out.push_str(next);
    if out.ends_with(FS_SEP) {
        out.pop();
    }
}