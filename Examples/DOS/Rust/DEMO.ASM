; ============================================================================
;  DEMO.ASM - the loader for DEMO.BIN, the Rust DEMO (src/demo.rs).
;
;      FASM DEMO.ASM DEMO.COM
;
;  All of it is lib/dos/LOADER.INC, which loads the .BIN named like the
;  .COM, from beside it: DPMI, four megabytes, two selectors, a jump.
; ============================================================================

LOG = 0
        include '..\..\..\lib\dos\LOADER.INC'
