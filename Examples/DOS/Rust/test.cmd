@echo off
rem The DOS smoke test, from Windows: build, run one frame under DOSBox-X,
rem and check that the frame the program wrote (SHOT.BIN, through DOS) is
rem the screen the emulator shows (SCREEN.BIN, straight out of B800) and
rem that the program left with code 0. Needs DOSBox-X: see RUNWIN.BAT.
rem
rem The .\ are on purpose: a machine may keep the current directory off
rem the path, and then a bare name is not found.
setlocal
cd /d %~dp0
call .\build.cmd || exit /b 1
call .\RUNWIN.BAT SHOT
if not exist SHOT.BIN echo FAIL: the program wrote no frame & exit /b 1
if not exist SCREEN.BIN echo FAIL: no screen was captured & exit /b 1
findstr /c:"errorlevel" OWLOS.OUT >nul && (echo FAIL: the program did not leave with code 0 & type OWLOS.OUT & exit /b 1)
fc /b SHOT.BIN SCREEN.BIN >nul || (echo FAIL: the frame and the screen differ & exit /b 1)
echo ok: the frame on the DOS screen is the frame the core drew
rem Then keys, typed by DOSBox-X: F4 opens the dialog, Escape cancels it,
rem F12 writes the frame, F10 and the menu leave. The program must come
rem back with code 0 and a frame.
call .\RUNWIN.BAT KEYS
if not exist SHOT.BIN echo FAIL: no frame from the keyed run & exit /b 1
findstr /c:"errorlevel" OWLOS.OUT >nul && (echo FAIL: the keyed run did not leave with code 0 & type OWLOS.OUT & exit /b 1)
findstr /c:"after" OWLOS.OUT >nul || (echo FAIL: the keyed run never came back & exit /b 1)
echo ok: keys reach the program on DOS and Exit leaves cleanly
rem Then the churn: the program drags, zooms and opens dialogs by itself
rem for a while. The heap has to hold; a PANIC is exit code 3.
call .\RUNWIN.BAT STRESS
findstr /c:"errorlevel" OWLOS.OUT >nul && (echo FAIL: the stress run did not survive & type OWLOS.OUT & type TRACE.TXT & exit /b 1)
if not exist SHOT.BIN echo FAIL: no frame after the stress run & exit /b 1
echo ok: a minute of dragging and zooming, and the heap held
rem Last, HELLO and DEMO, with the keys and the checks every DOS folder
rem shares: the same program as in Asm, C and Pascal must show the same.
call ..\CHECK.CMD || exit /b 1
