@echo off
rem Run OWLOSUI on DOS. CWSDPMI with no arguments stays for exactly one
rem program and unloads itself after it. Plain DOSBox has no 32-bit DPMI
rem even with CWSDPMI loaded; DOSBox-X and real machines have.
rem
rem     RUN            the program
rem     RUN SHOT       draw one frame into SHOT.BIN, leave
rem     RUN QUIT       enter the program and leave at once (a smoke test)
rem     RUN STRESS     a second of dragging, zooming and dialogs, done by
rem                    the program itself; then a frame, and code 0 if the
rem                    heap held up
rem     RUN DUMP       only SCRDUMP: the screen as it is into SCREEN.BIN
rem     RUN KEYS       DOSBox-X only: AUTOTYPE presses F4, F12 (a frame into
rem                    SHOT.BIN), Escape, then leaves through the File menu
rem     RUN x EXIT     ...and close the emulator (for RUNWIN.BAT)
rem
rem     RUN HELLO      HELLO, and RUN DEMO the demo: the same programs as in
rem     RUN DEMO       ..\Asm, ..\C and ..\Pascal, with the core linked in
rem     RUN x TEST     ...with the test's keys (..\TESTKEYS.BAT), then closes
rem
rem After the program, SCRDUMP writes the screen it left into SCREEN.BIN,
rem and OWLOS.OUT says what the batch saw: the file is the screen.
rem From Windows this starts DOSBox-X and runs itself in there (RUNWIN.BAT):
rem RUN DEMO is RUN DEMO on either side.
if "%OS%"=="Windows_NT" goto windows
rem DOS compares names as they are typed: demo and DEMO both count.
if "%1"=="HELLO" goto program
if "%1"=="hello" goto program
if "%1"=="DEMO" goto program
if "%1"=="demo" goto program
echo before %1 > OWLOS.OUT
if "%1"=="DUMP" goto dump
if "%1"=="HELP" goto help
CWSDPMI.EXE
if "%1"=="SHOT" OWLOS.COM /SHOT >> OWLOS.OUT
if "%1"=="QUIT" OWLOS.COM /QUIT >> OWLOS.OUT
if "%1"=="STRESS" OWLOS.COM /STRESS >> OWLOS.OUT
if "%1"=="KEYS" AUTOTYPE -w 2 -p 0.4 f4 f12 esc , , f12 , f10 down down enter
if "%1"=="KEYS" OWLOS.COM >> OWLOS.OUT
if "%1"=="PEEK" AUTOTYPE -w 2 -p 0.4 f12 , f10 down down enter
if "%1"=="PEEK" OWLOS.COM >> OWLOS.OUT
if "%1"=="" OWLOS.COM
if errorlevel 1 echo errorlevel 1 or more >> OWLOS.OUT
if errorlevel 7 echo errorlevel 7 or more >> OWLOS.OUT
echo after >> OWLOS.OUT
:help
if not "%1"=="HELP" goto dump
AUTOTYPE > AUTOTYPE.TXT
:dump
SCRDUMP SCREEN.BIN
if errorlevel 1 echo scrdump failed >> OWLOS.OUT
if "%2"=="EXIT" exit
goto end

:program
echo before %1 > RUN.OUT
CWSDPMI.EXE
if "%2"=="TEST" call ..\TESTKEYS.BAT %1
%1.COM
if errorlevel 1 echo errorlevel 1 or more >> RUN.OUT
echo after >> RUN.OUT
if "%2"=="TEST" exit
goto end
:windows
call "%~dp0RUNWIN.BAT" %1 %2
:end
