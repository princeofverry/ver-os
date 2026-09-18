use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use lazy_static::lazy_static;
use crate::{print, println};
use crate::gdt;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        idt
    };
}
use pic8259::ChainedPics;
use spin;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

use core::sync::atomic::{AtomicUsize, Ordering};
pub static TICKS: AtomicUsize = AtomicUsize::new(0);

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    TICKS.fetch_add(1, Ordering::Relaxed);
    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use spin::Mutex;

pub const KEYBOARD_BUFFER_SIZE: usize = 256;
pub struct KeyboardBuffer {
    buffer: [char; KEYBOARD_BUFFER_SIZE],
    head: usize,
    tail: usize,
}

impl KeyboardBuffer {
    pub const fn new() -> Self {
        KeyboardBuffer { buffer: ['\0'; KEYBOARD_BUFFER_SIZE], head: 0, tail: 0 }
    }
    
    pub fn push(&mut self, c: char) {
        let next_head = (self.head + 1) % KEYBOARD_BUFFER_SIZE;
        if next_head != self.tail {
            self.buffer[self.head] = c;
            self.head = next_head;
        }
    }
    
    pub fn pop(&mut self) -> Option<char> {
        if self.head != self.tail {
            let c = self.buffer[self.tail];
            self.tail = (self.tail + 1) % KEYBOARD_BUFFER_SIZE;
            Some(c)
        } else {
            None
        }
    }
}

pub static KEYBOARD_QUEUE: Mutex<KeyboardBuffer> = Mutex::new(KeyboardBuffer::new());

lazy_static! {
    static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
        Mutex::new(Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore));
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    let mut keyboard = KEYBOARD.lock();
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => KEYBOARD_QUEUE.lock().push(character),
                DecodedKey::RawKey(key) => {
                    if matches!(key, pc_keyboard::KeyCode::Backspace) {
                        KEYBOARD_QUEUE.lock().push('\x08');
                    }
                }
            }
        }
    }

    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(
    stack_frame: InterruptStackFrame)
{
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame, _error_code: u64) -> !
{
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}