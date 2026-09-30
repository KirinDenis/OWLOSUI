@echo off
rem The keys a test types at HELLO or DEMO: the same in every language,
rem because the programs are. DOSBox-X's AUTOTYPE presses them one at a
rem time, starting two seconds from now, while the program runs. F12 is
rem the resident's own key: a picture of the screen onto SHOTS.TXT.
rem
rem The demo: Enter closes the About box; F10 opens the menu bar, and
rem right and down walk it, because AUTOTYPE cannot hold Alt. In the
rem calculator 12+30= must show 42; the calendar pages a month on; in the
rem puzzle Up then Down slide at least one tile, wherever the scramble
rem left the hole; Tile arranges the windows; F3 is Open - Down and
rem Enter open the folder's first file, BUILD.CMD, in a viewer, and F3
rem again with Enter on ".." walks up a folder; F1 is About again;
rem File, Exit leaves.
if "%1"=="HELLO" AUTOTYPE -w 2 -p 0.4 f12 enter
if "%1"=="DEMO" AUTOTYPE -w 2 -p 0.5 enter f12 f10 right enter 1 2 kp_plus 3 0 equals f12 f10 right down enter f12 pagedown f12 f10 right down down enter f12 f10 right down down down enter up down f12 f10 right right t f12 f3 f12 down enter f12 f3 enter f12 esc f1 f12 enter f10 x
