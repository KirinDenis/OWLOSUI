@echo off
rem The assembler examples, from Windows: HELLO.COM and DEMO.COM with the
rem FASM for Windows. Set FASM to it, or put FASM.EXE on the PATH. Under
rem DOS the same lines work with the DOS FASM:  FASM HELLO.ASM HELLO.COM
setlocal
cd /d %~dp0
if "%FASM%"=="" set FASM=fasm.exe
if not exist "%FASM%" if exist C:\DOSFiles\wire-city-2\TOOLS\FASM\WIN\FASM.EXE set FASM=C:\DOSFiles\wire-city-2\TOOLS\FASM\WIN\FASM.EXE
for %%P in (HELLO DEMO) do (
  if exist %%P.ASM (
    del %%P.COM 2>nul
    "%FASM%" %%P.ASM %%P.COM || exit /b 1
  )
)
