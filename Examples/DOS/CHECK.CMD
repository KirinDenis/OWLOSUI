@echo off
rem Run HELLO and DEMO of the folder this is called from, in DOSBox-X, with
rem the keys of TESTKEYS.BAT, and check what they showed and how they left:
rem SHOTS.TXT holds every screen F12 photographed, RUN.OUT what RUN.BAT saw.
rem A folder's TEST.CMD builds first, then calls this.
rem
rem The emulator's own exit code is not looked at: this DOSBox-X faults
rem while closing after some runs that ended well, the Rust example's too.
del SHOTS.TXT RUN.OUT 2>nul
call .\RUNWIN.BAT HELLO TEST
findstr /c:"after" RUN.OUT >nul || (echo FAIL: HELLO never came back & exit /b 1)
findstr /c:"errorlevel" RUN.OUT >nul && (echo FAIL: HELLO did not leave with code 0 & exit /b 1)
findstr /c:"Hello, world!" SHOTS.TXT >nul || (echo FAIL: HELLO's window was not on the screen & exit /b 1)
echo ok: HELLO drew its window through INT 60h, and OK ended it
del SHOTS.TXT RUN.OUT 2>nul
call .\RUNWIN.BAT DEMO TEST
findstr /c:"after" RUN.OUT >nul || (echo FAIL: DEMO never came back & exit /b 1)
findstr /c:"errorlevel" RUN.OUT >nul && (echo FAIL: DEMO did not leave with code 0 & exit /b 1)
for %%S in ("File  Tools  Window  Help" "F1 Help  F3 Open" " Calculator " "Su Mo Tu We Th Fr Sa" " ASCII table " " Puzzle " "OWLOS UI Demo" "Open [modal]" "BUILD.CMD [view]" "@echo off" "TESTKEYS.BAT") do (
  findstr /c:%%S SHOTS.TXT >nul || (echo FAIL: %%~S was never on the screen & exit /b 1)
)
findstr /r /c:"Moves: [1-9]" SHOTS.TXT >nul || (echo FAIL: the puzzle never slid a tile & exit /b 1)
findstr /r /c:"                    42  " SHOTS.TXT >nul || (echo FAIL: the calculator did not make 12+30 into 42 & exit /b 1)
echo ok: DEMO - menus, the calculator's 42, the calendar, the table, the puzzle, Open with a file and a folder, About - and Exit left
