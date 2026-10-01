@echo off
rem The demo on DOS, written in Pascal: menus, a calculator, a calendar,
rem an ASCII table, a puzzle, File Open with the file panel, editors.
rem File, Exit (or Alt+X) leaves; then close DOSBox-X. Needs DOSBox-X only:
rem the program is built and in the repository.
rem The program: Examples\DOS\Pascal\DEMO.PAS
setlocal
cd /d "%~dp0"
call Examples\DOS\FINDDBX.CMD || (pause & exit /b 1)
call Examples\DOS\Pascal\RUNWIN.BAT DEMO
