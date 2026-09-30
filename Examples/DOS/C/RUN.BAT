@echo off
rem Run one of this folder's C programs, with the toolkit behind it:
rem
rem     RUN HELLO
rem     RUN DEMO
rem
rem CWSDPMI, with no arguments, stays for exactly one program - OWLOSRES -
rem and OWLOSRES stays for exactly one program: the one it is given. Both
rem live in lib\dos, three folders up.
rem
rem     RUN x TEST      DOSBox-X types the test's keys (..\TESTKEYS.BAT)
rem                     and closes when the program has ended
rem From Windows this starts DOSBox-X and runs itself in there (RUNWIN.BAT):
rem RUN DEMO is RUN DEMO on either side.
if "%OS%"=="Windows_NT" goto windows
if "%1"=="" goto usage
echo before %1 > RUN.OUT
..\..\..\lib\dos\CWSDPMI.EXE
if "%2"=="TEST" call ..\TESTKEYS.BAT %1
..\..\..\lib\dos\OWLOSRES.COM %1.EXE
if errorlevel 1 echo errorlevel 1 or more >> RUN.OUT
echo after >> RUN.OUT
if "%2"=="TEST" exit
goto end
:usage
echo RUN HELLO, or RUN DEMO
goto end
:windows
call "%~dp0RUNWIN.BAT" %1 %2
:end
