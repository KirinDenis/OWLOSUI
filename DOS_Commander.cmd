@echo off
rem A two-panel file manager on DOS, in Pascal, on this repository - in
rem DOSBox-X it is drive C:, and Examples is beside it: Tab the other side, Insert marks, F3 view, F4 edit, F5 copy,
rem F6 move, F7 new folder, F8 delete, F1 help, F10 quits. It works on real
rem files. Needs DOSBox-X only: the program is in the repository.
rem The program: Examples\DOS\Commandr\COMMANDR.PAS
setlocal
cd /d "%~dp0"
call Examples\DOS\FINDDBX.CMD || (pause & exit /b 1)
call Examples\DOS\Commandr\RUNWIN.BAT C:\ C:\EXAMPLES
