; ============================================================================
;  SCRDUMP - the text screen into a file, as it is.
;
;      FASM SCRDUMP.ASM SCRDUMP.COM
;      SCRDUMP SCREEN.BIN
;
;  4000 bytes straight out of B800: 2000 cells of glyph byte then
;  attribute byte. What a test reads instead of a screenshot - the file
;  IS the screen, with nothing left to interpret. Run it after a program
;  has drawn and left: DOS does not clear the screen on the way back.
; ============================================================================

        org 100h
        use16

start:
        ; the file name is the command tail, blank-trimmed, zero-ended
        mov  si,81h
        movzx cx,byte [80h]
        jcxz usage
.skip:  lodsb
        cmp  al,' '
        jne  .found
        loop .skip
        jmp  usage
.found: dec  si
        mov  dx,si
.end:   lodsb
        cmp  al,' '
        je   .cut
        cmp  al,13
        jne  .end
.cut:   mov  byte [si-1],0

        mov  ah,3Ch
        xor  cx,cx
        int  21h
        jc   fail
        mov  bx,ax
        push ds
        mov  ax,0B800h
        mov  ds,ax
        xor  dx,dx
        mov  cx,4000
        mov  ah,40h
        int  21h
        pop  ds
        mov  ah,3Eh
        int  21h
        mov  ax,4C00h
        int  21h

usage:  mov  dx,m_usage
        jmp  say
fail:   mov  dx,m_fail
say:    mov  ah,9
        int  21h
        mov  ax,4C01h
        int  21h

m_usage db 'SCRDUMP FILE.BIN - the text screen into FILE.BIN',13,10,'$'
m_fail  db 'Could not create the file.',13,10,'$'
