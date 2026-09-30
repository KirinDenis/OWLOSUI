; ============================================================================
;  OWLOS.ASM - the loader for this example.
;
;      FASM OWLOS.ASM OWLOS.COM
;
;  OWLOS.COM puts OWLOS.BIN - the core and the application, built from
;  src/main.rs - in extended memory and jumps into it. The loading is
;  lib/dos/LOADER.INC, the one the resident (OWLOSRES) uses too: DPMI,
;  four megabytes, two selectors, a jump. It reads like a lesson.
;
;  LOADER.LOG gets a mark per stage, for when something dies before the
;  program can say so.
; ============================================================================

LOG = 1
        include '..\..\..\lib\dos\LOADER.INC'
