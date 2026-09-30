; ============================================================================
;  OWLOSRES.ASM - the resident's loader.
;
;      FASM OWLOSRES.ASM OWLOSRES.COM
;
;  OWLOSRES.COM puts OWLOSRES.BIN - the toolkit, built from
;  src/owlosres.rs - in extended memory and jumps into it; that then
;  hooks INT 60h and runs the program named on the command line:
;
;      CWSDPMI
;      OWLOSRES DEMO.EXE
;
;  All of the loading is LOADER.INC, the same as the Rust example's.
;  No LOADER.LOG: this runs in the program's folder, not its own.
; ============================================================================

LOG = 0
        include 'LOADER.INC'
