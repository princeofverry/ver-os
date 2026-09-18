# Veros (Verry OS)

```text
 __      __        ____   _____
 \ \    / /       / __ \ / ____|
  \ \  / /__ _ __| |  | | (___
   \ \/ / _ \ '__| |  | |\___ \
    \  /  __/ |  | |__| |____) |
     \/ \___|_|   \____/|_____/
```

Veros is a bare-metal 64-bit educational operating system written entirely in Rust. It was built from scratch to demonstrate low-level OS concepts such as VGA text buffering, hardware interrupts, dynamic memory allocation, and interactive command-line evaluation.

## Features
- **Custom Bootable Image**: Compiles to a bare-metal `.bin` image using the `bootimage` tool.
- **VGA Text Mode**: Full control over `0xb8000` with text colors and a visual backspace implementation.
- **Interrupt Handling (IDT & GDT)**: Catches CPU exceptions (like double faults) and hardware interrupts.
- **Hardware Drivers**: 
  - **Programmable Interrupt Controller (8259 PIC)**
  - **Timer (IRQ0)**: Tracks system uptime ticks.
  - **PS/2 Keyboard (IRQ1)**: Processes raw scancodes and routes them to a ring buffer safely.
- **Memory Management**: 
  - Frame Allocator parsing the bootloader's memory map.
  - Paging and dynamic heap allocation using `linked_list_allocator` (enables Rust's `String` and `Vec`).
- **Interactive Shell**: A built-in command prompt (try `help`, `echo`, `time`, `mem`, `clear`, `reboot`).
- **Math Calculator**: Includes a recursive descent parser to evaluate arithmetic strings (e.g., `calc (10 + 5) * 2`).

## Prerequisites (Windows)
1. Install **Visual Studio C++ Build Tools** (Required for the Rust Linker).
2. Install **QEMU** to run the virtual machine (`winget install qemu`).
3. Set up the specific Rust nightly toolchain and components:
   ```bash
   rustup toolchain install nightly-x86_64-pc-windows-gnu
   rustup component add rust-src llvm-tools-preview --toolchain nightly-x86_64-pc-windows-gnu
   ```
4. Install the Bootimage builder:
   ```bash
   cargo install bootimage
   ```

## How to Run

1. Clone this repository.
2. Build and run the OS in QEMU:
   ```bash
   cargo run
   ```

## Creating a Bootable USB (Real Hardware)
After running `cargo build` or `cargo bootimage`, the compiled disk image will be located at `target/x86_64-veros/debug/bootimage-veros.bin`. You can flash this `.bin` file to a USB flash drive using tools like **Rufus** or **balenaEtcher**, plug it into a computer, and boot directly into Veros!
