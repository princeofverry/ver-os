use alloc::string::String;
use crate::{print, println, calculator, interrupts};
use x86_64::instructions::port::Port;
use crate::vga_buffer::Color;

pub struct File {
    pub name: String,
    pub content: String,
}

lazy_static::lazy_static! {
    pub static ref VFS: spin::Mutex<alloc::vec::Vec<File>> = spin::Mutex::new(alloc::vec::Vec::new());
}

fn try_read_char() -> Option<char> {
    let mut status_port = Port::<u8>::new(0x64);
    let status = unsafe { status_port.read() };
    if status & 1 != 0 {
        let mut data_port = Port::<u8>::new(0x60);
        let scancode = unsafe { data_port.read() };
        let mut keyboard = interrupts::KEYBOARD.lock();
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    pc_keyboard::DecodedKey::Unicode(character) => return Some(character),
                    pc_keyboard::DecodedKey::RawKey(k) => {
                        if matches!(k, pc_keyboard::KeyCode::Backspace) {
                            return Some('\x08');
                        }
                    }
                }
            }
        }
    }
    None
}

fn read_char() -> char {
    loop {
        if let Some(c) = try_read_char() {
            return c;
        }
        core::hint::spin_loop();
    }
}

pub fn read_line() -> String {
    let mut buf = String::new();
    loop {
        let c = read_char();
        if c == '\n' {
            println!();
            return buf;
        } else if c == '\x08' {
            if !buf.is_empty() {
                buf.pop();
                crate::vga_buffer::WRITER.lock().backspace();
            }
        } else {
            buf.push(c);
            print!("{}", c);
        }
    }
}

pub fn run() -> ! {
    println!("Type 'help' for a list of commands.");
    print!("> ");
    
    loop {
        let cmd = read_line();
        process_command(&cmd);
        print!("> ");
    }
}

fn process_command(cmd: &str) {
    let cmd = cmd.trim();
    if cmd.is_empty() {
        return;
    }

    let mut parts = cmd.splitn(2, ' ');
    let command = parts.next().unwrap_or("");
    let args = parts.next().unwrap_or("").trim();

    match command {
        "help" => {
            println!("Available commands:");
            println!("  help      - Show this help message");
            println!("  clear     - Clear the screen");
            println!("  echo ...  - Print the given arguments");
            println!("  info      - Show system info");
            println!("  mem       - Show memory info");
            println!("  time      - Show uptime ticks");
            println!("  top       - Live dynamic system monitor (RTC, Mem)");
            println!("  clock     - Show real-time clock (RTC)");
            println!("  calc ...  - Evaluate arithmetic expression");
            println!("  ls        - List RAM disk files");
            println!("  touch ... - Create empty file");
            println!("  cat ...   - Read file content");
            println!("  write ... - Append to file (write <name> <content>)");
            println!("  rm ...    - Delete file");
            println!("  color ... - Change theme (matrix, ocean, default)");
            println!("  guess     - Play number guessing game");
            println!("  tictactoe - Play Tic-Tac-Toe");
            println!("  cpuinfo   - Show CPU Vendor String");
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
        "color" => {
            let mut writer = crate::vga_buffer::WRITER.lock();
            match args {
                "matrix" => writer.set_color(Color::LightGreen, Color::Black),
                "ocean" => writer.set_color(Color::White, Color::Blue),
                _ => writer.set_color(Color::Yellow, Color::Black),
            }
            writer.clear_screen();
        }
        "ls" => {
            let vfs = VFS.lock();
            if vfs.is_empty() {
                println!("RAM Disk is empty.");
            } else {
                for file in vfs.iter() {
                    print!("{}  ", file.name);
                }
                println!();
            }
        }
        "touch" => {
            if args.is_empty() { println!("Usage: touch <filename>"); }
            else {
                let mut vfs = VFS.lock();
                if vfs.iter().any(|f| f.name == args) {
                    println!("File '{}' already exists.", args);
                } else {
                    vfs.push(File { name: String::from(args), content: String::new() });
                    println!("Created '{}'.", args);
                }
            }
        }
        "cat" => {
            if args.is_empty() { println!("Usage: cat <filename>"); }
            else {
                let vfs = VFS.lock();
                if let Some(file) = vfs.iter().find(|f| f.name == args) {
                    println!("{}", file.content);
                } else {
                    println!("File not found.");
                }
            }
        }
        "write" => {
            let mut split = args.splitn(2, ' ');
            let name = split.next().unwrap_or("");
            let content = split.next().unwrap_or("");
            if name.is_empty() || content.is_empty() {
                println!("Usage: write <filename> <content>");
            } else {
                let mut vfs = VFS.lock();
                if let Some(file) = vfs.iter_mut().find(|f| f.name == name) {
                    file.content.push_str(content);
                    file.content.push('\n');
                    println!("Written to '{}'.", name);
                } else {
                    println!("File not found.");
                }
            }
        }
        "rm" => {
            if args.is_empty() { println!("Usage: rm <filename>"); }
            else {
                let mut vfs = VFS.lock();
                let len_before = vfs.len();
                vfs.retain(|f| f.name != args);
                if vfs.len() < len_before {
                    println!("Deleted '{}'.", args);
                } else {
                    println!("File not found.");
                }
            }
        }
        "guess" => {
            println!("Guess the number (1-100)!");
            let ticks = interrupts::TICKS.load(core::sync::atomic::Ordering::Relaxed);
            let mut target = (ticks % 100) as i64 + 1;
            if target == 1 { target = 42; }
            loop {
                print!("Guess: ");
                let line = read_line();
                if line.trim() == "quit" { break; }
                if let Ok(guess) = line.trim().parse::<i64>() {
                    if guess < target { println!("Too low!"); }
                    else if guess > target { println!("Too high!"); }
                    else { println!("Correct! You win!"); break; }
                } else {
                    println!("Please enter a number, or 'quit'.");
                }
            }
        }
        "tictactoe" => {
            println!("Tic-Tac-Toe! Type 1-9 to move.");
            let mut board = [' '; 9];
            let mut turn = 'X';
            loop {
                println!(" {} | {} | {} ", board[0], board[1], board[2]);
                println!("---+---+---");
                println!(" {} | {} | {} ", board[3], board[4], board[5]);
                println!("---+---+---");
                println!(" {} | {} | {} ", board[6], board[7], board[8]);
                print!("Player {}: ", turn);
                let line = read_line();
                if line.trim() == "quit" { break; }
                if let Ok(idx) = line.trim().parse::<usize>() {
                    if idx >= 1 && idx <= 9 && board[idx-1] == ' ' {
                        board[idx-1] = turn;
                        let wins = [[0,1,2],[3,4,5],[6,7,8],[0,3,6],[1,4,7],[2,5,8],[0,4,8],[2,4,6]];
                        let mut won = false;
                        for w in wins.iter() {
                            if board[w[0]] == turn && board[w[1]] == turn && board[w[2]] == turn {
                                won = true;
                                break;
                            }
                        }
                        if won {
                            println!("Player {} wins!", turn);
                            break;
                        }
                        turn = if turn == 'X' { 'O' } else { 'X' };
                    } else {
                        println!("Invalid move!");
                    }
                }
            }
        }
        "top" => {
            crate::vga_buffer::WRITER.lock().clear_screen();
            loop {
                let mut addr_port = Port::<u8>::new(0x70);
                let mut data_port = Port::<u8>::new(0x71);
                let (h, m, s) = unsafe {
                    addr_port.write(0x04); let h = data_port.read();
                    addr_port.write(0x02); let m = data_port.read();
                    addr_port.write(0x00); let s = data_port.read();
                    let h = (h & 0x0F) + ((h >> 4) * 10);
                    let m = (m & 0x0F) + ((m >> 4) * 10);
                    let s = (s & 0x0F) + ((s >> 4) * 10);
                    (h, m, s)
                };
                let used = crate::allocator::HEAP_SIZE - crate::allocator::ALLOCATOR.lock().free();
                let ticks = interrupts::TICKS.load(core::sync::atomic::Ordering::Relaxed);
                
                let mut writer = crate::vga_buffer::WRITER.lock();
                writer.write_string_at(0, 0, "=== VEROS DYNAMIC SYSTEM MONITOR ===");
                let time_str = alloc::format!("Live RTC Time : {:02}:{:02}:{:02} UTC       ", h, m, s);
                writer.write_string_at(2, 0, &time_str);
                let mem_str = alloc::format!("Heap Memory   : {} bytes used / {} total    ", used, crate::allocator::HEAP_SIZE);
                writer.write_string_at(3, 0, &mem_str);
                let tick_str = alloc::format!("System Ticks  : {} (Timer Muted)       ", ticks);
                writer.write_string_at(4, 0, &tick_str);
                writer.write_string_at(6, 0, "Press 'q' to exit.              ");
                drop(writer);

                let mut exit = false;
                for _ in 0..5_000_000 {
                    if let Some('q') = try_read_char() {
                        exit = true;
                        break;
                    }
                    core::hint::spin_loop();
                }
                if exit {
                    crate::vga_buffer::WRITER.lock().clear_screen();
                    break;
                }
            }
        }
        "clock" => {
            let mut addr_port = Port::<u8>::new(0x70);
            let mut data_port = Port::<u8>::new(0x71);
            unsafe {
                addr_port.write(0x04); let h = data_port.read();
                addr_port.write(0x02); let m = data_port.read();
                addr_port.write(0x00); let s = data_port.read();
                let h = (h & 0x0F) + ((h >> 4) * 10);
                let m = (m & 0x0F) + ((m >> 4) * 10);
                let s = (s & 0x0F) + ((s >> 4) * 10);
                println!("RTC Time: {:02}:{:02}:{:02} UTC", h, m, s);
            }
        }
        "cpuinfo" => {
            let cpuid = unsafe { core::arch::x86_64::__cpuid(0) };
            let mut vendor = [0u8; 12];
            vendor[0..4].copy_from_slice(&cpuid.ebx.to_le_bytes());
            vendor[4..8].copy_from_slice(&cpuid.edx.to_le_bytes());
            vendor[8..12].copy_from_slice(&cpuid.ecx.to_le_bytes());
            if let Ok(s) = core::str::from_utf8(&vendor) {
                println!("CPU Vendor: {}", s);
            } else {
                println!("Unknown CPU");
            }
        }
        "reboot" => {
            println!("Rebooting...");
            unsafe {
                let mut port = Port::<u8>::new(0x64);
                port.write(0xFE);
            }
            loop {
                core::hint::spin_loop();
            }
        }
        _ => {
            if let Some(idx) = cmd.find('=') {
                let var = cmd[..idx].trim().to_lowercase();
                let val_str = cmd[idx+1..].trim();
                if let Ok(val) = calculator::eval(val_str) {
                    crate::calculator::set_var(var, val);
                    println!("Stored.");
                    return;
                }
            }
            println!("Unknown command: '{}'. Type 'help' for a list of commands.", command);
        }
    }
}
