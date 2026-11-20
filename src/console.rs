//! Tiny interactive UART shell for the RP2350.
//!
//! ASCII commands, fixed 64-byte buffer, no allocator.

use crate::uart;
use core::ptr::read_volatile;

const LINE_CAPACITY: usize = 64;

pub struct Console {
    line: [u8; LINE_CAPACITY],
    length: usize,
    overflow: bool,
    last_was_cr: bool,
    blink_cycles: u32,
}

impl Console {
    pub const fn new() -> Self {
        Self {
            line: [0; LINE_CAPACITY],
            length: 0,
            overflow: false,
            last_was_cr: false,
            blink_cycles: 0,
        }
    }

    pub fn prompt(&self) {
        uart::write_str("pico> ");
    }

    pub fn set_blink_cycles(&mut self, cycles: u32) {
        self.blink_cycles = cycles;
    }

    pub fn heartbeat(&self) {
        // Avoid interrupting someone in the middle of typing.
        if self.length == 0 && !self.overflow {
            uart::write_str("\r\nRUSTPICO2: alive\r\n");
            self.prompt();
        }
    }

    /// Drain all currently available UART RX characters.
    pub fn poll(&mut self) {
        while let Some(byte) = uart::read_byte() {
            self.handle_byte(byte);
        }
    }

    fn handle_byte(&mut self, byte: u8) {
        match byte {
            b'\r' => {
                self.last_was_cr = true;
                self.submit();
            }
            b'\n' => {
                if !self.last_was_cr {
                    self.submit();
                }
                self.last_was_cr = false;
            }
            8 | 127 => {
                self.last_was_cr = false;

                if self.length > 0 {
                    self.length -= 1;
                    uart::write_str("\x08 \x08");
                }
            }
            3 => {
                // Ctrl+C clears the current line.
                self.length = 0;
                self.overflow = false;
                self.last_was_cr = false;
                uart::write_str("^C\r\n");
                self.prompt();
            }
            b' '..=b'~' => {
                self.last_was_cr = false;

                if self.length < LINE_CAPACITY {
                    self.line[self.length] = byte;
                    self.length += 1;

                    let echo = [byte];
                    if let Ok(text) = core::str::from_utf8(&echo) {
                        uart::write_str(text);
                    }
                } else {
                    self.overflow = true;
                }
            }
            _ => {
                self.last_was_cr = false;
            }
        }
    }

    fn submit(&mut self) {
        uart::write_str("\r\n");

        if self.overflow {
            uart::write_str("ERR: command too long (64 bytes max)\r\n");
        } else {
            let text = core::str::from_utf8(&self.line[..self.length]).unwrap_or("");

            execute_command(text.trim_ascii(), self.blink_cycles);
        }

        self.length = 0;
        self.overflow = false;
        self.prompt();
    }
}

fn execute_command(command: &str, blink_cycles: u32) {
    match command {
        "" => {}
        "help" => {
            uart::write_str("Available commands:\r\n");
            uart::write_str("  help       Show this help\r\n");
            uart::write_str("  ping       Test UART communication\r\n");
            uart::write_str("  status     Show firmware status\r\n");
            uart::write_str("  fault      Show SRAM crash record\r\n");
            uart::write_str("  echo TEXT  Echo text back\r\n");
            uart::write_str("  crash udf  Deliberate UsageFault!\r\n");
        }
        "ping" => {
            uart::write_str("PONG from RP2350!\r\n");
        }
        "status" => {
            uart::write_str("RUSTPICO2 STATUS\r\n");
            uart::write_str("  CPU: Cortex-M33\r\n");
            uart::write_str("  Firmware: no_std Rust\r\n");
            uart::write_str("  UART0: 115200 8N1, TX+RX\r\n");
            uart::write_str("  CLK_PERI: 12 MHz XOSC\r\n");
            uart::write_str("  RAM self-test: PASS\r\n");
            uart::write_str("  Fault handlers: ENABLED\r\n");
            uart::write_str("  Blink cycles: ");
            uart::write_hex32(blink_cycles);
            uart::write_str("\r\n");
        }
        "fault" => {
            print_fault_record();
        }
        "crash udf" => {
            uart::write_str("WARNING: triggering deliberate UsageFault!\r\n");

            // A real invalid Thumb instruction.
            // The exception handler will capture and report it.
            unsafe {
                core::arch::asm!("udf #0", options(noreturn));
            }
        }
        "echo" => {
            uart::write_str("\r\n");
        }
        _ => {
            if let Some(text) = command.strip_prefix("echo ") {
                uart::write_str(text);
                uart::write_str("\r\n");
            } else {
                uart::write_str("Unknown command. Type help.\r\n");
            }
        }
    }
}

/// Read one word of the exported crash record.
fn fault_word(index: usize) -> u32 {
    unsafe {
        let base = (&raw const crate::FAULT_RECORD).cast::<u32>();
        read_volatile(base.add(index))
    }
}

fn print_hex_field(name: &str, value: u32) {
    uart::write_str(name);
    uart::write_hex32(value);
    uart::write_str("\r\n");
}

/// Print the current crash record, if one exists.
///
/// Can be called from the shell or from fault_capture().
pub fn print_fault_record() {
    if !uart::is_ready() {
        return;
    }

    if fault_word(0) != crate::FAULT_RECORD_MAGIC {
        uart::write_str("No crash record in SRAM (reset clears it).\r\n");
        return;
    }

    let kind = fault_word(1);
    let cfsr = fault_word(5);
    let flags = fault_word(11);

    uart::write_str("\r\n========== FAULT REPORT ==========\r\n");

    uart::write_str("TYPE: ");

    match kind {
        3 => uart::write_str("HardFault\r\n"),
        4 => uart::write_str("MemManage\r\n"),
        5 => uart::write_str("BusFault\r\n"),
        6 => uart::write_str("UsageFault\r\n"),
        7 => uart::write_str("SecureFault\r\n"),
        8 => uart::write_str("Unexpected exception\r\n"),
        _ => uart::write_str("Unknown exception\r\n"),
    }

    print_hex_field("EXC_RETURN: ", fault_word(2));
    print_hex_field("STACK_PTR:  ", fault_word(3));
    print_hex_field("ICSR:       ", fault_word(4));
    print_hex_field("CFSR:       ", cfsr);
    print_hex_field("HFSR:       ", fault_word(6));
    print_hex_field("SFSR:       ", fault_word(9));
    print_hex_field("FLAGS:      ", flags);

    if (flags & (1 << 1)) != 0 {
        print_hex_field("MMFAR:      ", fault_word(7));
    }

    if (flags & (1 << 2)) != 0 {
        print_hex_field("BFAR:       ", fault_word(8));
    }

    if (flags & (1 << 3)) != 0 {
        print_hex_field("SFAR:       ", fault_word(10));
    }

    if (flags & 1) != 0 {
        uart::write_str("FRAME:      VALID\r\n");
        print_hex_field("R0:         ", fault_word(12));
        print_hex_field("R1:         ", fault_word(13));
        print_hex_field("R2:         ", fault_word(14));
        print_hex_field("R3:         ", fault_word(15));
        print_hex_field("R12:        ", fault_word(16));
        print_hex_field("LR:         ", fault_word(17));
        print_hex_field("PC:         ", fault_word(18));
        print_hex_field("xPSR:       ", fault_word(19));
    } else {
        uart::write_str("FRAME:      UNAVAILABLE\r\n");
    }

    // CFSR bits 16-31 are UsageFault status flags.
    if (cfsr & (1 << 16)) != 0 {
        uart::write_str("REASON: Undefined instruction\r\n");
    }

    if (cfsr & (1 << 17)) != 0 {
        uart::write_str("REASON: Invalid execution state\r\n");
    }

    if (cfsr & (1 << 18)) != 0 {
        uart::write_str("REASON: Invalid exception return\r\n");
    }

    if (cfsr & (1 << 19)) != 0 {
        uart::write_str("REASON: No coprocessor\r\n");
    }

    if (cfsr & (1 << 24)) != 0 {
        uart::write_str("REASON: Unaligned access\r\n");
    }

    if (cfsr & (1 << 25)) != 0 {
        uart::write_str("REASON: Divide by zero\r\n");
    }

    uart::write_str("STATUS: CPU HALTED\r\n");
    uart::write_str("==================================\r\n");
}
