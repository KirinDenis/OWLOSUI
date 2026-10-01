@echo off
rem The Rust application in a native Windows window - the same one the
rem browser and DOS show. Alt+X leaves. Needs Rust.
rem The program: Examples\Desktop\Rust\02-Window, Examples\shared\app.rs
setlocal
cd /d "%~dp0"
where cargo >nul 2>&1 || (echo This needs Rust: https://rustup.rs & pause & exit /b 1)
cargo run -q --release -p owlosui-win
if errorlevel 1 pause
