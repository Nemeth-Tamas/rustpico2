//! Bare-metal RP2350 TIMER0 driver.
//!
//! TIMER0 is a free-running 64-bit microsecond counter.
//! TICKS_TIMER0 divides the 12 MHz crystal reference by 12.
//!
//! No SDK, HAL, allocation or external crates.

use core::hint::spin_loop;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicU32, Ordering};

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

// TIMER0 alarm 0 and interrupt registers.
const TIMER0_ALARM0: usize = TIMER0_BASE + 0x10;
const TIMER0_INTR: usize = TIMER0_BASE + 0x3C;
const TIMER0_INTE: usize = TIMER0_BASE + 0x40;
const TIMER0_INTS: usize = TIMER0_BASE + 0x48;

// Cortex-M33 NVIC: external IRQ 0 = TIMER0_IRQ_0.
const NVIC_ISER0: usize = 0xE000_E100;
const NVIC_ICER0: usize = 0xE000_E180;
const NVIC_ICPR0: usize = 0xE000_E280;

// Cortex-M33 System Control Register.
const SCB_SCR: usize = 0xE000_ED10;
const SCB_SLEEPDEEP: u32 = 1 << 2;

const ALARM0_BIT: u32 = 1;
const ALARM_INTERVAL_US: u32 = 1_000;

// Updated only by the hardware interrupt handler.
static ALARM0_COUNT: AtomicU32 = AtomicU32::new(0);

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

/// Number of hardware alarm interrupts observed.
///
/// This is a wrapping 32-bit counter. At approximately
/// 1000 interrupts per second it wraps after ~49.7 days.
pub fn irq_count() -> u32 {
    ALARM0_COUNT.load(Ordering::Relaxed)
}

/// Configure TIMER0 alarm 0 to interrupt roughly every 1 ms.
///
/// The first interrupt must arrive within 50 ms or the
/// initialization is considered unsuccessful.
pub fn start_alarm0_irq() -> bool {
    ALARM0_COUNT.store(0, Ordering::Relaxed);

    // Disable IRQ0 while preparing its hardware source.
    write32(NVIC_ICER0, ALARM0_BIT);

    // Disable only TIMER0 alarm 0, preserving other alarms.
    let enabled = read32(TIMER0_INTE);
    write32(TIMER0_INTE, enabled & !ALARM0_BIT);

    // Clear any previously latched alarm interrupt.
    write32(TIMER0_INTR, ALARM0_BIT);

    // Remove any pending NVIC IRQ0 request.
    write32(NVIC_ICPR0, ALARM0_BIT);

    // Ensure WFI uses ordinary sleep rather than deep sleep.
    let sleep_control = read32(SCB_SCR);
    write32(SCB_SCR, sleep_control & !SCB_SLEEPDEEP);

    // Program the first one-shot alarm 1 ms into the future.
    let first_deadline = read32(TIMER0_TIMELR).wrapping_add(ALARM_INTERVAL_US);

    write32(TIMER0_ALARM0, first_deadline);

    // Enable alarm 0 at the TIMER0 peripheral.
    write32(TIMER0_INTE, enabled | ALARM0_BIT);

    // Enable external interrupt 0 in the NVIC.
    write32(NVIC_ISER0, ALARM0_BIT);

    // Explicitly enable normal maskable interrupts.
    unsafe {
        core::arch::asm!("cpsie i", options(nomem, nostack, preserves_flags));
    }

    // Verify the actual exception handler executes.
    let start = micros();

    while micros().wrapping_sub(start) < 50_000 {
        if irq_count() != 0 {
            return true;
        }

        spin_loop();
    }

    // If no interrupt arrived, disable the source.
    write32(NVIC_ICER0, ALARM0_BIT);
    write32(TIMER0_INTE, enabled & !ALARM0_BIT);

    false
}

/// Actual Cortex-M33 interrupt service routine.
///
/// Called by vector table entry 16 (external IRQ 0).
/// Keep this handler short: no serial printing, no
/// allocation, no waiting for locks or other events.
#[unsafe(no_mangle)]
pub extern "C" fn timer0_irq0_handler() {
    if (read32(TIMER0_INTS) & ALARM0_BIT) == 0 {
        return;
    }

    // Clear the latched interrupt (write-one-to-clear).
    write32(TIMER0_INTR, ALARM0_BIT);

    // Alarm 0 is one-shot. Rearm it from the current
    // hardware time for approximately 1 ms later.
    let next_deadline = read32(TIMER0_TIMELR).wrapping_add(ALARM_INTERVAL_US);

    write32(TIMER0_ALARM0, next_deadline);

    // Record a successful interrupt.
    ALARM0_COUNT.fetch_add(1, Ordering::Relaxed);
}
