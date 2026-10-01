@echo off
rem The commander, tested: built, then run in DOSBox-X on a folder of its
rem own, PLAY, with keys typed at it (RUN.BAT's :test says which), and
rem checked twice - what it showed (SHOTS.TXT) and what it did to the disk.
rem The resident itself must be built: lib\dos\build.cmd.
setlocal
cd /d %~dp0
call .\BUILD.CMD || exit /b 1

rem PLAY\A holds what the keys work on; PLAY\B starts empty.
rd /s /q PLAY 2>nul
mkdir PLAY\A\SUB
mkdir PLAY\B
> PLAY\A\ONE.TXT echo Hello from ONE.
> PLAY\A\GONE.TXT echo Deleted by F8.
> PLAY\A\SUB\THREE.TXT echo Moved with its folder.
> PLAY\A\TWO.PAS echo program Two;
>> PLAY\A\TWO.PAS echo begin
>> PLAY\A\TWO.PAS echo   WriteLn('two');
>> PLAY\A\TWO.PAS echo end.

rem The keys are typed as on a US keyboard, whatever the host's is. The
rem words looked for below have no ? or *: in a FOR list cmd takes those
rem for a file pattern, and a pattern that matches nothing is skipped -
rem a check that passes by not running.
set DOSBOXX_OPTS=%DOSBOXX_OPTS% -set "dos keyboardlayout=us"
del SHOTS.TXT RUN.OUT 2>nul
call .\RUNWIN.BAT TEST
findstr /c:"after" RUN.OUT >nul || (echo FAIL: the commander never came back & exit /b 1)
findstr /c:"errorlevel" RUN.OUT >nul && (echo FAIL: it did not leave with code 0 & exit /b 1)
for %%S in ("ONE.TXT" "Hello from ONE." "Copy 1 file(s)" "Delete 1 file(s): GONE.TXT" "TWO.PAS" "Attributes" "Alt+F10" "EXAMPLES" "Left side: drive") do (
  findstr /c:%%S SHOTS.TXT >nul || (echo FAIL: %%~S was never on the screen & exit /b 1)
)
findstr /c:"Tree of C:" SHOTS.TXT >nul || (echo FAIL: the tree's window did not say so in its title & exit /b 1)
rem The folder is in each window's title now, and the foot says only what
rem the cursor is on: no path with a mask left anywhere on the screen.
findstr /l /c:"*.*" SHOTS.TXT >nul && (echo FAIL: a path with its mask is still at the foot of a panel & exit /b 1)
echo ok: the two sides, the viewer, the questions, the editor, properties, the tree and the drives were shown
if not exist PLAY\B\ONE.TXT (echo FAIL: F5 did not copy ONE.TXT & exit /b 1)
if not exist PLAY\A\ONE.TXT (echo FAIL: the copy took ONE.TXT away & exit /b 1)
if not exist PLAY\B\NEW\ (echo FAIL: F7 did not make NEW & exit /b 1)
if not exist PLAY\B\SUB\THREE.TXT (echo FAIL: F6 did not move SUB with its file & exit /b 1)
if exist PLAY\A\SUB\ (echo FAIL: F6 left SUB behind & exit /b 1)
if exist PLAY\A\GONE.TXT (echo FAIL: F8 did not delete GONE.TXT & exit /b 1)
rem Either case: DOSBox-X's AUTOTYPE types the x as X.
findstr /b /i /c:"xprogram Two;" PLAY\A\TWO.PAS >nul || (echo FAIL: F4, x and F2 did not save TWO.PAS & exit /b 1)
findstr /c:"WriteLn('two');" PLAY\A\TWO.PAS >nul || (echo FAIL: saving lost the rest of TWO.PAS & exit /b 1)
echo ok: copied, made, moved, deleted and saved, on the disk
