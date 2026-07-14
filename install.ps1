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
#   6. Prints a PS7 completion-registration HINT by default; with
#      -RegisterCompletions it idempotently appends the `box completions` recipe
#      to the user's $PROFILE (D-11 — never an unprompted profile edit).
#
# Re-running is safe: the copy is a plain overwrite, the PATH add is dedup-guarded,
# and the -RegisterCompletions $PROFILE append is sentinel-guarded — so no entry is
# ever duplicated.

# D-11: opt-in completion registration. WITHOUT this switch install.ps1 only PRINTS
# a hint and never touches $PROFILE (unprompted profile edits are a far-reaching
# change). The param block must be the first executable statement in the script.
param(
    [switch]$RegisterCompletions
)

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

# 6. PS7 completion registration (CMP-01 / D-11). By DEFAULT just print a hint —
#    NEVER silently mutate $PROFILE. `-RegisterCompletions` opts in to an idempotent
#    append of the LIVE-command form (so completions regenerate each shell start and
#    auto-track future `box` upgrades). The `# box completions` sentinel guard
#    mirrors the step-3 PATH dedup idiom.
$oneliner = 'box completions powershell | Out-String | Invoke-Expression'
if (-not $RegisterCompletions) {
    Write-Host "To enable tab-completion, add $oneliner to your `$PROFILE — or re-run with -RegisterCompletions."
} else {
    # Ensure $PROFILE and its parent dir exist (same New-Item -Force idiom as step 2).
    $profileDir = Split-Path -Parent $PROFILE
    New-Item -ItemType Directory -Force -Path $profileDir | Out-Null
    if (-not (Test-Path -LiteralPath $PROFILE)) {
        New-Item -ItemType File -Force -Path $PROFILE | Out-Null
    }
    # Idempotency guard: skip the append if the `# box completions` sentinel is
    # already present (re-running -RegisterCompletions never duplicates the block).
    if (-not (Select-String -Quiet -Pattern '# box completions' -Path $PROFILE)) {
        Add-Content -LiteralPath $PROFILE -Value ''
        Add-Content -LiteralPath $PROFILE -Value '# box completions'
        Add-Content -LiteralPath $PROFILE -Value $oneliner
        Write-Host "Registered box completions in $PROFILE"
    } else {
        Write-Host "box completions already registered — skipped"
    }
}
