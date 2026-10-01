@echo off
rem The same demo on DOS, written in assembler (FASM): an 8 KB .COM file.
rem Alt+X leaves; then close DOSBox-X. Needs DOSBox-X only: the program is
rem in the repository.
rem The program: Examples\DOS\Asm\DEMO.ASM
setlocal
cd /d "%~dp0"
call Examples\DOS\FINDDBX.CMD || (pause & exit /b 1)
call Examples\DOS\Asm\RUNWIN.BAT DEMO
