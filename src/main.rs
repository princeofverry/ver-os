#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]

extern crate alloc;

mod vga_buffer;
mod gdt;
mod interrupts;
mod memory;
mod allocator;
pub mod serial;
pub mod shell;
pub mod calculator;
use core::panic::PanicInfo;
use bootloader::{BootInfo, entry_point};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!(r#"
 __      __        ____   _____
 \ \    / /       / __ \ / ____|
  \ \  / /__ _ __| |  | | (___
   \ \/ / _ \ '__| |  | |\___ \
    \  /  __/ |  | |__| |____) |
     \/ \___|_|   \____/|_____/
    "#);
    println!("Welcome to Veros!");

    gdt::init();
    interrupts::init_idt();
    unsafe { 
        let mut pics = interrupts::PICS.lock();
        pics.initialize();
        // Unmask IRQ1 (Keyboard) ONLY. Mask IRQ0 (Timer) to prevent freezes.
        pics.write_masks(0b1111_1111, 0b1111_1111);
    }
    x86_64::instructions::interrupts::enable();

    use x86_64::VirtAddr;
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        memory::BootInfoFrameAllocator::init(&boot_info.memory_map)
    };

    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");

    println!("It did not crash!");

    // Start the shell
    crate::println!("Calling run..."); crate::shell::run();
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {
        x86_64::instructions::hlt();
    }
}