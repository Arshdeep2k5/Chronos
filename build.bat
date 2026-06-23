@echo off
REM Switch back to MSVC toolchain
echo Switching to MSVC toolchain...
rustup default stable-x86_64-pc-windows-msvc

REM Try to find and run VsDevCmd
echo Setting up Visual Studio environment...

REM Check for VS 2022 BuildTools
if exist "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" (
    call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=x64
    goto build
)

REM Check for VS 2022 Community
if exist "C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\VsDevCmd.bat" (
    call "C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\VsDevCmd.bat" -arch=x64
    goto build
)

REM Check for VS 2022 Professional
if exist "C:\Program Files\Microsoft Visual Studio\2022\Professional\Common7\Tools\VsDevCmd.bat" (
    call "C:\Program Files\Microsoft Visual Studio\2022\Professional\Common7\Tools\VsDevCmd.bat" -arch=x64
    goto build
)

REM Check for VS 2022 Enterprise
if exist "C:\Program Files\Microsoft Visual Studio\2022\Enterprise\Common7\Tools\VsDevCmd.bat" (
    call "C:\Program Files\Microsoft Visual Studio\2022\Enterprise\Common7\Tools\VsDevCmd.bat" -arch=x64
    goto build
)

REM Check for VS 2019
if exist "C:\Program Files (x86)\Microsoft Visual Studio\2019\Community\Common7\Tools\VsDevCmd.bat" (
    call "C:\Program Files (x86)\Microsoft Visual Studio\2019\Community\Common7\Tools\VsDevCmd.bat" -arch=x64
    goto build
)

echo Warning: Could not find Visual Studio DevCmd batch file
echo Attempting build anyway...

:build
cd /d d:\Chronos_Hackathon\src-tauri
echo Building Rust project...
cargo build

if %ERRORLEVEL% NEQ 0 (
    echo Build failed with exit code %ERRORLEVEL%
    exit /b %ERRORLEVEL%
)

echo Build completed successfully!
pause
