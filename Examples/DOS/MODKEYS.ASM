; ============================================================================
;  MODKEYS.ASM - keys pressed with Shift, Ctrl or Alt held, for a test.
;
;      FASM MODKEYS.ASM MODKEYS.COM
;      MODKEYS 4100 1E61 c2E03 s5200 ...
;
;  DOSBox-X's AUTOTYPE presses one key at a time and cannot hold another,
;  so Ctrl+Ins, Shift+Home and Ctrl+C are out of its reach. This stays
;  resident and does what the keyboard's own BIOS handler does for a
;  press: the held keys into the flags byte at 0040:0017, and the key -
;  its scan code and its ASCII code - onto the end of the BIOS's ring.
;  One key each half second, from five seconds after it is loaded; half
;  a second after the last, nothing is held any more.
;
;  A key is four hex digits, the scan code and then the ASCII code, as
;  INT 16h hands them over: 1E61 is a, 4F00 End, 9200 Ctrl's own Ins.
;  In front of it, the keys held: s Shift, c Ctrl, a Alt - c2E03 is
;  Ctrl+C. Up to 32 keys, as many as fit on a DOS command line.
;
;  Load it before CWSDPMI: that one stays for exactly the next program,
;  and the next program must be the one the keys are for.
; ============================================================================

        org  100h
        jmp  install

MAX_KEYS = 32
FIRST    = 91                   ; ticks before the first key: five seconds
STEP     = 9                    ; ticks between keys: half a second

old1c   dd   0                  ; the timer's handler before this one
count   dw   0                  ; ticks since loading
next    dw   FIRST              ; the tick the next key is due on
pos     dw   0                  ; which key is next
keys_n  dw   0                  ; how many keys there are
keys    rb   MAX_KEYS*4         ; each: flags, ASCII, scan, unused

; ---------------------------------------------------------------- the tick
; INT 1Ch, 18.2 times a second. Everything it touches is saved first: it
; interrupts whatever is running, the DOS extender's real-mode side too.
int1c:
        push ax bx dx si ds es
        push cs
        pop  ds
        inc  [count]
        mov  ax,[count]
        cmp  ax,[next]
        jb   .done
        add  ax,STEP            ; the one after it, half a second on
        mov  [next],ax
        mov  si,[pos]
        cmp  si,[keys_n]
        ja   .done              ; all pressed, and let go of
        mov  ax,40h
        mov  es,ax              ; the BIOS's data
        je   .let_go            ; (flags of the cmp above) one past the last
        mov  bx,si
        shl  bx,2               ; four bytes a key
        mov  al,[keys+bx]       ; the keys held with it
        mov  ah,[es:17h]
        and  ah,0F0h            ; Shift, Ctrl and Alt off - the locks kept
        or   ah,al
        mov  [es:17h],ah
        mov  ax,word [keys+bx+1]; AL the ASCII code, AH the scan code
        mov  bx,[es:1Ch]        ; the ring's tail: where the key goes
        mov  dx,bx
        add  dx,2               ; and where the tail goes after it
        cmp  dx,[es:82h]        ; past the ring's end?
        jb   .room
        mov  dx,[es:80h]        ; round to its start
.room:
        cmp  dx,[es:1Ah]        ; onto the head: the ring is full, and the
        je   .pressed           ; BIOS would drop the key too
        mov  [es:bx],ax
        mov  [es:1Ch],dx
.pressed:
        inc  [pos]
        jmp  .done
.let_go:
        and  byte [es:17h],0F0h ; nothing held any more
        inc  [pos]
.done:
        pop  es ds si dx bx ax
        jmp  far [cs:old1c]

; ------------------------------------------------------------- loading
; Everything from here on is let go of when it stays resident.
install:
        mov  si,81h             ; the command line
        mov  cl,[80h]
        xor  ch,ch
        add  cx,si              ; CX: where it ends
        mov  di,keys
.token:
        cmp  si,cx
        jae  .parsed
        mov  al,[si]
        cmp  al,' '
        jbe  .space
        xor  dl,dl              ; DL: the keys held with this one
.held:
        or   al,20h             ; s, c, a in either case
        cmp  al,'s'
        jne  .not_s
        or   dl,1               ; right Shift
        jmp  .prefix
.not_s:
        cmp  al,'c'
        jne  .not_c
        or   dl,4               ; Ctrl
        jmp  .prefix
.not_c:
        cmp  al,'a'
        jne  .digits
        or   dl,8               ; Alt
.prefix:
        inc  si
        cmp  si,cx
        jae  usage
        mov  al,[si]
        jmp  .held
.digits:
        cmp  [keys_n],MAX_KEYS
        jae  usage
        mov  [di],dl
        call hex_byte
        jc   usage
        mov  [di+2],al          ; the scan code
        call hex_byte
        jc   usage
        mov  [di+1],al          ; the ASCII code
        add  di,4
        inc  [keys_n]
        jmp  .token
.space:
        inc  si
        jmp  .token
.parsed:
        cmp  [keys_n],0
        je   usage

        mov  ax,351Ch           ; the timer's handler, kept to be called on
        int  21h
        mov  word [old1c],bx
        mov  word [old1c+2],es
        mov  dx,int1c
        mov  ax,251Ch
        int  21h
        mov  es,[2Ch]           ; the environment is not needed resident
        mov  ah,49h
        int  21h
        mov  dx,(install+15) shr 4  ; paragraphs up to here, PSP included
        mov  ax,3100h           ; stay resident, errorlevel 0
        int  21h

usage:
        mov  dx,text_usage
        mov  ah,9
        int  21h
        mov  ax,4C01h
        int  21h

; Two hex digits at SI into AL; carry on anything else. CX: the line's end.
hex_byte:
        call hex_digit
        jc   .out
        shl  al,4
        mov  ah,al
        call hex_digit
        jc   .out
        or   al,ah
.out:
        ret

hex_digit:
        cmp  si,cx
        jae  .bad
        lodsb
        cmp  al,'0'
        jb   .bad
        cmp  al,'9'
        jbe  .number
        or   al,20h             ; A-F as a-f
        cmp  al,'a'
        jb   .bad
        cmp  al,'f'
        ja   .bad
        sub  al,'a'-10
        clc
        ret
.number:
        sub  al,'0'
        clc
        ret
.bad:
        stc
        ret

text_usage db 'MODKEYS key ...  - a key is four hex digits, scan code then ASCII,',13,10
           db '                   with s, c or a in front for Shift, Ctrl or Alt held',13,10,'$'
