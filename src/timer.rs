//! Bare-metal RP2350 TIMER0 driver.
//!
//! TIMER0 is a free-running 64-bit microsecond counter.
//! TICKS_TIMER0 divides the 12 MHz crystal reference by 12.
//!
//! No SDK, HAL, allocation or external crates.

use core::hint::spin_loop;
use core::ptr::{read_volatile, write_volatile};

// RP2350 clock reference registers.
const CLK_REF_CTRL: usize = 0x4001_0030;
const CLK_REF_DIV: usize = 0x4001_0034;
const CLK_REF_SELECTED: usize = 0x4001_0038;

// RP2350 reset controller.
const RESETS_SET: usize = 0x4002_2000;
const RESETS_CLR: usize = 0x4002_3000;
const RESETS_DONE: usize = 0x4002_0008;

const RESET_TIMER0: u32 = 1 << 23;

// RP2350 dedicated tick generators.
const TICKS_BASE: usize = 0x4010_8000;
const TICKS_TIMER0_CTRL: usize = TICKS_BASE + 0x18;
const TICKS_TIMER0_CYCLES: usize = TICKS_BASE + 0x1C;

const TICKS_ENABLE: u32 = 1 << 0;
const TICKS_RUNNING: u32 = 1 << 1;

// System timer instance 0.
const TIMER0_BASE: usize = 0x400B_0000;

// TIMELR latches the upper half into TIMEHR.
const TIMER0_TIMEHR: usize = TIMER0_BASE + 0x08;
const TIMER0_TIMELR: usize = TIMER0_BASE + 0x0C;

// Clock reference: directly select XOSC.
const CLK_REF_XOSC: u32 = 2;
const CLK_REF_XOSC_SELECTED: u32 = 1 << 2;

// Fixed-point clock divider: integer 1, fraction 0.
const CLK_DIV_ONE: u32 = 1 << 16;

// Pico 2 has a 12 MHz crystal.
const XOSC_CYCLES_PER_US: u32 = 12;

#[inline(always)]
fn read32(address: usize) -> u32 {
    unsafe { read_volatile(address as *const u32) }
}

#[inline(always)]
fn write32(address: usize, value: u32) {
    unsafe {
        write_volatile(address as *mut u32, value);
    }
}

fn wait_for_bits(address: usize, mask: u32) -> bool {
    for _ in 0..1_000_000 {
        if (read32(address) & mask) == mask {
            return true;
        }

        spin_loop();
    }

    false
}

/// Configure TIMER0 to count once per microsecond.
///
/// Must be called after UART initialization, which starts
/// the XOSC if necessary.
///
/// Returns false if clock selection, reset release or the
/// timer tick generator fails.
pub fn init() -> bool {
    // Configure clk_ref = 12 MHz XOSC / 1.
    // SRC is a glitchless clock-source selection field.
    write32(CLK_REF_DIV, CLK_DIV_ONE);
    write32(CLK_REF_CTRL, CLK_REF_XOSC);

    if !wait_for_bits(CLK_REF_SELECTED, CLK_REF_XOSC_SELECTED) {
        return false;
    }

    // Stop the TIMER0 tick generator before changing
    // its cycle divider.
    write32(TICKS_TIMER0_CTRL, 0);

    // One microsecond = 12 cycles of clk_ref.
    write32(TICKS_TIMER0_CYCLES, XOSC_CYCLES_PER_US);

    // Reset TIMER0 to establish a fresh time origin.
    write32(RESETS_SET, RESET_TIMER0);
    write32(RESETS_CLR, RESET_TIMER0);

    if !wait_for_bits(RESETS_DONE, RESET_TIMER0) {
        return false;
    }

    // Start the dedicated 1 MHz tick generator.
    write32(TICKS_TIMER0_CTRL, TICKS_ENABLE);

    if !wait_for_bits(TICKS_TIMER0_CTRL, TICKS_RUNNING) {
        return false;
    }

    // Confirm that the actual hardware counter advances.
    let initial = read32(TIMER0_TIMELR);

    for _ in 0..1_000_000 {
        let current = read32(TIMER0_TIMELR);

        if current.wrapping_sub(initial) >= 5 {
            return true;
        }

        spin_loop();
    }

    false
}

/// Read the 64-bit microsecond counter coherently.
///
/// Reading TIMELR first latches the upper half for TIMEHR,
/// preventing a rollover between the two reads.
#[inline(always)]
pub fn micros() -> u64 {
    let low = read32(TIMER0_TIMELR);
    let high = read32(TIMER0_TIMEHR);

    (u64::from(high) << 32) | u64::from(low)
}

/// Milliseconds since the TIMER0 timebase started.
#[inline(always)]
pub fn millis() -> u64 {
    micros() / 1_000
}

/// Seconds since the TIMER0 timebase started.
#[inline(always)]
pub fn seconds() -> u64 {
    micros() / 1_000_000
}
