param(
    [Parameter(Mandatory = $true, Position = 0)]
    [ValidatePattern('^COM\d+$')]
    [string]$Port,

    [int]$Baud = 115200
)

$ErrorActionPreference = "Stop"

$serial = [System.IO.Ports.SerialPort]::new(
    $Port,
    $Baud,
    [System.IO.Ports.Parity]::None,
    8,
    [System.IO.Ports.StopBits]::One
)

$serial.Handshake = [System.IO.Ports.Handshake]::None
$serial.ReadTimeout = 100
$serial.WriteTimeout = 1000
$serial.Encoding = [System.Text.Encoding]::ASCII

$originalCtrlC = [Console]::TreatControlCAsInput

try {
    $serial.Open()
    [Console]::TreatControlCAsInput = $true

    Write-Host "RUSTPICO2 SERIAL TERMINAL" -ForegroundColor Cyan
    Write-Host "Port: $Port | Baud: $Baud | 8N1"
    Write-Host "Type commands directly. Ctrl+C exits."
    Write-Host "----------------------------------------"

    while ($true) {
        # Read all available data from the Pico.
        $incoming = $serial.ReadExisting()

        if ($incoming.Length -gt 0) {
            [Console]::Write($incoming)
        }

        # Check the keyboard without blocking UART input.
        if ([Console]::KeyAvailable) {
            $key = [Console]::ReadKey($true)

            $controlHeld = (
                ($key.Modifiers -band [ConsoleModifiers]::Control) -ne 0
            )

            if ($controlHeld -and $key.Key -eq [ConsoleKey]::C) {
                break
            }

            if ($key.Key -eq [ConsoleKey]::Enter) {
                $serial.Write("`r")
                continue
            }

            if ($key.Key -eq [ConsoleKey]::Backspace) {
                $serial.Write([string][char]8)
                continue
            }

            $character = [int]$key.KeyChar

            if ($character -ge 32 -and $character -le 126) {
                # The Pico itself echoes accepted characters.
                $serial.Write([string]$key.KeyChar)
            }
        }

        Start-Sleep -Milliseconds 5
    }
}
finally {
    [Console]::TreatControlCAsInput = $originalCtrlC

    if ($serial.IsOpen) {
        $serial.Close()
    }

    $serial.Dispose()
    Write-Host "`nSerial terminal closed."
}