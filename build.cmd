@echo off
setlocal
pushd "%~dp0"
if errorlevel 1 (
    echo Could not open the project directory. 1>&2
    endlocal
    exit /b 1
)

where cargo >nul 2>nul
if errorlevel 1 (
    echo Cargo was not found. Install Rust from https://www.rust-lang.org/tools/install. 1>&2
    popd
    endlocal
    exit /b 1
)

set CROSSXW=0
set TARGET_TRIPLE=
:parse_args
if "%~1"=="" goto args_parsed
if /i "%~1"=="-crossxw" goto crossxw_flag
if /i "%~1"=="/crossxw" goto crossxw_flag
if not "%TARGET_TRIPLE%"=="" goto usage
set "TARGET_TRIPLE=%~1"
shift
goto parse_args
:crossxw_flag
set "CROSSXW=1"
shift
goto parse_args
:args_parsed

if "%TARGET_TRIPLE%"=="" goto build

set "TARGET_ARCH="
if /i "%TARGET_TRIPLE%"=="i686-pc-windows-msvc" set "TARGET_ARCH=x86"
if /i "%TARGET_TRIPLE%"=="x86_64-pc-windows-msvc" set "TARGET_ARCH=x86_64"
if /i "%TARGET_TRIPLE%"=="aarch64-pc-windows-msvc" set "TARGET_ARCH=arm64"
if "%TARGET_ARCH%"=="" (
    echo Unsupported Windows target: %TARGET_TRIPLE% 1>&2
    popd
    endlocal
    exit /b 2
)

rem Which architecture this shell runs on, so an x64 shell emulated on ARM64,
rem or an ARM64 shell started from an x64 one, is not mistaken for the
rem machine's own architecture.
set "HOST_ARCH=%PROCESSOR_ARCHITECTURE%"
if defined PROCESSOR_ARCHITEW6432 set "HOST_ARCH=%PROCESSOR_ARCHITEW6432%"
if /i "%HOST_ARCH%"=="AMD64" set "HOST_ARCH=x86_64"

rem A target the machine does not run natively is a cross-compilation, which is
rem opt-in: the caller asks for it with -crossxw, or GitHub Actions asks for it
rem by building a matrix of targets on runners built for one architecture.
rem Everything else builds for the machine it runs on.
if /i "%TARGET_ARCH%"=="%HOST_ARCH%" goto build
if "%CROSSXW%"=="1" goto build
if /i "%GITHUB_ACTIONS%"=="true" goto build
echo Building %TARGET_TRIPLE% on Windows %HOST_ARCH% is cross-compilation. Pass -crossxw to build it anyway. 1>&2
popd
endlocal
exit /b 2

:build
if "%TARGET_TRIPLE%"=="" (
    set "BINARY=target\release\squiglab.exe"
    cargo build --release
) else (
    set "BINARY=target\%TARGET_TRIPLE%\release\squiglab.exe"
    cargo build --release --target "%TARGET_TRIPLE%"
)
if errorlevel 1 goto build_failed

cargo run --release --bin app-bundle -- package windows "%BINARY%" dist\windows
if errorlevel 1 goto build_failed

echo.
echo Build and package complete: %CD%\dist\windows
popd
endlocal
exit /b 0

:usage
echo Usage: %~nx0 [-crossxw] [Rust target triple] 1>&2
popd
endlocal
exit /b 2

:build_failed
echo Build failed. 1>&2
popd
endlocal
exit /b 1
