@echo off
rem The Rust demo on this terminal, the toolkit linked into the program:
rem windows, a help viewer, the file panel, every control. Alt+X leaves.
rem Needs Rust.
rem The program: Examples\Desktop\Rust\01-Terminal
setlocal
cd /d "%~dp0"
where cargo >nul 2>&1 || (echo This needs Rust: https://rustup.rs & pause & exit /b 1)
cargo run -q --release -p owlosui-demo
if errorlevel 1 pause
