@echo off
rem OWLOSRES, the resident: OWLOSRES.BIN (the toolkit, a flat 32-bit
rem binary) from Rust nightly with build-std, and OWLOSRES.COM (its loader)
rem from FASM. Needs, once:
rem     rustup toolchain install nightly --component rust-src
rem and a FASM for Windows: set FASM to it, or put FASM.EXE on the PATH.
setlocal
cd /d %~dp0
if "%FASM%"=="" set FASM=fasm.exe
if not exist "%FASM%" if exist C:\DOSFiles\wire-city-2\TOOLS\FASM\WIN\FASM.EXE set FASM=C:\DOSFiles\wire-city-2\TOOLS\FASM\WIN\FASM.EXE
cargo +nightly build --release || exit /b 1
copy /y target\i386-owlos-none\release\owlosres OWLOSRES.BIN >nul || exit /b 1
"%FASM%" OWLOSRES.ASM OWLOSRES.COM || exit /b 1
for %%F in (OWLOSRES.BIN) do echo OWLOSRES.BIN: %%~zF bytes
