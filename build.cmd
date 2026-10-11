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

cargo build --release
if errorlevel 1 goto build_failed

echo.
echo Build complete: %CD%\target\release\crigraph-native.exe
popd
endlocal
exit /b 0

:build_failed
echo Build failed. 1>&2
popd
endlocal
exit /b 1
