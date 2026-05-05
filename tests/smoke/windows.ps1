$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $RepoRoot

cargo build --workspace --all-features

$Bin = Join-Path $RepoRoot "target\debug"
$Tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("axt-smoke-" + [System.Guid]::NewGuid())
New-Item -ItemType Directory -Force -Path (Join-Path $Tmp "project\src") | Out-Null

Set-Content -NoNewline -Path (Join-Path $Tmp "project\Cargo.toml") -Value @"
[package]
name = "axt-smoke"
version = "0.0.0"
edition = "2021"
"@
Set-Content -NoNewline -Path (Join-Path $Tmp "project\src\lib.rs") -Value @"
pub fn answer() -> u8 { 42 }

#[test]
fn answer_is_stable() {
    assert_eq!(answer(), 42);
}
"@
Set-Content -Path (Join-Path $Tmp "project\.gitignore") -Value "ignored.txt"
Set-Content -Path (Join-Path $Tmp "project\ignored.txt") -Value "ignored"

try {
    & (Join-Path $Bin "axt-peek.exe") --json (Join-Path $Tmp "project") | Out-Null
    & (Join-Path $Bin "axt-run.exe") --json --no-save --timeout 5s -- pwsh -NoProfile -Command "Write-Output smoke" | Out-Null
    Push-Location (Join-Path $Tmp "project")
    & (Join-Path $Bin "axt-test.exe") --json --framework cargo | Out-Null
    Pop-Location

    Push-Location (Join-Path $Tmp "project")
    & (Join-Path $Bin "axt-drift.exe") mark --name smoke --hash | Out-Null
    Set-Content -Path "src\changed.txt" -Value "changed"
    & (Join-Path $Bin "axt-drift.exe") --json diff --since smoke --hash | Out-Null
    Pop-Location

    try {
        $Listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Parse("127.0.0.1"), 0)
        $Listener.Start()
        $Port = $Listener.LocalEndpoint.Port
        & (Join-Path $Bin "axt-port.exe") --json free --dry-run $Port | Out-Null
    }
    catch {
        Write-Host "Skipping axt-port smoke: unable to bind local listener"
    }
    finally {
        if ($null -ne $Listener) {
            $Listener.Stop()
        }
    }

    & (Join-Path $Bin "axt-outline.exe") --json (Join-Path $Tmp "project\src\lib.rs") | Out-Null
    & (Join-Path $Bin "axt-slice.exe") --json (Join-Path $Tmp "project\src\lib.rs") --symbol answer | Out-Null
    & (Join-Path $Bin "axt-ctxpack.exe") --json --pattern smoke=answer (Join-Path $Tmp "project\src\lib.rs") | Out-Null
    & (Join-Path $Bin "axt-bundle.exe") --json (Join-Path $Tmp "project") | Out-Null
    "2026-04-28T10:00:00Z error smoke failure" | & (Join-Path $Bin "axt-logdx.exe") --stdin --json --severity error | Out-Null
}
finally {
    if (Get-Location | Where-Object { $_.Path -like "$Tmp*" }) {
        Pop-Location
    }
    Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}
