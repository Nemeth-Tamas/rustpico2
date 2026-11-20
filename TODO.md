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
    just status
    just push "commit message"

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
- [ ] Compile and flash normal firmware.
- [ ] Verify normal LED blink.
- [ ] Verify intentional panic gives 2 flashes.
- [ ] Verify intentional UsageFault gives 6 flashes.
- [ ] Restore normal firmware and confirm blinking.
- [ ] Commit and push Milestone 2B.

### Milestone 2C — Advanced Fault Diagnostics

- [ ] Capture CFSR and HFSR fault status registers.
- [ ] Capture faulting PC and stacked CPU registers.
- [ ] Add distinguishable fault subcodes.
- [ ] Explore robust stack-overflow recovery.
- [ ] Add panic and exception output over UART.

## Milestone 3 — UART

- [ ] Configure system clocks.
- [ ] Initialize UART0 using MMIO.
- [ ] Print first boot banner.
- [ ] Implement UART transmit.
- [ ] Implement UART receive.
- [ ] Add a minimal serial command interface.

## Milestone 4 — Hardware Time

- [ ] Hardware timers.
- [ ] Interrupt handling.
- [ ] NVIC configuration.
- [ ] Accurate delays.
- [ ] Uptime counter.

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
