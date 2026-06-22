#Requires -Version 7
#
# install.ps1 — build, install, and PATH-register `box` for the current user.
#
# What it does (all user-scope, no admin / UAC):
#   1. Builds the release binary with the MSVC ABI + static CRT so the .exe is
#      portable across Windows 10/11 with no redistributable.
#   2. Copies box.exe to %LOCALAPPDATA%\Programs\box (the per-user installed-app
#      convention; the install dir IS the PATH entry).
#   3. Adds that dir to the *user* PATH idempotently, preserving REG_EXPAND_SZ
#      when the existing PATH contains %VARS% (so other apps' %VAR% entries are
#      not silently expanded/downgraded).
#   4. Refreshes the live $env:Path from BOTH User and Machine scopes so `box`
#      works in THIS same PowerShell 7 session (and System32 etc. are not lost).
#   5. Smoke-tests `box --help` and reports readiness.
#
# Re-running is safe: the copy is a plain overwrite and the PATH add is
# dedup-guarded, so the entry is never duplicated.

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$BinDir = Join-Path $env:LOCALAPPDATA 'Programs\box'                 # D-01
$ExeSrc = '.\target\x86_64-pc-windows-msvc\release\box.exe'
$ExeDst = Join-Path $BinDir 'box.exe'

# 1. Build (crt-static, MSVC) — CLAUDE.md "Release Build / Static Linking"
Write-Host "Building release binary (x86_64-pc-windows-msvc, +crt-static)..."
$env:RUSTFLAGS = '-C target-feature=+crt-static'
cargo build --release --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw "cargo build failed (exit $LASTEXITCODE)" }
if (-not (Test-Path -LiteralPath $ExeSrc)) { throw "build succeeded but $ExeSrc was not produced" }

# 2. Copy (plain overwrite — D-04)
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
Copy-Item -Force -LiteralPath $ExeSrc -Destination $ExeDst
Write-Host "Installed to $ExeDst"

# 3. Idempotent user-PATH update (D-02), preserving REG_EXPAND_SZ if %VARS% present.
#    Read the RAW (un-expanded) value so the dedup check and the type decision
#    are correct — GetEnvironmentVariable/Get-ItemPropertyValue would expand %VARS%.
$key     = 'HKCU:\Environment'
$rawPath = (Get-Item -Path $key).GetValue('Path', '', 'DoNotExpandEnvironmentNames')
$entries = $rawPath -split ';' | Where-Object { $_ -ne '' }
if ($entries -inotcontains $BinDir) {
    $newPath = (@($entries) + $BinDir) -join ';'
    if ($rawPath -match '%') {
        # %VARS% present — write as ExpandString to avoid the REG_EXPAND_SZ -> REG_SZ
        # downgrade that would corrupt other apps' %VAR% PATH entries (Pitfall 3).
        Set-ItemProperty -Path $key -Name 'Path' -Value $newPath -Type ExpandString
    } else {
        # Purely literal paths — the [Environment] user-scope write is sufficient.
        [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
    }
    Write-Host "Added $BinDir to user PATH"
} else {
    Write-Host "$BinDir already in user PATH — skipped"
}

# 4. Refresh current session — merge User + Machine so System32 etc. survive (D-03).
$env:Path = [Environment]::GetEnvironmentVariable('Path', 'User') + ';' +
            [Environment]::GetEnvironmentVariable('Path', 'Machine')

# 5. Smoke test (FOUND-08) — confirm `box` resolves and runs in THIS session.
& box --help | Out-Null
if ($LASTEXITCODE -eq 0) {
    Write-Host "box is ready. Try: box --help"
} else {
    Write-Warning "Installed, but 'box' did not run in this session. Open a new terminal and try 'box --help'."
}
