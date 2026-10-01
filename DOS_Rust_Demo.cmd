@echo off
rem The same demo on DOS, written in Rust with the toolkit linked in: a
rem 32-bit program under DPMI. Alt+X leaves; then close DOSBox-X. Needs
rem DOSBox-X only: the program is in the repository.
rem The program: Examples\DOS\Rust\src\demo.rs
setlocal
cd /d "%~dp0"
call Examples\DOS\FINDDBX.CMD || (pause & exit /b 1)
call Examples\DOS\Rust\RUNWIN.BAT DEMO
