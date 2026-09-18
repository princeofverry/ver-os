use alloc::string::String;
use crate::{print, println, calculator, interrupts};
use x86_64::instructions::port::Port;

pub fn run() -> ! {
    println!("Type 'help' for a list of commands.");
    print!("> ");
    
    let mut command_buffer = String::new();

    loop {
        let key = x86_64::instructions::interrupts::without_interrupts(|| {
            interrupts::KEYBOARD_QUEUE.lock().pop()
        });

        if let Some(c) = key {
            if c == '\n' {
                println!();
                process_command(&command_buffer);
                command_buffer.clear();
                print!("> ");
            } else if c == '\x08' {
                if !command_buffer.is_empty() {
                    command_buffer.pop();
                    crate::vga_buffer::WRITER.lock().backspace();
                }
            } else {
                command_buffer.push(c);
                print!("{}", c);
            }
        } else {
            x86_64::instructions::hlt();
        }
    }
}

fn process_command(cmd: &str) {
    let cmd = cmd.trim();
    if cmd.is_empty() {
        return;
    }

    let mut parts = cmd.splitn(2, ' ');
    let command = parts.next().unwrap_or("");
    let args = parts.next().unwrap_or("");

    match command {
        "help" => {
            println!("Available commands:");
            println!("  help      - Show this help message");
            println!("  clear     - Clear the screen");
            println!("  echo ...  - Print the given arguments");
            println!("  info      - Show system info");
            println!("  mem       - Show memory info");
            println!("  time      - Show uptime ticks");
            println!("  calc ...  - Evaluate arithmetic expression");
            println!("  reboot    - Reboot the system");
        }
        "clear" => {
            crate::vga_buffer::WRITER.lock().clear_screen();
        }
        "echo" => {
            println!("{}", args);
        }
        "info" => {
            println!("Veros - Educational Rust OS");
        }
        "mem" => {
            println!("Heap allocator is active.");
        }
        "time" => {
            let ticks = interrupts::TICKS.load(core::sync::atomic::Ordering::Relaxed);
            println!("Ticks: {}", ticks);
        }
        "calc" => {
            if args.is_empty() {
                println!("Usage: calc <expression>");
            } else {
                match calculator::eval(args) {
                    Ok(result) => println!("Result: {}", result),
                    Err(e) => println!("Error: {}", e),
                }
            }
        }
        "reboot" => {
            println!("Rebooting...");
            unsafe {
                let mut port = Port::<u8>::new(0x64);
                port.write(0xFE);
            }
            loop {
                x86_64::instructions::hlt();
            }
        }
        _ => {
            println!("Unknown command: '{}'. Type 'help' for a list of commands.", command);
        }
    }
}