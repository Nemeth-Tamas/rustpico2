#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;
use core::ptr::{read_volatile, write_volatile};

mod console;
mod timer;
mod uart;

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

// Cortex-M33 System Handler Control and State Register.
const SCB_SHCSR: usize = 0xE000_ED24;

// Enable MemManage, BusFault, UsageFault and SecureFault.
const FAULT_ENABLE_MASK: u32 = (1 << 16) | (1 << 17) | (1 << 18) | (1 << 19);

// Cortex-M33 fault and exception status registers.
const SCB_ICSR: usize = 0xE000_ED04;
const SCB_CFSR: usize = 0xE000_ED28;
const SCB_HFSR: usize = 0xE000_ED2C;
const SCB_MMFAR: usize = 0xE000_ED34;
const SCB_BFAR: usize = 0xE000_ED38;
const SCB_SFSR: usize = 0xE000_EDE4;
const SCB_SFAR: usize = 0xE000_EDE8;

const FAULT_RECORD_MAGIC: u32 = 0xFA17_2350;
const SRAM_START: usize = 0x2000_0000;
const SRAM_END: usize = 0x2008_2000;
const FLASH_START: u32 = 0x1000_0000;
const FLASH_END: u32 = 0x1040_0000;

// Real writable RAM variables used to verify startup initialization.
// The nonzero initializer belongs in .data.
// The zero initializer belongs in .bss.
const BOOT_DATA_MAGIC: u32 = 0x2350_DA7A;
const BOOT_BSS_MAGIC: u32 = 0xA55A_5AA5;

static mut BOOT_DATA_MARKER: u32 = BOOT_DATA_MAGIC;
static mut BOOT_BSS_MARKER: u32 = 0;

// 20 words of crash diagnostics in SRAM.
// Exported by name for future SWD debugging and UART reporting.
//
// [0]  magic, written last after a complete capture
// [1]  exception type
// [2]  EXC_RETURN
// [3]  original exception stack pointer
// [4]  ICSR
// [5]  CFSR
// [6]  HFSR
// [7]  MMFAR
// [8]  BFAR
// [9]  SFSR
// [10] SFAR
// [11] validity flags
// [12] R0
// [13] R1
// [14] R2
// [15] R3
// [16] R12
// [17] stacked LR
// [18] stacked PC
// [19] stacked xPSR
#[unsafe(no_mangle)]
pub static mut FAULT_RECORD: [u32; 20] = [0; 20];

#[cfg(all(feature = "fault-test-panic", feature = "fault-test-usage"))]
compile_error!("Enable only one fault-test feature at a time.");

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

    // ARMv8-M system exception vectors, slots 2 through 15.
    .word Default_Handler     // 2: NMI
    .word HardFault_Entry     // 3: HardFault
    .word MemManage_Entry     // 4: MemManage
    .word BusFault_Entry      // 5: BusFault
    .word UsageFault_Entry    // 6: UsageFault
    .word SecureFault_Entry   // 7: SecureFault

    // Remaining system exceptions, slots 8 through 15.
    .rept 8
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
    // Protect the reserved 4 KiB Main Stack.
    // Hardware faults when MSP descends below this limit.
    ldr r0, =__stack_bottom
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
    movs r0, #8
    b Fault_Entry

    .size Default_Handler, .-Default_Handler

    // Create one tiny entry stub for each CPU fault.
    // r0 carries the fault classification.
    .macro FAULT_VENEER name, number
        .section .text.\name, "ax", %progbits
        .global \name
        .type \name, %function
        .thumb_func
\name:
        movs r0, #\number
        b Fault_Entry
        .size \name, .-\name
    .endm

    FAULT_VENEER HardFault_Entry, 3
    FAULT_VENEER MemManage_Entry, 4
    FAULT_VENEER BusFault_Entry, 5
    FAULT_VENEER UsageFault_Entry, 6
    FAULT_VENEER SecureFault_Entry, 7

    // Shared fault entry.
    // r0 = fault type
    // r1 = original exception stack frame
    // r2 = EXC_RETURN
    .section .text.Fault_Entry, "ax", %progbits
    .global Fault_Entry
    .type Fault_Entry, %function
    .thumb_func

Fault_Entry:
    mov r2, lr

    // EXC_RETURN bit 2 selects MSP or PSP.
    tst lr, #4
    ite eq
    mrseq r1, msp
    mrsne r1, psp

    // Tail-branch into our Rust fault recorder.
    b fault_capture

    .size Fault_Entry, .-Fault_Entry
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

fn enable_fault_handlers() {
    unsafe {
        let current = read_reg(SCB_SHCSR);
        write_reg(SCB_SHCSR, current | FAULT_ENABLE_MASK);
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

// Write one word of the exported SRAM crash record.
#[inline(always)]
fn fault_record_write(index: usize, value: u32) {
    unsafe {
        let base = (&raw mut FAULT_RECORD).cast::<u32>();
        write_volatile(base.add(index), value);
    }
}

// Called directly from the assembly exception entry.
// The stack-frame pointer was captured before Rust entry.
#[unsafe(no_mangle)]
pub extern "C" fn fault_capture(kind: u32, frame: *const u32, exc_return: u32) -> ! {
    // Invalidate the previous record before updating it.
    for index in 0..20 {
        fault_record_write(index, 0);
    }

    // Snapshot the ARM System Control Block registers.
    let (icsr, cfsr, hfsr, mmfar, bfar, sfsr, sfar) = unsafe {
        (
            read_reg(SCB_ICSR),
            read_reg(SCB_CFSR),
            read_reg(SCB_HFSR),
            read_reg(SCB_MMFAR),
            read_reg(SCB_BFAR),
            read_reg(SCB_SFSR),
            read_reg(SCB_SFAR),
        )
    };

    fault_record_write(1, kind);
    fault_record_write(2, exc_return);
    fault_record_write(3, frame as usize as u32);
    fault_record_write(4, icsr);
    fault_record_write(5, cfsr);
    fault_record_write(6, hfsr);
    fault_record_write(7, mmfar);
    fault_record_write(8, bfar);
    fault_record_write(9, sfsr);
    fault_record_write(10, sfar);

    // Only decode a basic eight-word frame.
    // Extended floating-point/security frames require
    // additional handling in a future milestone.
    let frame_address = frame as usize;

    let basic_frame = (exc_return & (1 << 4)) != 0;

    let inside_sram = frame_address >= SRAM_START && frame_address <= SRAM_END - 32;

    let aligned = (frame_address & 3) == 0;

    // Exception stacking failures can make the frame invalid.
    const STACKING_FAILURE_MASK: u32 = (1 << 4) | (1 << 12);

    let stacking_ok = (cfsr & STACKING_FAILURE_MASK) == 0;

    let frame_valid = basic_frame && inside_sram && aligned && stacking_ok;

    let mut flags = 0u32;
    let mut fault_pc = 0u32;

    if frame_valid {
        flags |= 1;

        // The basic ARM exception frame contains:
        // R0, R1, R2, R3, R12, LR, PC, xPSR.
        for index in 0..8 {
            let value = unsafe { read_volatile(frame.add(index)) };

            fault_record_write(12 + index, value);

            if index == 6 {
                fault_pc = value;
            }
        }
    }

    // Validity of fault address registers.
    if (cfsr & (1 << 7)) != 0 {
        flags |= 1 << 1; // MMFAR valid
    }

    if (cfsr & (1 << 15)) != 0 {
        flags |= 1 << 2; // BFAR valid
    }

    if (sfsr & (1 << 6)) != 0 {
        flags |= 1 << 3; // SFAR valid
    }

    fault_record_write(11, flags);

    // Magic is the commit marker: write it last.
    fault_record_write(0, FAULT_RECORD_MAGIC);

    // Special confirmation for the deliberate UDF test:
    // UsageFault + UNDEFINSTR + valid flash PC.
    let verified_undefined_instruction = kind == 6
        && (cfsr & (1 << 16)) != 0
        && frame_valid
        && (FLASH_START..FLASH_END).contains(&fault_pc);

    let detail = u32::from(verified_undefined_instruction);

    // Report the fault over UART before entering the LED loop.
    // The bounded UART transmitter avoids indefinite FIFO waits.
    console::print_fault_record();

    fault_signal(kind, detail)
}

// Primary fault code: regular LED pulses.
// Optional detail: longer pulse after a separating gap.
// This remains best-effort if the stack or GPIO is damaged.
#[inline(never)]
fn fault_signal(code: u32, detail: u32) -> ! {
    initialize_led();

    loop {
        // Primary exception type.
        for _ in 0..code {
            unsafe {
                write_reg(SIO_GPIO_OUT_SET, LED_MASK);
            }

            delay();

            unsafe {
                write_reg(SIO_GPIO_OUT_CLR, LED_MASK);
            }

            delay();
        }

        if detail != 0 {
            // Distinguish details from the main flash group.
            for _ in 0..3 {
                delay();
            }

            for _ in 0..detail {
                unsafe {
                    write_reg(SIO_GPIO_OUT_SET, LED_MASK);
                }

                // Longer flash indicates verified detail.
                for _ in 0..3 {
                    delay();
                }

                unsafe {
                    write_reg(SIO_GPIO_OUT_CLR, LED_MASK);
                }

                delay();
            }
        }

        // Long gap between repeating fault groups.
        for _ in 0..6 {
            delay();
        }
    }
}

// CPU exceptions now enter through assembly and fault_capture().
// Ordinary Rust panics still use the separate panic handler below.

#[unsafe(no_mangle)]
pub extern "C" fn rust_main() -> ! {
    initialize_led();
    enable_fault_handlers();

    if !ram_self_test() {
        // Solid LED indicates RAM initialization failure.
        unsafe {
            write_reg(SIO_GPIO_OUT_SET, LED_MASK);
        }

        loop {
            core::hint::spin_loop();
        }
    }

    // Initialize UART0 once RAM and GPIO are operational.
    if !uart::init() {
        // Nine LED flashes means UART/clock initialization failed.
        fault_signal(9, 0);
    }

    // Start the RP2350 hardware microsecond counter.
    if !timer::init() {
        uart::write_str("ERROR: TIMER0 initialization failed\r\n");

        // Ten LED flashes indicate timer initialization failure.
        fault_signal(10, 0);
    }

    uart::write_str("\r\n");
    uart::write_str("================================\r\n");
    uart::write_str(" RUSTPICO2 - BARE METAL RP2350\r\n");
    uart::write_str("================================\r\n");
    uart::write_str("CPU: ARM Cortex-M33\r\n");
    uart::write_str("Runtime: custom no_std Rust\r\n");
    uart::write_str("Clock: XOSC 12 MHz -> CLK_PERI + CLK_REF\r\n");
    uart::write_str("Timer: TIMER0, 1 MHz hardware timebase\r\n");
    uart::write_str("UART0: 115200 8N1\r\n");
    uart::write_str("UART0 TX: GPIO0\r\n");
    uart::write_str("UART0 RX: GPIO1\r\n");
    uart::write_str("RAM initialization: PASS\r\n");
    uart::write_str("Fault handlers: ENABLED\r\n");
    uart::write_str("Status: BOOT OK\r\n\r\n");

    let mut shell = console::Console::new();
    uart::write_str("Interactive console ready. Type help.\r\n");
    shell.prompt();

    // Optional deliberate crash tests.
    // Normal firmware does not compile either block.

    #[cfg(feature = "fault-test-panic")]
    panic!("Intentional Rust panic: LED code 2");

    #[cfg(feature = "fault-test-usage")]
    unsafe {
        // Undefined ARM instruction: triggers UsageFault
        // when the configurable exception is enabled.
        core::arch::asm!("udf #0", options(noreturn));
    }

    // Hardware-timed event scheduling.
    const LED_HALF_PERIOD_US: u64 = 500_000;
    const HEARTBEAT_PERIOD_US: u64 = 8_000_000;

    let mut blink_cycles = 0u32;
    let mut led_on = false;

    let mut last_led_event = timer::micros();
    let mut last_heartbeat = last_led_event;

    loop {
        // Service serial commands without intentional delays.
        shell.poll();

        let now = timer::micros();

        // Toggle the LED every 500 milliseconds.
        if now.wrapping_sub(last_led_event) >= LED_HALF_PERIOD_US {
            last_led_event = now;
            led_on = !led_on;

            if led_on {
                unsafe {
                    write_reg(SIO_GPIO_OUT_SET, LED_MASK);
                }
            } else {
                unsafe {
                    write_reg(SIO_GPIO_OUT_CLR, LED_MASK);
                }

                // A complete on/off cycle has finished.
                blink_cycles = blink_cycles.wrapping_add(1);
                shell.set_blink_cycles(blink_cycles);
            }
        }

        // Emit a heartbeat approximately every 8 seconds.
        if now.wrapping_sub(last_heartbeat) >= HEARTBEAT_PERIOD_US {
            last_heartbeat = now;
            shell.heartbeat();
        }

        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // A Rust panic does not create a CPU exception frame.
    // It therefore has no hardware fault PC record here.
    uart::write_str("\r\n*** RUST PANIC ***\r\n");
    uart::write_str("No hardware exception frame available.\r\n");
    uart::write_str("Entering LED panic pattern: 2 flashes.\r\n");

    fault_signal(2, 0)
}
