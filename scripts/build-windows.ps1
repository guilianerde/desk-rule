[CmdletBinding()]
param(
    [string]$VcpkgRoot = $env:VCPKG_ROOT
)

$ErrorActionPreference = 'Stop'
if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw 'Run this script on Windows with the x64 MSVC build tools installed.'
}
if (-not $VcpkgRoot) {
    throw 'Set VCPKG_ROOT or pass -VcpkgRoot to a bootstrapped vcpkg checkout.'
}

$root = Split-Path -Parent $PSScriptRoot
$VcpkgRoot = (Resolve-Path -LiteralPath $VcpkgRoot).Path
$vcpkg = Join-Path $VcpkgRoot 'vcpkg.exe'
if (-not (Test-Path -LiteralPath $vcpkg -PathType Leaf)) {
    throw "Cannot find $vcpkg. Run bootstrap-vcpkg.bat in that checkout."
}
if (-not (Test-Path -LiteralPath (Join-Path $root 'libs/hbb_common/Cargo.toml'))) {
    throw 'Restore libs/hbb_common before building (git submodule update --init --recursive).'
}

if (-not $env:LIBCLANG_PATH) {
    $llvm = Join-Path $env:ProgramFiles 'LLVM/bin'
    if (Test-Path -LiteralPath (Join-Path $llvm 'libclang.dll')) {
        $env:LIBCLANG_PATH = $llvm
    }
}
if (-not $env:LIBCLANG_PATH) {
    throw 'Set LIBCLANG_PATH to the folder containing the x64 libclang.dll.'
}

$env:VCPKG_ROOT = $VcpkgRoot
$env:VCPKG_INSTALLED_ROOT = Join-Path $VcpkgRoot 'installed'
$triplet = 'x64-windows-static'
$overlay = Join-Path $root 'res/vcpkg'
$packages = @('libvpx', 'libyuv', 'aom', 'opus', 'libjpeg-turbo') |
    ForEach-Object { "${_}:$triplet" }

# Use classic mode so the portable build only installs its own codec dependencies.
Push-Location $VcpkgRoot
try {
    & $vcpkg install --classic "--overlay-ports=$overlay" @packages
    if ($LASTEXITCODE -ne 0) { throw "vcpkg failed with exit code $LASTEXITCODE." }
}
finally {
    Pop-Location
}

$target = 'x86_64-pc-windows-msvc'
$buildDir = Join-Path $root 'target/windows-portable'
$staging = Join-Path $buildDir 'desk-rule-windows-x64'
$zip = Join-Path $buildDir 'desk-rule-windows-x64.zip'
New-Item -ItemType Directory -Force -Path $staging | Out-Null

Push-Location $root
try {
    & python res/inline-sciter.py
    if ($LASTEXITCODE -ne 0) { throw "Embedding Sciter resources failed with exit code $LASTEXITCODE." }
    & cargo build --locked --release --features inline --bin desk-rule --target $target --target-dir $buildDir
    if ($LASTEXITCODE -ne 0) { throw "Cargo failed with exit code $LASTEXITCODE." }
}
finally {
    Pop-Location
}

$sciter = Join-Path $staging 'sciter.dll'
$sciterRevision = 'f33df075d9eb2f8d252cb88f1b2c8096e56197ed'
$sciterHash = '4d97528e157c55ef1fabe9e37a9697116ab66660d7da6163f90a3a7abf80dd56'
Invoke-WebRequest -UseBasicParsing -Uri "https://raw.githubusercontent.com/c-smile/sciter-sdk/$sciterRevision/bin.win/x64/sciter.dll" -OutFile $sciter
if ((Get-FileHash -Algorithm SHA256 -LiteralPath $sciter).Hash -ne $sciterHash) {
    throw 'The Windows Sciter DLL did not match its expected SHA-256.'
}

$exe = Join-Path $buildDir "$target/release/desk-rule.exe"
Copy-Item -LiteralPath $exe -Destination $staging -Force
Copy-Item -LiteralPath (Join-Path $root 'LICENCE') -Destination $staging -Force
Copy-Item -LiteralPath (Join-Path $root 'README.md') -Destination $staging -Force
Compress-Archive -LiteralPath $staging -DestinationPath $zip -Force
Write-Output $zip
