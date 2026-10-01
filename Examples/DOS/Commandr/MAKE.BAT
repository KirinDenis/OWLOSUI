@echo off
rem Build the commander, under DOS, with Borland Pascal 7's command-line
rem compiler, for real mode.
rem
rem     MAKE            COMMANDR.EXE
rem     MAKE EXIT       ...and close DOSBox after (for BUILD.CMD)
rem
rem BP is where Borland Pascal is - the folder holding BIN. BUILD.CMD
rem mounts it as P: and that is the default here too. The unit OWLOSUI is
rem compiled from lib\dos\pascal and its OWLOSUI.TPU is left here, then
rem deleted: nothing is written into lib.
if "%BP%"=="" set BP=P:\
set PATH=%BP%BIN;%PATH%
echo before > MAKE.OUT
del COMMANDR.EXE
echo compiling COMMANDR >> MAKE.OUT
BPC -CD -Q -B -E. -U..\..\..\lib\dos\pascal COMMANDR.PAS >> MAKE.OUT
del OWLOSUI.TPU
if not exist COMMANDR.EXE echo COMMANDR.EXE was not built >> MAKE.OUT
echo after >> MAKE.OUT
if "%1"=="EXIT" exit
if "%1"=="exit" exit
type MAKE.OUT
