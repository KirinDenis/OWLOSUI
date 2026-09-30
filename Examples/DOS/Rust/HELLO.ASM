; ============================================================================
;  HELLO.ASM - the loader for HELLO.BIN, the Rust HELLO (src/hello.rs).
;
;      FASM HELLO.ASM HELLO.COM
;
;  All of it is lib/dos/LOADER.INC, which loads the .BIN named like the
;  .COM, from beside it: DPMI, four megabytes, two selectors, a jump.
; ============================================================================

LOG = 0
        include '..\..\..\lib\dos\LOADER.INC'
