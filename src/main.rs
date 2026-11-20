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

// Real writable RAM variables used to verify startup initialization.
// The nonzero initializer belongs in .data.
// The zero initializer belongs in .bss.
const BOOT_DATA_MAGIC: u32 = 0x2350_DA7A;
const BOOT_BSS_MAGIC: u32 = 0xA55A_5AA5;

static mut BOOT_DATA_MARKER: u32 = BOOT_DATA_MAGIC;
static mut BOOT_BSS_MARKER: u32 = 0;

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

    // Copy initialized .data from flash to RAM.
    // r0 = source in flash
    // r1 = destination in RAM
    // r2 = destination end
    ldr r0, =__sidata
    ldr r1, =__sdata
    ldr r2, =__edata

1:
    cmp r1, r2
    bhs 2f

    ldr r3, [r0], #4
    str r3, [r1], #4
    b 1b

2:
    // Clear .bss to zero.
    ldr r1, =__sbss
    ldr r2, =__ebss
    movs r3, #0

3:
    cmp r1, r2
    bhs 4f

    str r3, [r1], #4
    b 3b

4:
    // RAM is initialized. Rust may now safely execute.
    bl rust_main

5:
    b 5b

    // Literal constants used by the address loads.
    .ltorg

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

fn ram_self_test() -> bool {
    unsafe {
        // Check that the initialized value was copied from flash.
        if read_volatile(&raw const BOOT_DATA_MARKER) != BOOT_DATA_MAGIC {
            return false;
        }

        // Check that .bss was cleared to zero.
        if read_volatile(&raw const BOOT_BSS_MARKER) != 0 {
            return false;
        }

        // Verify that the RAM variable is actually writable.
        write_volatile(&raw mut BOOT_BSS_MARKER, BOOT_BSS_MAGIC);

        read_volatile(&raw const BOOT_BSS_MARKER) == BOOT_BSS_MAGIC
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

    if !ram_self_test() {
        // Solid LED indicates RAM initialization failure.
        unsafe {
            write_reg(SIO_GPIO_OUT_SET, LED_MASK);
        }

        loop {
            core::hint::spin_loop();
        }
    }

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
