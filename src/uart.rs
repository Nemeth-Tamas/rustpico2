//! Bare-metal RP2350 UART0 transmitter and receiver.
//!
//! GPIO0 (physical pin 1): UART0 TX
//! GPIO1 (physical pin 2): UART0 RX
//! Clock: Pico 2 XOSC, 12 MHz
//! Baud: 115200, 8N1
//!
//! No HAL, no external crates, no allocation.

use core::hint::spin_loop;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicBool, Ordering};

// RP2350 clock and oscillator registers.
const XOSC_CTRL: usize = 0x4004_8000;
const XOSC_STATUS: usize = 0x4004_8004;
const XOSC_STARTUP: usize = 0x4004_800C;

const CLK_PERI_CTRL: usize = 0x4001_0048;
const CLK_PERI_DIV: usize = 0x4001_004C;

// RP2350 reset controller.
const RESETS_SET: usize = 0x4002_2000;
const RESETS_CLR: usize = 0x4002_3000;
const RESETS_DONE: usize = 0x4002_0008;

// UART0 reset bit.
const RESET_UART0: u32 = 1 << 26;

// GPIO0 TX and GPIO1 RX control registers.
const GPIO0_CTRL: usize = 0x4002_8004;
const GPIO1_CTRL: usize = 0x4002_800C;
const GPIO0_PAD: usize = 0x4003_8004;
const GPIO1_PAD: usize = 0x4003_8008;

const GPIO_FUNC_UART: u32 = 2;
const PAD_ISO: u32 = 1 << 8;
const PAD_OD: u32 = 1 << 7;
const PAD_IE: u32 = 1 << 6;
const PAD_PUE: u32 = 1 << 3;
const PAD_PDE: u32 = 1 << 2;

// ARM PL011-compatible UART0 registers.
const UART0_BASE: usize = 0x4007_0000;

const UART_DR: usize = UART0_BASE;
const UART_RSR_ECR: usize = UART0_BASE + 0x004;
const UART_FR: usize = UART0_BASE + 0x018;
const UART_IBRD: usize = UART0_BASE + 0x024;
const UART_FBRD: usize = UART0_BASE + 0x028;
const UART_LCR_H: usize = UART0_BASE + 0x02C;
const UART_CR: usize = UART0_BASE + 0x030;
const UART_ICR: usize = UART0_BASE + 0x044;

const UART_FR_TXFF: u32 = 1 << 5;
const UART_FR_RXFE: u32 = 1 << 4;
const UART_DR_ERROR_MASK: u32 = 0xF << 8;

const UART_CR_UARTEN: u32 = 1 << 0;
const UART_CR_TXE: u32 = 1 << 8;
const UART_CR_RXE: u32 = 1 << 9;

const XOSC_STABLE: u32 = 1 << 31;
const CLK_ENABLED: u32 = 1 << 28;

// RP2350 CLK_PERI AUXSRC = XOSC.
const CLK_PERI_XOSC: u32 = 4 << 5;
const CLK_ENABLE: u32 = 1 << 11;

// 12 MHz / (16 * 115200) = 6.510416...
// Integer divisor = 6
// Fractional divisor = round(0.510416 * 64) = 33
const BAUD_IBRD: u32 = 6;
const BAUD_FBRD: u32 = 33;

// UART 8-bit words, FIFO enabled, no parity, one stop bit.
const UART_8N1_FIFO: u32 = (3 << 5) | (1 << 4);

// UART is usable only after the entire initialization succeeds.
static UART_READY: AtomicBool = AtomicBool::new(false);

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

// Bounded polling avoids hanging forever if a clock fails.
fn wait_bit(address: usize, mask: u32, expected_set: bool) -> bool {
    for _ in 0..1_000_000 {
        let is_set = (read32(address) & mask) != 0;

        if is_set == expected_set {
            return true;
        }

        spin_loop();
    }

    false
}

/// Initialize UART0 TX at 115200 baud.
///
/// Returns false if the crystal, peripheral clock,
/// or UART reset controller fails to become ready.
pub fn init() -> bool {
    UART_READY.store(false, Ordering::Release);

    // Keep UART0 in reset during clock configuration.
    write32(RESETS_SET, RESET_UART0);

    // Start the 12 MHz crystal if the boot ROM has not
    // already enabled it. Do not disturb a stable XOSC.
    if (read32(XOSC_STATUS) & XOSC_STABLE) == 0 {
        // Frequency range: 1 to 15 MHz.
        write32(XOSC_CTRL, 0xAA0);

        // Approximate 1 ms startup delay at 12 MHz.
        write32(XOSC_STARTUP, 47);

        // Magic XOSC enable field 0xFAB plus range 0xAA0.
        write32(XOSC_CTRL, 0x00FA_BAA0);
    }

    if !wait_bit(XOSC_STATUS, XOSC_STABLE, true) {
        return false;
    }

    // Stop clk_peri before switching its auxiliary source.
    write32(CLK_PERI_CTRL, 0);

    if !wait_bit(CLK_PERI_CTRL, CLK_ENABLED, false) {
        return false;
    }

    // Set clk_peri = 12 MHz XOSC / 1.
    write32(CLK_PERI_DIV, 1 << 16);
    write32(CLK_PERI_CTRL, CLK_PERI_XOSC | CLK_ENABLE);

    if !wait_bit(CLK_PERI_CTRL, CLK_ENABLED, true) {
        return false;
    }

    // Release UART0 from reset.
    write32(RESETS_CLR, RESET_UART0);

    if !wait_bit(RESETS_DONE, RESET_UART0, true) {
        return false;
    }

    // Disable UART before programming baud and frame format.
    write32(UART_CR, 0);

    // Configure the baud-rate divisor.
    write32(UART_IBRD, BAUD_IBRD);
    write32(UART_FBRD, BAUD_FBRD);

    // PL011 latches the divisor on the LCR_H write.
    write32(UART_LCR_H, UART_8N1_FIFO);

    // Clear pending UART interrupt status.
    write32(UART_ICR, 0x7FF);

    // Remove GPIO0 isolation and output disable.
    // Also disable pull-up/pull-down on the TX pin.
    let pad = read32(GPIO0_PAD);
    write32(GPIO0_PAD, pad & !(PAD_ISO | PAD_OD | PAD_PUE | PAD_PDE));

    // Connect GPIO0 to UART0 TX.
    write32(GPIO0_CTRL, GPIO_FUNC_UART);

    // Prepare GPIO1 as a UART receive input.
    // Enable the input buffer and hold the idle line HIGH
    // with a weak pull-up when the adapter is disconnected.
    let rx_pad = read32(GPIO1_PAD);
    write32(
        GPIO1_PAD,
        (rx_pad & !(PAD_ISO | PAD_OD | PAD_PDE)) | PAD_IE | PAD_PUE,
    );

    // Connect GPIO1 to UART0 RX.
    write32(GPIO1_CTRL, GPIO_FUNC_UART);

    // Enable UART0, TX and RX.
    write32(UART_CR, UART_CR_UARTEN | UART_CR_TXE | UART_CR_RXE);

    UART_READY.store(true, Ordering::Release);
    true
}

/// Has UART0 completed initialization?
pub fn is_ready() -> bool {
    UART_READY.load(Ordering::Acquire)
}

/// Transmit one byte, with a bounded FIFO wait.
fn write_byte(byte: u8) -> bool {
    if !is_ready() {
        return false;
    }

    for _ in 0..1_000_000 {
        if (read32(UART_FR) & UART_FR_TXFF) == 0 {
            write32(UART_DR, u32::from(byte));
            return true;
        }

        spin_loop();
    }

    false
}

/// Write an ASCII/UTF-8 string without a heap or formatter.
pub fn write_str(message: &str) {
    for &byte in message.as_bytes() {
        if !write_byte(byte) {
            return;
        }
    }
}

/// Read one UART0 byte without blocking.
///
/// Returns None if no byte is ready or the received
/// character has a UART framing, parity, break or
/// overrun error.
pub fn read_byte() -> Option<u8> {
    if !is_ready() {
        return None;
    }

    if (read32(UART_FR) & UART_FR_RXFE) != 0 {
        return None;
    }

    let data = read32(UART_DR);

    if (data & UART_DR_ERROR_MASK) != 0 {
        // Clear the receive error flags.
        write32(UART_RSR_ECR, 0xF);
        return None;
    }

    Some((data & 0xFF) as u8)
}

/// Write an unsigned 64-bit integer in decimal.
///
/// Uses a fixed stack buffer, no allocation and no std.
pub fn write_u64_decimal(mut value: u64) {
    let mut digits = [0u8; 20];
    let mut index = digits.len();

    loop {
        index -= 1;
        digits[index] = b'0' + (value % 10) as u8;
        value /= 10;

        if value == 0 {
            break;
        }
    }

    for &byte in &digits[index..] {
        if !write_byte(byte) {
            return;
        }
    }
}

/// Write a 32-bit integer as eight uppercase hex digits.
/// Example: 0xFA172350
pub fn write_hex32(value: u32) {
    write_str("0x");

    for nibble in (0..8).rev() {
        let digit = ((value >> (nibble * 4)) & 0xF) as u8;

        let character = if digit < 10 {
            b'0' + digit
        } else {
            b'A' + digit - 10
        };

        if !write_byte(character) {
            return;
        }
    }
}
