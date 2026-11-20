#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;
use core::ptr::{read_volatile, write_volatile};

// RP2350 peripheral addresses.
const RESETS_RESET_CLR: usize = 0x4002_3000;
const RESETS_RESET_DONE: usize = 0x4002_0008;

const IO_BANK0_GPIO25_CTRL: usize = 0x4002_80CC;
const PADS_BANK0_GPIO25: usize = 0x4003_8068;

const SIO_GPIO_OUT_SET: usize = 0xD000_0018;
const SIO_GPIO_OUT_CLR: usize = 0xD000_0020;
const SIO_GPIO_OE_SET: usize = 0xD000_0038;

// Onboard LED: GPIO25.
const LED_MASK: u32 = 1 << 25;

// Reset controller bits.
const RESET_IO_BANK0: u32 = 1 << 6;
const RESET_PADS_BANK0: u32 = 1 << 9;

// GPIO function-select value for SIO.
const GPIO_FUNC_SIO: u32 = 5;

// Pad isolation control.
const PAD_ISO: u32 = 1 << 8;

// ARM Cortex-M33 startup and RP2350 image metadata.
global_asm!(
    r#"
    .syntax unified
    .thumb

    .section .vector_table, "a", %progbits
    .balign 512

    .global __vector_table
__vector_table:
    .word __stack_top
    .word Reset_Handler

    // Remaining Cortex-M exception vectors.
    .rept 14
        .word Default_Handler
    .endr

    // RP2350 external interrupt vectors.
    .rept 52
        .word Default_Handler
    .endr

    .section .image_def, "a", %progbits
    .balign 4

    // RP2350 minimum ARM Secure executable image definition.
    .word 0xffffded3
    .word 0x10210142
    .word 0x000001ff
    .word 0x00000000
    .word 0xab123579

    .section .text.Reset_Handler, "ax", %progbits

    .global Reset_Handler
    .type Reset_Handler, %function
    .thumb_func

Reset_Handler:
    // Disable stack-limit trapping before entering Rust.
    movs r0, #0
    msr MSPLIM, r0

    // Enter our Rust firmware.
    bl rust_main

1:
    b 1b

    .size Reset_Handler, .-Reset_Handler

    .section .text.Default_Handler, "ax", %progbits

    .global Default_Handler
    .type Default_Handler, %function
    .thumb_func

Default_Handler:
    b Default_Handler

    .size Default_Handler, .-Default_Handler
"#
);

#[inline(always)]
unsafe fn write_reg(address: usize, value: u32) {
    unsafe {
        write_volatile(address as *mut u32, value);
    }
}

#[inline(always)]
unsafe fn read_reg(address: usize) -> u32 {
    unsafe { read_volatile(address as *const u32) }
}

fn initialize_led() {
    let reset_mask = RESET_IO_BANK0 | RESET_PADS_BANK0;

    unsafe {
        // Release the GPIO-related peripherals from reset.
        write_reg(RESETS_RESET_CLR, reset_mask);

        while read_reg(RESETS_RESET_DONE) & reset_mask != reset_mask {
            core::hint::spin_loop();
        }

        // Remove GPIO25 pad isolation.
        let pad_config = read_reg(PADS_BANK0_GPIO25);
        write_reg(PADS_BANK0_GPIO25, pad_config & !PAD_ISO);

        // Connect GPIO25 to the SIO peripheral.
        write_reg(IO_BANK0_GPIO25_CTRL, GPIO_FUNC_SIO);

        // Start low, then enable output.
        write_reg(SIO_GPIO_OUT_CLR, LED_MASK);
        write_reg(SIO_GPIO_OE_SET, LED_MASK);
    }
}

#[inline(never)]
fn delay() {
    // Rough delay only. We have not configured a clock or
    // hardware timer yet.
    for _ in 0..1_000_000 {
        core::hint::spin_loop();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_main() -> ! {
    initialize_led();

    loop {
        // LED on.
        unsafe {
            write_reg(SIO_GPIO_OUT_SET, LED_MASK);
        }

        delay();

        // LED off.
        unsafe {
            write_reg(SIO_GPIO_OUT_CLR, LED_MASK);
        }

        delay();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
