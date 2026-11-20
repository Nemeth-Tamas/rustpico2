
# rustpico2 development tasks
# Host: Windows PowerShell
# Firmware: RP2350 Cortex-M33, no_std

set shell := ["powershell.exe", "-NoProfile", "-Command"]

# List available commands
default:
    @just --list

# Format all Rust source
fmt:
    cargo fmt --all

# Check formatting and firmware compilation
check:
    cargo fmt --all -- --check
    cargo check --release --offline

# Build firmware and generate the UF2 image
# Optionally enable a named Cargo feature.
build features="":
    cargo build --release --offline {{if features == "" {""} else {"--features " + features}}}
    rustc --edition=2024 -O .\tools\elf2uf2.rs -o .\tools\elf2uf2.exe
    .\tools\elf2uf2.exe .\target\thumbv8m.main-none-eabihf\release\rustpico2 .\rustpico2.uf2

# Build and flash to a Pico 2 in BOOTSEL mode
flash features="": (build features)
    $volumes = @(Get-Volume -FileSystemLabel 'RP2350' -ErrorAction SilentlyContinue | Where-Object { $_.DriveLetter }); if ($volumes.Count -ne 1) { throw "Expected exactly one RP2350 BOOTSEL drive; found $($volumes.Count). Hold BOOTSEL while connecting the Pico 2." }; $destination = ('{0}:\rustpico2.uf2' -f $volumes[0].DriveLetter); Copy-Item -LiteralPath '.\rustpico2.uf2' -Destination $destination -ErrorAction Stop; Write-Host "UF2 copied to $destination"

# Remove generated files
clean:
    cargo clean
    Remove-Item -LiteralPath '.\rustpico2.uf2' -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath '.\tools\elf2uf2.exe' -ErrorAction SilentlyContinue

# Show current Git state
status:
    git status --short --branch

# Commit all changes and push to master
push message:
    git add -A
    git commit -m "{{message}}"
    git push origin master
