# rustpico2 — Bare-Metal RP2350

Target: Raspberry Pi Pico 2 (non-W)
Chip: RP2350
Architecture: Cortex-M33 initially
Firmware: Rust no_std, no HAL, no SDK

## Rules

- GitHub master is the source of truth.
- Full-file replacements for every code change.
- No third-party firmware dependencies.
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
- [ ] Compile firmware on development PC.
- [ ] Generate UF2.
- [ ] Flash Pico 2.
- [ ] Confirm LED blinks.
- [ ] Commit and push tested results.

## Milestone 2 — Startup Improvements

- [ ] Initialize .data from flash.
- [ ] Clear .bss.
- [ ] Add stack layout and guards.
- [ ] Implement panic and fault diagnostics.
- [ ] Verify vector table and memory map.

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