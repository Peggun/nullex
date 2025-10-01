# === dev-setup: starting ===
"WARNING: This script will install and update multiple development packages and tools on your system."
"It will need multiple administrator requests to be able to install all needed packages and tools and may make significant changes to your system."

$yes_no = Read-Host "Continue with the installation? [Y/n] (This will take some time.)"

if ($yes_no.ToLower() -ine "y") {
    "Aborted by user."
    return;
}

$yes_no_sure = Read-Host "Are you absolutely sure you want to proceed? [Y/n]"

if ($yes_no_sure.ToLower() -ine "y") {
    "Aborted by user."
    return;
}

"-- Installing rustup (non-interactive) and setting default toolchain to nightly..."
$is_64bit = [System.Environment]::Is64BitOperatingSystem
$downloadsPath = (New-Object -ComObject Shell.Application).Namespace('shell:Downloads').Self.Path
$rustupDownloadPath = $downloadsPath + "/rustup-init.exe"
$vsBuildToolsDownloadPath = $downloadsPath + "/vs_BuildTools.exe"

# before installing rust we need to install the vs build tools
"-- Installing VS Build Tools for Rust installation..."
"-- WARNING: Another window will pop up."

& winget.exe install --id=Microsoft.VisualStudio.2022.BuildTools -e --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools;Microsoft.VisualStudio.Component.VC.Tools.x86.x64;Microsoft.VisualStudio.Component.Windows11SDK.22621"

if ($is_64bit) {
    Start-BitsTransfer -Source "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe" -Destination $rustupDownloadPath
} else {
    Start-BitsTransfer -Source "https://static.rust-lang.org/rustup/dist/i686-pc-windows-msvc/rustup-init.exe" -Destination $rustupDownloadPath
}

# run the .exe to install rust (add -y here.)
Start-Process -FilePath $rustupDownloadPath -ArgumentList "-y --default-toolchain nightly" -Wait -NoNewWindow

# remove the file after finished with it (clean up)
Remove-Item -Path $rustupDownloadPath

"-- Adding llvm-tools-preview to nightly toolchain..."
& "$HOME\.cargo\bin\rustup.exe" component add llvm-tools-preview rust-src --toolchain nightly
"Added 'llvm-tools-preview' & 'rust-src' components."

"-- Installing cargo tools (bootimage) --"
& "$HOME\.cargo\bin\cargo.exe" install bootimage

"-- Installing MSYS2 and required tools (QEMU)"
# get through winget as msys2 files include date of release so we cant get through their website.
& winget.exe download --id=MSYS2.MSYS2 -d $downloadsPath

$msys2exe = (Get-ChildItem -Path $downloadsPath -Filter 'MSYS2*.exe' -File | Select-Object -First 1 -ExpandProperty Name)
$msys2exefullpath = $downloadsPath + "\" + $msys2exe

# https://www.msys2.org/docs/installer/
"-- Running MSYS2 Installler (non-interactive)"
"-- WARNING: Another terminal with admin privileges will open."

Start-Process -FilePath $msys2exefullpath -ArgumentList "in --confirm-command --accept-messages --root C:/msys64" -Wait

"-- Installing required MSYS2 packages"
"-- WARNING: Multiple terminal windows will open."
& C:\msys64\msys2_shell.cmd -defterm -no-start -here -ucrt64 -c "pacman -Syu"
& C:\msys64\msys2_shell.cmd -defterm -no-start -here -ucrt64 -c "pacman -Sy --needed base-devel mingw-w64-ucrt-x86_64-toolchain mingw-w64-ucrt-x86_64-make mingw-w64-ucrt-x86_64-qemu mingw-w64-ucrt-x86_64-qemu-image-util, mingw-w64-ucrt-x86_64-make mingw-w64-ucrt-x86_64-python"

"-- Adding C:\msys64\ucrt64\bin to PATH..."
$newPath = "C:\msys64\ucrt64\bin"
$currentPath = [Environment]::GetEnvironmentVariable("Path", "Machine")

if (-not ($currentPath -split ';' | Select-String -Pattern "^$([regex]::Escape($newPath))$" -Quiet)) {
    $updatedPath = "$currentPath;$newPath"
    [Environment]::SetEnvironmentVariable("Path", $updatedPath, "Machine")
    Write-Host "Path '$newPath' added to system PATH."
} else {
    Write-Host "Path '$newPath' already exists in system PATH."
}