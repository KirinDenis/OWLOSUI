; ============================================================================
;  HELLO.ASM - a window, words, a button: the whole shape of a program.
;
;      FASM HELLO.ASM HELLO.COM
;      RUN HELLO
;
;  Build the windows, then wait for the person until the answer is OK.
;  Every window, frame and shadow is drawn by the OWLOSUI core; this
;  program only says what it wants, through INT 60h (OWLOSUI.INC).
; ============================================================================

        org  100h
        jmp  start
        include '..\..\..\lib\dos\asm\OWLOSUI.INC'

CM_OK = 1                       ; the program's own command number

start:
        call owl_check          ; started by OWLOSRES?

        owl_op   OP_INIT        ; the desktop: 80 by 25, as DOS's screen is
        owl_w    80
        owl_w    25
        owl_send

        owl_op   OP_WINDOW      ; a grey dialog, centred (-1, -1)
        owl_w    0              ; on the desktop
        owl_rect -1,-1,42,9
        owl_b    WF_DIALOG
        owl_s    'Hello'
        owl_send
        mov  [window],ax

        owl_op   OP_STATIC
        owl_w    [window]
        owl_rect 2,1,36,1
        owl_s    'Hello, world!'
        owl_send

        owl_op   OP_STATIC
        owl_w    [window]
        owl_rect 2,3,36,2
        owl_s    'This window is drawn by a Rust core and shown by assembler.'
        owl_send

        owl_op   OP_BUTTONS     ; one button, docked bottom right
        owl_w    [window]
        owl_b    1              ; how many
        owl_w    CM_OK
        owl_b    BF_DEFAULT     ; Enter presses it
        owl_s    '~O~K'
        owl_send

.loop:  call owl_wait           ; AX: a button, DX: a command
        cmp  ax,CM_OK
        je   .done
        cmp  dx,CM_OK
        jne  .loop

.done:  owl_op   OP_QUIT
        owl_send
        mov  ax,4C00h
        int  21h

window  dw 0
