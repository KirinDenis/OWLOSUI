@echo off
rem Run one of this folder's assembler programs, with the toolkit behind it:
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
if "%1"=="" goto usage
echo before %1 > RUN.OUT
..\..\..\lib\dos\CWSDPMI.EXE
if "%2"=="TEST" call ..\TESTKEYS.BAT %1
..\..\..\lib\dos\OWLOSRES.COM %1.COM
if errorlevel 1 echo errorlevel 1 or more >> RUN.OUT
echo after >> RUN.OUT
if "%2"=="TEST" exit
goto end
:usage
echo RUN HELLO, or RUN DEMO
:end
