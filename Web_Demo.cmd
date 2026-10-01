@echo off
rem The demo in your browser: the toolkit compiled to WebAssembly, served
rem on http://localhost:8765 and opened. Its File menu opens and saves
rem files in the browser's own storage, in Examples\Web\files here, or in a
rem WebDAV folder. Ctrl+C in this window stops the server.
rem Needs Rust, and once its WebAssembly target (this says how if it is missing).
rem The page: Examples\Web\JavaScript\05-Demo
setlocal
cd /d "%~dp0"
where cargo >nul 2>&1 || (echo This needs Rust: https://rustup.rs & pause & exit /b 1)
rem Not findstr /x: rustup ends its lines with LF alone, and /x then matches nothing.
rustup target list --installed 2>nul | findstr /c:"wasm32-unknown-unknown" >nul || (
  echo The browser build needs Rust's WebAssembly target. Add it once with:
  echo.
  echo     rustup target add wasm32-unknown-unknown
  echo.
  pause
  exit /b 1
)
call Examples\Web\RUN.CMD
if errorlevel 1 pause
