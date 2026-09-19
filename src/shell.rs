use alloc::string::String;
use alloc::vec::Vec;
use crate::{print, println, calculator, interrupts};
use x86_64::instructions::port::Port;
use crate::vga_buffer::Color;

pub struct File {
    pub name: String,
    pub content: String,
}

lazy_static::lazy_static! {
    pub static ref VFS: spin::Mutex<Vec<File>> = spin::Mutex::new(Vec::new());
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

pub fn get_rtc_time() -> (u8, u8, u8) {
    let mut addr_port = Port::<u8>::new(0x70);
    let mut data_port = Port::<u8>::new(0x71);
    unsafe {
        loop {
            addr_port.write(0x0A);
            if (data_port.read() & 0x80) == 0 {
                break;
            }
            core::hint::spin_loop();
        }

        addr_port.write(0x00); let mut s = data_port.read();
        addr_port.write(0x02); let mut m = data_port.read();
        addr_port.write(0x04); let mut h = data_port.read();

        addr_port.write(0x0B); let reg_b = data_port.read();
        if (reg_b & 0x04) == 0 {
            s = (s & 0x0F) + ((s >> 4) * 10);
            m = (m & 0x0F) + ((m >> 4) * 10);
            h = (h & 0x0F) + ((h >> 4) * 10);
        }
        h = (h + 7) % 24;
        (h, m, s)
    }
}

pub fn update_top_clock() {
    static mut LAST_SEC: u8 = 255;
    let (h, m, s) = get_rtc_time();
    unsafe {
        if s != LAST_SEC {
            LAST_SEC = s;
            let time_str = alloc::format!("[ {:02}:{:02}:{:02} WIB ]", h, m, s);
            crate::vga_buffer::WRITER.lock().write_string_at(0, 60, &time_str);
        }
    }
}

fn read_char() -> char {
    loop {
        update_top_clock();
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

pub fn read_serial_line() -> String {
    let mut buf = String::new();
    loop {
        let mut serial = crate::serial::SERIAL1.lock();
        let c = serial.receive() as char;
        drop(serial);
        if c == '\n' { break; }
        if c != '\r' { buf.push(c); }
    }
    buf
}

pub fn run() -> ! {
    update_top_clock();
    println!("Type 'help' for a list of commands.");
    print!("> ");
    
    loop {
        let cmd = read_line();
        process_command(&cmd);
        print!("> ");
    }
}

fn play_snake() {
    let mut writer = crate::vga_buffer::WRITER.lock();
    writer.clear_screen();
    drop(writer);

    let arena_width: i32 = 30;
    let arena_height: i32 = 18;
    let offset_x: usize = 25;
    let offset_y: usize = 3;

    let mut snake: Vec<(i32, i32)> = alloc::vec![(15, 9), (14, 9), (13, 9)];
    let mut dir: (i32, i32) = (1, 0);
    let mut food: (i32, i32) = (20, 9);
    let mut score: u32 = 0;
    let mut rng_state: u32 = {
        let (h, m, s) = get_rtc_time();
        (h as u32 * 3600 + m as u32 * 60 + s as u32) ^ 0xDEADBEEF
    };

    let mut rand_next = |max: i32| -> i32 {
        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        let val = (rng_state / 65536) % 32768;
        ((val as i32) % max).abs()
    };

    // Draw borders and header
    {
        let mut writer = crate::vga_buffer::WRITER.lock();
        writer.write_string_at(offset_y - 2, offset_x, "=== VEROS RETRO SNAKE ===");
        let score_str = alloc::format!("Score: {} | Controls: WASD, Q to Quit", score);
        writer.write_string_at(offset_y - 1, offset_x, &score_str);

        for x in 0..(arena_width + 2) {
            writer.write_string_at(offset_y, offset_x + x as usize, "#");
            writer.write_string_at(offset_y + arena_height as usize + 1, offset_x + x as usize, "#");
        }
        for y in 0..(arena_height + 2) {
            writer.write_string_at(offset_y + y as usize, offset_x, "#");
            writer.write_string_at(offset_y + y as usize, offset_x + arena_width as usize + 1, "#");
        }
    }

    loop {
        let mut key = None;
        for _ in 0..120_000 {
            if let Some(c) = try_read_char() {
                key = Some(c);
            }
            core::hint::spin_loop();
        }

        if let Some(c) = key {
            match c {
                'w' | 'W' => if dir.1 != 1 { dir = (0, -1); },
                's' | 'S' => if dir.1 != -1 { dir = (0, 1); },
                'a' | 'A' => if dir.0 != 1 { dir = (-1, 0); },
                'd' | 'D' => if dir.0 != -1 { dir = (1, 0); },
                'q' | 'Q' => break,
                _ => {}
            }
        }

        let head = snake[0];
        let new_head = (head.0 + dir.0, head.1 + dir.1);

        if new_head.0 < 0 || new_head.0 >= arena_width || new_head.1 < 0 || new_head.1 >= arena_height {
            break;
        }

        if snake.contains(&new_head) {
            break;
        }

        snake.insert(0, new_head);

        if new_head == food {
            score += 10;
            loop {
                let fx = rand_next(arena_width);
                let fy = rand_next(arena_height);
                if !snake.contains(&(fx, fy)) {
                    food = (fx, fy);
                    break;
                }
            }
            let mut writer = crate::vga_buffer::WRITER.lock();
            let score_str = alloc::format!("Score: {} | Controls: WASD, Q to Quit", score);
            writer.write_string_at(offset_y - 1, offset_x, &score_str);
        } else {
            let tail = snake.pop().unwrap();
            let mut writer = crate::vga_buffer::WRITER.lock();
            writer.write_string_at(offset_y + 1 + tail.1 as usize, offset_x + 1 + tail.0 as usize, " ");
        }

        {
            let mut writer = crate::vga_buffer::WRITER.lock();
            writer.write_string_at(offset_y + 1 + new_head.1 as usize, offset_x + 1 + new_head.0 as usize, "@");
            if snake.len() > 1 {
                let body = snake[1];
                writer.write_string_at(offset_y + 1 + body.1 as usize, offset_x + 1 + body.0 as usize, "O");
            }
            writer.write_string_at(offset_y + 1 + food.1 as usize, offset_x + 1 + food.0 as usize, "*");
        }
    }

    let mut writer = crate::vga_buffer::WRITER.lock();
    writer.write_string_at(offset_y + arena_height as usize / 2, offset_x + 6, "=== GAME OVER ===");
    let final_str = alloc::format!("Final Score: {} - Press any key", score);
    writer.write_string_at(offset_y + arena_height as usize / 2 + 1, offset_x + 3, &final_str);
    drop(writer);

    let _ = read_char();
    crate::vga_buffer::WRITER.lock().clear_screen();
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
            println!("  snake     - Play Retro Snake game");
            println!("  guess     - Play number guessing game");
            println!("  tictactoe - Play Tic-Tac-Toe");
            println!("  cpuinfo   - Show CPU Vendor String");
            println!("  weather   - Fetch real-time weather via Serial Proxy");
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
        "weather" => {
            println!("Connecting to Host Modem via COM1...");
            {
                let mut serial = crate::serial::SERIAL1.lock();
                use core::fmt::Write;
                serial.write_str("WEATHER\n").unwrap();
            }
            println!("Waiting for weather data from Internet...");
            let response = read_serial_line();
            println!("Host says: {}", response);
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
        "snake" => {
            play_snake();
        }
        "top" => {
            crate::vga_buffer::WRITER.lock().clear_screen();
            let mut last_s = 255;
            loop {
                let (h, m, s) = get_rtc_time();
                if s != last_s {
                    last_s = s;
                    let used = crate::allocator::HEAP_SIZE - crate::allocator::ALLOCATOR.lock().free();
                    let ticks = interrupts::TICKS.load(core::sync::atomic::Ordering::Relaxed);
                    
                    let mut writer = crate::vga_buffer::WRITER.lock();
                    writer.write_string_at(0, 0, "=== VEROS DYNAMIC SYSTEM MONITOR ===");
                    let time_str = alloc::format!("Live RTC Time : {:02}:{:02}:{:02} WIB       ", h, m, s);
                    writer.write_string_at(2, 0, &time_str);
                    let mem_str = alloc::format!("Heap Memory   : {} bytes used / {} total    ", used, crate::allocator::HEAP_SIZE);
                    writer.write_string_at(3, 0, &mem_str);
                    let tick_str = alloc::format!("System Ticks  : {} (Timer Muted)       ", ticks);
                    writer.write_string_at(4, 0, &tick_str);
                    writer.write_string_at(6, 0, "Press 'q' to exit.              ");
                }

                if let Some('q') = try_read_char() {
                    crate::vga_buffer::WRITER.lock().clear_screen();
                    break;
                }
                core::hint::spin_loop();
            }
        }
        "clock" => {
            let (h, m, s) = get_rtc_time();
            println!("RTC Time: {:02}:{:02}:{:02} WIB", h, m, s);
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
