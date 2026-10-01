@echo off
rem The same demo on DOS, written in C (Open Watcom). Alt+X leaves; then
rem close DOSBox-X. Needs DOSBox-X only: the program is in the repository.
rem The program: Examples\DOS\C\DEMO.C
setlocal
cd /d "%~dp0"
call Examples\DOS\FINDDBX.CMD || (pause & exit /b 1)
call Examples\DOS\C\RUNWIN.BAT DEMO
