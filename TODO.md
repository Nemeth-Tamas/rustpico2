# rustpico2 — Bare-Metal RP2350

Target: Raspberry Pi Pico 2 (non-W)
Chip: RP2350
Architecture: Cortex-M33 initially
Firmware: Rust no_std, no HAL, no SDK

## Rules

- GitHub master is the source of truth.
- Fetch exact current source from GitHub before every edit.
- Use exact FIND and REPLACE blocks, not whole-file replacements.
- No leading or trailing blank lines inside code blocks.
- No third-party firmware dependencies.
- Host development dependencies are allowed.
- No vendor SDK, HAL, Embassy, or Zephyr.
- Build in small milestones.
- Every milestone is committed and pushed,
  including broken or experimental versions.
- Update this TODO for every source change.

## Milestone 1 — First Boot and Blink

- [x] Create Cargo project structure.
- [x] Configure Cortex-M33 build target.
- [x] Configure offline Cargo builds.
- [x] Create custom linker script.
- [x] Create ARM vector table.
- [x] Create RP2350 IMAGE_DEF.
- [x] Create reset entry point.
- [x] Clear MSPLIM before Rust entry.
- [x] Implement volatile MMIO.
- [x] Configure GPIO25.
- [x] Implement onboard LED blink loop.
- [x] Create standalone Rust ELF-to-UF2 converter.
- [x] Compile firmware on development PC.
- [x] Generate UF2.
- [x] Flash Pico 2.
- [x] Confirm LED blinks.
- [x] Push initial working firmware to GitHub.

Hardware confirmation: November 20, 2026.
First bare-metal RP2350 firmware successfully executed.

## Milestone 1.1 — Development Automation

- [x] Choose Just task runner.
- [x] Add Windows PowerShell recipes.
- [x] Automate Cargo firmware builds.
- [x] Automate Rust UF2 converter compilation.
- [x] Automate ELF-to-UF2 conversion.
- [x] Add automatic RP2350 BOOTSEL drive detection.
- [x] Add automated UF2 copying.
- [x] Add formatting and check commands.
- [x] Add cleanup command.
- [x] Add Git commit/push command.
- [x] Correct Cargo release profile typo.
- [x] Verify Just recipes on Windows.
- [x] Test automatic flashing.
- [x] Commit and push Milestone 1.1.

Automation confirmed working on November 20, 2026.

## Development Commands

Install task runner:

    scoop install just

Available commands:

    just
    just fmt
    just check
    just build
    just flash
    just clean
    just ports
    just monitor COM7
    just status
    just push "commit message"

Replace COM7 with the actual serial adapter port.

UART monitoring uses Windows PowerShell's built-in
System.IO.Ports.SerialPort implementation.

Fault-injection builds:

    just flash fault-test-panic
    just flash fault-test-usage

Always restore normal firmware after fault testing:

    just flash

Firmware builds use Cargo offline mode.

Host development tools may use external dependencies.
Target firmware remains no_std and dependency-free.

## Milestone 2 — Startup Improvements

### Milestone 2A — RAM Initialization

- [x] Implement .data copying from flash to RAM.
- [x] Implement .bss zero initialization.
- [x] Add initialized and zero-initialized RAM test variables.
- [x] Add volatile RAM verification.
- [x] Add solid-LED failure indicator.
- [x] Assert vector table size and IMAGE_DEF offset.
- [x] Assert word alignment of RAM initialization regions.
- [x] Build and test on physical Pico 2.
- [x] Confirm normal LED blink after RAM self-test.
- [x] Commit and push Milestone 2A.

Hardware confirmation: November 20, 2026.
RAM startup self-test passed and normal LED blinking confirmed.

### Milestone 2B — Fault Handling and Stack Safety

- [x] Reserve a 4 KiB main stack.
- [x] Add linker assertion against stack overlap.
- [x] Configure Cortex-M33 MSPLIM.
- [x] Define dedicated system fault vectors.
- [x] Enable configurable CPU fault exceptions.
- [x] Implement panic and exception LED diagnostics.
- [x] Add optional Rust panic fault injection.
- [x] Add optional CPU UsageFault injection.
- [x] Extend Just build and flash commands with features.
- [x] Compile and flash normal firmware.
- [x] Verify normal LED blink.
- [x] Verify intentional panic gives 2 flashes.
- [x] Verify intentional UsageFault gives 6 flashes.
- [x] Restore normal firmware and confirm blinking.
- [x] Commit and push Milestone 2B.

Hardware confirmation: November 20, 2026.
Normal operation and deliberate fault injection both passed.

### Milestone 2C — Advanced Fault Diagnostics

#### Milestone 2C.1 — SRAM Fault Recorder

- [x] Add ARM exception-entry assembly stubs.
- [x] Capture original MSP/PSP and EXC_RETURN.
- [x] Capture CFSR, HFSR, MMFAR, BFAR, SFSR, SFAR.
- [x] Capture ICSR active exception information.
- [x] Validate basic exception stack frame.
- [x] Capture R0-R3, R12, LR, PC and xPSR.
- [x] Allocate a 20-word SRAM fault record.
- [x] Export FAULT_RECORD for future debugging.
- [x] Add record validity flags and magic marker.
- [x] Add verified UDF fault LED detail pattern.
- [x] Build and test normal firmware.
- [x] Confirm normal blinking.
- [x] Confirm Rust panic still gives 2 flashes.
- [x] Confirm UDF gives 6 flashes plus 1 long flash.
- [x] Restore normal firmware.
- [x] Commit and push Milestone 2C.1.

Hardware confirmation: November 20, 2026.
The deliberate UsageFault generated six short LED flashes
followed by one long flash. Fault frame verification passed.

FAULT_RECORD is an exported 20-word array in SRAM.
Magic word: 0xFA172350.
The record is cleared during normal startup, so it does
not survive reboot or reflashing.

Word offsets:
 0: magic
 1: exception type
 2: EXC_RETURN
 3: exception stack pointer
 4: ICSR
 5: CFSR
 6: HFSR
 7: MMFAR
 8: BFAR
 9: SFSR
10: SFAR
11: validity flags
12: R0
13: R1
14: R2
15: R3
16: R12
17: LR
18: PC
19: xPSR

Validity flags:
 bit 0: basic exception frame captured
 bit 1: MMFAR valid
 bit 2: BFAR valid
 bit 3: SFAR valid

A verified undefined-instruction UsageFault produces
6 normal LED flashes followed by 1 longer detail flash.

#### Milestone 2C.2 — Fault Reporting

- [x] Implement initial fault-register decoding helpers.
- [x] Implement the fault command for SRAM record inspection.
- [x] Implement automatic UART exception diagnostics.
- [x] Verify printed fault report on physical hardware.

Hardware confirmation: November 20, 2026.
UART reported UsageFault with CFSR=0x00010000,
PC=0x1000081C and a valid exception frame.
- [ ] Investigate persistent crash records.
- [ ] Explore robust stack-overflow recovery.
- [ ] Support extended exception frames.
- [ ] Add faulting PC to source-line lookup.

## Milestone 3 — UART

### Milestone 3A — UART0 Transmitter

- [x] Create src/uart.rs with no external crates.
- [x] Implement volatile UART register access.
- [x] Configure Pico 2 12 MHz crystal oscillator.
- [x] Configure CLK_PERI from XOSC.
- [x] Configure GPIO0 for UART0 TX.
- [x] Release UART0 from reset.
- [x] Configure UART0 for 115200 baud, 8N1.
- [x] Implement bounded transmit FIFO polling.
- [x] Implement UART string output.
- [x] Add startup serial banner.
- [x] Add periodic heartbeat output.
- [x] Add UART initialization failure LED code 9.
- [x] Add Just serial port discovery.
- [x] Add Just serial monitor recipe.
- [x] Compile normal firmware.
- [x] Flash and confirm LED blinking.
- [x] Verify UART0 output with serial adapter.
- [x] Confirm startup banner.
- [x] Confirm periodic heartbeat.
- [x] Commit and push Milestone 3A.

Hardware confirmation: November 20, 2026.
UART0 transmitter successfully outputs the startup banner
and periodic heartbeat messages at 115200 baud.

Hardware:
  Pico 2 GPIO0, physical pin 1 -> USB-TTL RX
  Pico 2 GND, physical pin 3 -> USB-TTL GND

Serial parameters:
  Baud: 115200
  Data bits: 8
  Parity: none
  Stop bits: 1
  Flow control: none
  Logic level: 3.3 V

The Pico 2 is powered through its own USB connection.
The USB-to-TTL adapter is for external UART monitoring only.
Do not connect the adapter VCC pin.

Clock configuration:
  XOSC = 12 MHz
  CLK_PERI = XOSC / 1
  UART IBRD = 6
  UART FBRD = 33

The existing CPU clock is not reconfigured.

### Milestone 3B — UART Receiver and Console

- [x] Configure GPIO1 for UART0 RX.
- [x] Enable GPIO1 input buffer and idle pull-up.
- [x] Enable UART0 receiver.
- [x] Implement nonblocking UART receive.
- [x] Detect and discard UART receive errors.
- [x] Implement fixed 64-byte ASCII line buffer.
- [x] Handle Enter, Backspace and CRLF.
- [x] Implement tiny command parser.
- [x] Add help, ping, status and echo commands.
- [x] Add fault record reporting command.
- [x] Add deliberate crash udf command.
- [x] Implement hexadecimal register output.
- [x] Add UART diagnostics to CPU fault handling.
- [x] Add UART message to Rust panic handling.
- [x] Preserve LED blinking while polling RX.
- [x] Prevent heartbeat messages while typing.
- [x] Create bidirectional PowerShell serial monitor.
- [x] Integrate serial monitor with Just.
- [x] Build and flash normal firmware.
- [x] Verify normal LED blinking.
- [x] Verify interactive shell prompt.
- [x] Test help, ping and echo.
- [ ] Explicitly verify status output.
- [x] Verify fault command reports no previous crash.
- [x] Trigger crash udf and inspect UART fault report.
- [ ] Confirm six-short/one-long LED pattern after shell crash.
- [ ] Explicitly confirm restored normal firmware.
- [x] Commit and push Milestone 3B.

Hardware confirmation: November 20, 2026.
Bidirectional UART console and on-demand crash reporting passed.

Hardware:
  GPIO0 (physical pin 1) -> USB-TTL RX
  GPIO1 (physical pin 2) -> USB-TTL TX
  GND   (physical pin 3) -> USB-TTL GND

USB-TTL logic must be 3.3 V.
Do not connect the USB-TTL VCC pin.
Power the Pico 2 through its own USB connector.

Interactive commands:
  help
  ping
  status
  fault
  echo TEXT
  crash udf

The crash udf command intentionally causes a UsageFault.
The fault handler prints the SRAM crash record over UART,
then enters its repeating LED fault pattern.

The SRAM crash record is not persistent across resets.
The fault command on a fresh boot should report no record.

### Milestone 3C.1 — Hardware Timer and Nonblocking Main Loop

- [x] Create src/timer.rs.
- [x] Configure CLK_REF to use 12 MHz XOSC.
- [x] Configure the dedicated TIMER0 tick generator.
- [x] Release TIMER0 from reset.
- [x] Verify TIMER0 advances during initialization.
- [x] Implement coherent 64-bit microsecond reads.
- [x] Add millisecond and second conversion functions.
- [x] Implement decimal output without allocation.
- [x] Add the uptime console command.
- [x] Add uptime reporting to status.
- [x] Replace normal-operation software delays.
- [x] Implement hardware-timed 1 Hz LED blinking.
- [x] Implement 8-second heartbeat scheduling.
- [x] Preserve UART polling during normal operation.
- [x] Retain independent fault LED delay fallback.
- [x] Add LED fault code 10 for timer initialization failure.
- [ ] Verify compilation.
- [ ] Verify TIMER0 initialization on physical hardware.
- [ ] Verify accurate LED blinking.
- [ ] Verify uptime increases correctly.
- [ ] Verify status displays hardware uptime.
- [ ] Verify UART commands remain responsive.
- [ ] Verify fault reporting still works.
- [ ] Commit and push Milestone 3C.1.

Hardware timer configuration:
  TIMER0 base: 0x400B0000
  TICKS base:  0x40108000
  CLK_REF:     12 MHz XOSC
  Tick divider: 12
  Timer frequency: 1 MHz
  Timer resolution: 1 microsecond

The uptime counter starts during firmware initialization.
Normal operation polls the hardware timebase.
Interrupt-driven scheduling is a future milestone.

### Milestone 3C.2 — Advanced Console

- [ ] Add arbitrary safe register inspection commands.
- [ ] Add memory map and RAM usage reporting.
- [ ] Add numerical argument parsing.
- [ ] Implement interrupt-driven UART RX.
- [ ] Add RX ring buffer and overrun statistics.
- [ ] Add hardware timer-based uptime.
- [ ] Add command history and basic editing.
- [ ] Add a controlled software reboot command.
- [ ] Add faulting PC to source-line lookup.

## Milestone 4 — Hardware Time

- [x] Implement TIMER0 microsecond timebase.
- [x] Implement 64-bit uptime counter.
- [x] Implement hardware-timed LED scheduling.
- [x] Implement hardware-timed heartbeat scheduling.
- [ ] Verify actual timing on physical hardware.
- [ ] Implement TIMER0 alarm interrupts.
- [ ] Configure NVIC for timer interrupts.
- [ ] Implement interrupt-driven scheduling.
- [ ] Add low-power WFI idle support.
- [ ] Implement timer-driven software tasks.
- [ ] Investigate repeating and one-shot timers.

## Milestone 5 — Peripherals

- [ ] PIO.
- [ ] PWM.
- [ ] SPI.
- [ ] I2C.
- [ ] DMA.
- [ ] Native USB.

## Milestone 6 — Multicore

- [ ] Launch Cortex-M33 core 1.
- [ ] Inter-core FIFO.
- [ ] Shared memory.
- [ ] Synchronization primitives.

## Milestone 7 — RISC-V

- [ ] Investigate Hazard3 boot path.
- [ ] Build native RISC-V firmware.
- [ ] Compare ARM and RISC-V implementations.
- [ ] Explore dual-architecture operation.

## Long-Term Goal

Create an experimental, entirely homemade
RP2350 operating environment with our own
drivers, scheduler, and command interface.

No borrowed runtime.
No vendor hardware abstraction.
Just Rust, hardware registers, and pain.
