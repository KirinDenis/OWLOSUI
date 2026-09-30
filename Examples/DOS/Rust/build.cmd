@echo off
rem The DOS build: OWLOS.BIN (the core and the shared application, a flat
rem 32-bit binary), HELLO.BIN and DEMO.BIN (the programs the other DOS
rem folders have, part for part) from Rust nightly with build-std, and a
rem loader .COM for each from FASM. Needs, once:
rem     rustup toolchain install nightly --component rust-src
rem and a FASM for Windows: set FASM to it, or put FASM.EXE on the PATH.
setlocal
cd /d %~dp0
if "%FASM%"=="" set FASM=fasm.exe
if not exist "%FASM%" if exist C:\DOSFiles\wire-city-2\TOOLS\FASM\WIN\FASM.EXE set FASM=C:\DOSFiles\wire-city-2\TOOLS\FASM\WIN\FASM.EXE
cargo +nightly build --release || exit /b 1
copy /y target\i386-owlos-none\release\owlos OWLOS.BIN >nul || exit /b 1
copy /y target\i386-owlos-none\release\hello HELLO.BIN >nul || exit /b 1
copy /y target\i386-owlos-none\release\demo DEMO.BIN >nul || exit /b 1
"%FASM%" OWLOS.ASM OWLOS.COM || exit /b 1
"%FASM%" HELLO.ASM HELLO.COM || exit /b 1
"%FASM%" DEMO.ASM DEMO.COM || exit /b 1
"%FASM%" SCRDUMP.ASM SCRDUMP.COM || exit /b 1
for %%F in (OWLOS.BIN) do echo OWLOS.BIN: %%~zF bytes
echo Run RUN.BAT under DOS (DOSBox-X or a real machine), or RUNWIN.BAT here.
