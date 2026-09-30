@echo off
rem Build the C examples, under DOS, with Open Watcom's 16-bit compiler.
rem
rem     MAKE            HELLO.EXE and DEMO.EXE
rem     MAKE EXIT       ...and close DOSBox after (for BUILD.CMD)
rem
rem WATCOM is where Watcom is - the folder holding BINW and H. BUILD.CMD
rem mounts it as W: and that is the default here too.
if "%WATCOM%"=="" set WATCOM=W:\
set INCLUDE=%WATCOM%H;..\..\..\lib\dos\c
set PATH=%WATCOM%BINW;%PATH%
echo before > MAKE.OUT
del HELLO.EXE
del DEMO.EXE
echo compiling HELLO >> MAKE.OUT
wcl -q -ms -bt=dos -fe=HELLO.EXE HELLO.C ..\..\..\lib\dos\c\OWLOSUI.C >> MAKE.OUT
echo compiling DEMO >> MAKE.OUT
wcl -q -ms -bt=dos -fe=DEMO.EXE DEMO.C ..\..\..\lib\dos\c\OWLOSUI.C >> MAKE.OUT
del *.OBJ
if not exist HELLO.EXE echo HELLO.EXE was not built >> MAKE.OUT
if not exist DEMO.EXE echo DEMO.EXE was not built >> MAKE.OUT
echo after >> MAKE.OUT
if "%1"=="EXIT" exit
type MAKE.OUT
