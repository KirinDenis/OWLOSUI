@echo off
rem Build the Pascal examples, under DOS, with Borland Pascal 7's
rem command-line compiler, for real mode.
rem
rem     MAKE            HELLO.EXE and DEMO.EXE
rem     MAKE EXIT       ...and close DOSBox after (for BUILD.CMD)
rem
rem BP is where Borland Pascal is - the folder holding BIN. BUILD.CMD
rem mounts it as P: and that is the default here too. The unit OWLOSUI is
rem compiled from lib\dos\pascal and its OWLOSUI.TPU is left here, then
rem deleted: nothing is written into lib.
if "%BP%"=="" set BP=P:\
set PATH=%BP%BIN;%PATH%
echo before > MAKE.OUT
del HELLO.EXE
del DEMO.EXE
echo compiling HELLO >> MAKE.OUT
BPC -CD -Q -B -E. -U..\..\..\lib\dos\pascal HELLO.PAS >> MAKE.OUT
echo compiling DEMO >> MAKE.OUT
BPC -CD -Q -B -E. -U..\..\..\lib\dos\pascal DEMO.PAS >> MAKE.OUT
del OWLOSUI.TPU
if not exist HELLO.EXE echo HELLO.EXE was not built >> MAKE.OUT
if not exist DEMO.EXE echo DEMO.EXE was not built >> MAKE.OUT
echo after >> MAKE.OUT
if "%1"=="EXIT" exit
type MAKE.OUT
