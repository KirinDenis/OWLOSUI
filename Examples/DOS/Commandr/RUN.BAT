@echo off
rem Run the commander, with the toolkit behind it:
rem
rem     RUN                 both sides on this folder
rem     RUN C:\ C:\GAMES    the left side on C:\, the right side on C:\GAMES
rem     RUN TEST            the test: PLAY\A and PLAY\B, which TEST.CMD
rem                         makes, and the keys typed by DOSBox-X
rem
rem CWSDPMI, with no arguments, stays for exactly one program - OWLOSRES -
rem and OWLOSRES stays for exactly one program: the one it is given, whose
rem arguments follow it. Both live in lib\dos, three folders up.
rem From Windows this starts DOSBox-X and runs itself in there (RUNWIN.BAT).
if "%OS%"=="Windows_NT" goto windows
if "%1"=="TEST" goto test
if "%1"=="test" goto test
..\..\..\lib\dos\CWSDPMI.EXE
..\..\..\lib\dos\OWLOSRES.COM COMMANDR.EXE %1 %2
goto end
:test
rem The test's keys, pressed one at a time from two seconds on. F12 is the
rem resident's own: a picture of the screen onto SHOTS.TXT. AUTOTYPE cannot
rem hold Alt, which is why the tree and the drives are reached through the
rem Help box's buttons here.
rem   f12                        the two sides
rem   o f3 f12 esc               ONE.TXT in the viewer, and closed
rem   insert f5 f12 left enter   ONE.TXT marked, copied to PLAY\B: Yes
rem   tab f7 n e w enter tab     a folder NEW made in PLAY\B
rem   s f6 left enter            the folder SUB moved to PLAY\B
rem   g f8 f12 left enter        GONE.TXT deleted
rem   t f4 f12 x f2              TWO.PAS edited - an x typed - and saved
rem   o enter f12 esc            ONE.TXT's properties
rem   f1 f12 right enter f12     Help, and its Tree button: the drive as a tree
rem   f1 right enter             ...and the files again
rem   f1 right right enter f12   the drive list for the left side
rem   esc f10                    and out
echo before > RUN.OUT
..\..\..\lib\dos\CWSDPMI.EXE
AUTOTYPE -w 2 -p 0.5 f12 o f3 f12 esc insert f5 f12 left enter tab f7 n e w enter tab s f6 left enter g f8 f12 left enter t f4 f12 x f2 o enter f12 esc f1 f12 right enter f12 f1 right enter f1 right right enter f12 esc f10
..\..\..\lib\dos\OWLOSRES.COM COMMANDR.EXE PLAY\A PLAY\B
if errorlevel 1 echo errorlevel 1 or more >> RUN.OUT
echo after >> RUN.OUT
exit
:windows
call "%~dp0RUNWIN.BAT" %1 %2
:end
