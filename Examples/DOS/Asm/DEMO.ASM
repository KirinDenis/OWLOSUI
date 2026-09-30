; ============================================================================
;  DEMO.ASM - the OWLOS UI demo, in assembler.
;
;      FASM DEMO.ASM DEMO.COM
;      RUN DEMO
;
;  The shape every text-mode toolkit's demo has had, because it is the right
;  shape: a menu bar, a status line, a desktop, and small windows that each
;  use one part of the kit.
;
;    File      New: an editor window of its own. Open: the file panel,
;              the folder read through DOS, a file shown in a viewer. Exit.
;    Tools     Calculator  - a keypad of button rows; typing works too,
;                            because the window carries the keys it wants
;              Calendar    - a canvas drawn again for every month
;              ASCII table - every glyph of the font; a click names one
;              Puzzle      - the fifteen puzzle: click a tile, or arrows
;    Window    Zoom, Next, Previous, Close, List, Cascade, Tile
;    Help      About
;
;  The same demo is in Examples\DOS\C and Examples\DOS\Pascal, part for
;  part, and its bigger brother is the C# one in Examples\Desktop.
;
;  The toolkit - every frame, menu, button and the editor - is OWLOSRES,
;  reached through INT 60h (lib\dos\asm\OWLOSUI.INC). What is here is the
;  program: what to build, and what to do with each command. The
;  arithmetic is the 386's: OWLOSRES needs one anyway.
; ============================================================================

        org  100h
        jmp  start
        include '..\..\..\lib\dos\asm\OWLOSUI.INC'

; ---- the program's command numbers: its own, 0 means none -------------------
CM_NEW          = 1
CM_OPEN         = 3
CM_EXIT         = 2
CM_CALC         = 10
CM_CALENDAR     = 11
CM_ASCII        = 12
CM_PUZZLE       = 13
CM_NEXT         = 30
CM_ZOOM         = 31
CM_CLOSE        = 32
CM_CASCADE      = 33
CM_TILE         = 34
CM_PREVIOUS     = 35
CM_LIST         = 36
CM_ABOUT        = 40
CM_HELP         = 41
CM_DISMISS      = 62
CM_FILE_OPEN    = 63            ; the Open dialog's buttons
CM_OPEN_CANCEL  = 64
; the calculator's keys: 100..116
CM_DIGIT        = 100           ; 100..109: 0..9
CM_ADD          = 110
CM_SUB          = 111
CM_MUL          = 112
CM_DIV          = 113
CM_EQUALS       = 114
CM_CLEAR        = 115
CM_NEGATE       = 116
; the calendar's: 160..162
CM_MONTH_BACK   = 160
CM_MONTH_ON     = 161
CM_TODAY        = 162
; the puzzle's: 180..184
CM_SCRAMBLE     = 180
CM_UP           = 181
CM_DOWN         = 182
CM_LEFT         = 183
CM_RIGHT        = 184

; ============================================================================
;  the program
; ============================================================================

start:
        call owl_check

        owl_op   OP_INIT
        owl_w    80
        owl_w    25
        owl_send
        call owl_load_glyphs    ; the glyph table, for the ASCII table
        call build_bars
        call about              ; say hello

main_loop:
        call owl_wait           ; AX: a button pressed, DX: a command chosen
        push dx
        or   ax,ax
        jz   .chosen
        call command
        jc   .leave
.chosen:
        pop  ax
        or   ax,ax
        jz   .polled
        call command
        jc   quit
.polled:
        call ascii_poll         ; what is not a command: a click on a canvas
        call puzzle_poll
        call open_poll          ; and a name entered in the file panel
        jmp  main_loop
.leave: pop  ax
quit:   owl_op   OP_QUIT
        owl_send
        mov  ax,4C00h
        int  21h

; ---- the menu bar and the status line ----------------------------------------
build_bars:
        owl_op   OP_MENU_BAR
        owl_b    4
        owl_menu '~F~ile',4
        owl_item '~N~ew',,'An editor window of its own',CM_NEW
        owl_item '~O~pen...','F3','A file from the disk, in a viewer',CM_OPEN
        owl_line
        owl_item 'E~x~it','Alt+X','Leave the program',CM_EXIT
        owl_menu '~T~ools',4
        owl_item '~C~alculator',,'Add, take, times, share: type or click',CM_CALC
        owl_item 'Ca~l~endar',,'A month at a time; PgUp and PgDn page',CM_CALENDAR
        owl_item '~A~SCII table',,'Every glyph of the font; click one',CM_ASCII
        owl_item '~P~uzzle',,'The fifteen puzzle: click a tile, or arrows',CM_PUZZLE
        owl_menu '~W~indow',8
        owl_item '~Z~oom','F5','The window fills the desktop, or goes back',CM_ZOOM
        owl_item '~N~ext','F6','The front window goes to the back',CM_NEXT
        owl_item '~P~revious','Shift+F6','The window at the back comes to the front',CM_PREVIOUS
        owl_item '~C~lose','Alt+F3','Close the front window',CM_CLOSE
        owl_item '~L~ist...','Alt+0','Every window by number',CM_LIST
        owl_line
        owl_item 'C~a~scade',,'The windows along the diagonal',CM_CASCADE
        owl_item '~T~ile',,'The windows share the desktop',CM_TILE
        owl_menu '~H~elp',1
        owl_item '~A~bout',,'What this program is and what draws it',CM_ABOUT
        owl_send

        owl_op   OP_STATUS
        owl_b    6
        owl_status_item '~F1~ Help',CM_HELP,KEY_F,1,0
        owl_status_item '~F3~ Open',CM_OPEN,KEY_F,3,0
        owl_status_item '~F5~ Zoom',CM_ZOOM,KEY_F,5,0
        owl_status_item '~F6~ Next',CM_NEXT,KEY_F,6,0
        owl_status_item '~Alt-F3~ Close',CM_CLOSE,KEY_F,3,MOD_ALT
        owl_status_item '~Alt-X~ Exit',CM_EXIT,KEY_CHAR,'x',MOD_ALT
        owl_send
        ret

; ---- one command: AX. CF set means leave -------------------------------------
command:
        cmp  ax,CM_EXIT
        je   .leave
        cmp  ax,CM_DIGIT
        jb   .not_calc
        cmp  ax,CM_NEGATE
        ja   .not_calc
        call calc_key
        jmp  .done
.not_calc:
        cmp  ax,CM_MONTH_BACK
        jb   .not_cal
        cmp  ax,CM_TODAY
        ja   .not_cal
        call calendar_key
        jmp  .done
.not_cal:
        cmp  ax,CM_SCRAMBLE
        jb   .not_puzzle
        cmp  ax,CM_RIGHT
        ja   .not_puzzle
        call puzzle_key
        jmp  .done
.not_puzzle:
        mov  si,commands        ; the rest: a table of command and routine
.find:  cmp  word [si],0
        je   .done
        cmp  [si],ax
        je   .found
        add  si,4
        jmp  .find
.found: call word [si+2]
.done:  clc
        ret
.leave: stc
        ret

commands:
        dw CM_NEW,editor_new
        dw CM_OPEN,open_show
        dw CM_FILE_OPEN,open_button
        dw CM_OPEN_CANCEL,open_close
        dw CM_CALC,calc_show
        dw CM_CALENDAR,calendar_show
        dw CM_ASCII,ascii_show
        dw CM_PUZZLE,puzzle_show
        dw CM_ABOUT,about
        dw CM_HELP,about
        dw CM_DISMISS,box_close
        dw CM_NEXT,window_next
        dw CM_PREVIOUS,window_previous
        dw CM_ZOOM,window_zoom
        dw CM_CLOSE,window_close
        dw CM_CASCADE,window_cascade
        dw CM_TILE,window_tile
        dw CM_LIST,window_list
        dw 0

; ---- the desktop's own verbs -------------------------------------------------
window_next:
        owl_op   OP_CYCLE
        owl_send
        ret
window_previous:
        owl_op   OP_CYCLE_BACK
        owl_send
        ret
window_cascade:
        owl_op   OP_CASCADE
        owl_send
        ret
window_tile:
        owl_op   OP_TILE
        owl_send
        ret
window_list:
        owl_op   OP_WINDOW_LIST
        owl_send
        ret
window_zoom:
        owl_op   OP_ACTIVE
        owl_send
        or   ax,ax
        jz   .none
        mov  bx,ax
        owl_op   OP_ZOOM
        owl_w    bx
        owl_send
.none:  ret

; The front window is being closed: whoever owns it forgets it.
window_close:
        owl_op   OP_ACTIVE
        owl_send
        or   ax,ax
        jz   .none
        mov  bx,ax
        mov  si,tool_windows
.forget:
        cmp  word [si],0FFFFh
        je   .close
        mov  di,[si]
        cmp  [di],bx
        jne  .next
        mov  word [di],0
.next:  add  si,2
        jmp  .forget
.close: owl_op   OP_CLOSE
        owl_w    bx
        owl_send
.none:  ret

tool_windows dw calc_win,cal_win,ascii_win,puz_win,box,open_dialog,0FFFFh

; ---- About, and the one message box ------------------------------------------
about:
        call box_close
        owl_op   OP_MESSAGE_BOX
        owl_s    'OWLOS UI Demo'
        owl_s    'An assembler program on DOS. Every window in it is drawn by the OWLOSUI core, a Rust program behind INT 60h - the same core that draws the C# demo on Windows and the JavaScript one in a browser.'
        owl_b    1
        owl_button CM_DISMISS,BF_DEFAULT,'~O~K'
        owl_send
        mov  [box],ax
        ret

box_close:
        mov  bx,[box]
        or   bx,bx
        jz   .none
        owl_op   OP_CLOSE
        owl_w    bx
        owl_send
        mov  word [box],0
.none:  ret

; ---- File > New: an editor, a window like any other --------------------------
editor_new:
        inc  word [editors]
        mov  di,title_buf       ; UNTITLEDn.TXT
        mov  si,s_untitled
        call copy_z
        movzx eax,word [editors]
        call itoa
        mov  si,s_txt
        call copy_z
        mov  bx,[editors]
        and  bx,7
        owl_op   OP_WINDOW
        owl_w    0
        mov  ax,bx
        add  ax,2
        call owl_word           ; x
        mov  ax,bx
        inc  ax
        call owl_word           ; y
        owl_w    56
        owl_w    14
        owl_b    WF_BLUE
        mov  si,title_buf
        call owl_zstr
        owl_w    CM_CLOSE
        owl_send
        mov  bx,ax
        owl_op   OP_TEXT
        owl_w    bx
        owl_rect 0,0,0,0
        owl_b    0              ; fill the window
        owl_b    0              ; editable
        owl_s    'Type here. Shift and the arrows select, Ctrl+Z takes it back. The Edit menu has Find, Replace and Word wrap.'
        owl_send
        ; Everything the editor has, on an Edit menu of its own while this
        ; window is in front: the core runs it all, nothing comes back here.
        mov  bx,ax
        mov  ax,OFFER_ALL       ; AH 0: starts plain
        call owl_editor
        ret

; ============================================================================
;  File > Open: the file panel, the folder read through DOS
; ============================================================================

open_show:
        mov  bx,[open_dialog]
        or   bx,bx
        jz   .new
        jmp  activate
.new:   cmp  byte [open_dir],0
        jne  .known
        mov  ah,19h             ; the current drive: 0 is A
        int  21h
        add  al,'A'
        mov  [open_dir],al
        mov  word [open_dir+1],':\'
        mov  ah,47h             ; and its current folder, after the backslash
        xor  dl,dl
        mov  si,open_dir+3
        int  21h
.known: owl_op   OP_WINDOW
        owl_w    0
        owl_rect -1,-1,70,20
        owl_b    WF_DIALOG or WF_MODAL
        owl_s    'Open'
        owl_w    CM_OPEN_CANCEL
        owl_send
        mov  [open_dialog],ax
        mov  bx,ax
        mov  dx,1               ; a row of air above the panel
        mov  si,open_dir
        call owl_files          ; DOS reads the folder, the panel shows it
        mov  [open_panel],ax
        owl_op   OP_BUTTONS
        owl_w    bx
        owl_b    2
        owl_button CM_FILE_OPEN,BF_DEFAULT,'~O~pen'
        owl_button CM_OPEN_CANCEL,BF_CANCEL,'~C~ancel'
        owl_send
        ret

open_close:
        mov  bx,[open_dialog]
        or   bx,bx
        jz   .gone
        owl_op   OP_CLOSE
        owl_w    bx
        owl_send
.gone:  mov  word [open_dialog],0
        mov  word [open_panel],0
        ret

; SI: a name. path_buf: it made into a whole path - a drive's path as it
; is, anything else from the folder the panel shows.
join:
        push si
        mov  di,path_buf
        cmp  byte [si+1],':'
        je   .name
        cmp  byte [si],'\'
        jne  .relative
        mov  ax,word [open_dir] ; the drive, C:
        stosw
        jmp  .name
.relative:
        push si
        mov  si,open_dir
        call copy_z
        pop  si
        cmp  byte [di-1],'\'
        je   .name
        mov  byte [di],'\'
        inc  di
.name:  call copy_z
        pop  si
        ret

; SI: a string. CF set if it has a mask in it, * or ?.
has_mask:
        push si
.char:  lodsb
        or   al,al
        jz   .no
        cmp  al,'*'
        je   .yes
        cmp  al,'?'
        jne  .char
.yes:   pop  si
        stc
        ret
.no:    pop  si
        clc
        ret

; SI: a path, cut in place to its folder - C:\A\B to C:\A, C:\A to C:\.
cut_last:
        push bx
        push si
        xor  bx,bx              ; the last backslash
.scan:  lodsb
        or   al,al
        jz   .cut
        cmp  al,'\'
        jne  .scan
        lea  bx,[si-1]
        jmp  .scan
.cut:   pop  si
        or   bx,bx
        jz   .done
        mov  ax,bx
        sub  ax,si
        cmp  ax,2
        jne  .here
        inc  bx                 ; C:\ keeps its backslash
.here:  mov  byte [bx],0
.done:  pop  bx
        ret

; SI: a path. SI is left on the name after its last backslash.
last_name:
        push bx
        mov  bx,si
.scan:  lodsb
        or   al,al
        jz   .done
        cmp  al,'\'
        jne  .scan
        mov  bx,si
        jmp  .scan
.done:  mov  si,bx
        pop  bx
        ret

; SI: a folder for the panel - or says why it cannot be. A path with a
; mask on the end, as the path line shows it, means its folder.
go_to:
        mov  di,temp_path
        call copy_z
        mov  si,temp_path
        call has_mask
        jnc  .plain
        call cut_last
.plain: mov  bx,[open_panel]
        call owl_set_files
        jc   .missing
        mov  di,open_dir
        jmp  copy_z
.missing:
        mov  di,msg_buf
        push si
        mov  si,s_no_folder
        call copy_z
        pop  si
        call copy_z
        mov  si,msg_buf
        mov  bx,[open_panel]
        jmp  owl_files_error

; "..": the folder above.
go_up:
        mov  si,open_dir
        mov  di,up_path
        call copy_z
        mov  si,up_path
        call cut_last
        jmp  go_to

; SI: a name entered in the panel, or a path typed: a folder is walked
; into, a file is opened and the dialog closes.
chosen:
        cmp  word [si],'..'
        jne  .named
        cmp  byte [si+2],0
        jne  .named
        jmp  go_up
.named: call join
        mov  dx,path_buf
        mov  ax,4300h           ; its attributes, from DOS
        int  21h
        jc   .missing
        test cl,10h
        jz   .file
        mov  si,path_buf
        jmp  go_to
.file:  call view_file
        jc   .unreadable
        jmp  open_close
.missing:
        mov  si,s_not_found
        jmp  .say
.unreadable:
        mov  si,s_cannot_read
.say:   mov  di,msg_buf
        call copy_z
        mov  si,path_buf
        call copy_z
        mov  si,msg_buf
        mov  bx,[open_panel]
        jmp  owl_files_error

; The file at path_buf into a viewer window of its own: its first 8 KB,
; read by DOS. CF set if DOS cannot read it.
view_file:
        mov  dx,path_buf
        mov  ax,3D00h
        int  21h
        jc   .fail
        mov  bx,ax
        mov  ah,3Fh
        mov  cx,FILE_BUF_SIZE
        mov  dx,file_buf
        int  21h
        jc   .close_fail
        mov  [file_n],ax
        mov  ax,4202h           ; the size: how far the end is
        xor  cx,cx
        xor  dx,dx
        int  21h
        mov  word [file_size],ax
        mov  word [file_size+2],dx
        mov  ah,3Eh
        int  21h
        mov  si,path_buf        ; the title: the name, and how much of it
        call last_name
        mov  di,title_buf
        call copy_z
        cmp  word [file_size+2],0
        jne  .part
        mov  ax,word [file_size]
        cmp  ax,[file_n]
        jbe  .whole
.part:  mov  si,s_first
        call copy_z
        movzx eax,word [file_n]
        call itoa
        mov  si,s_bytes
        call copy_z
.whole: owl_op   OP_WINDOW
        owl_w    0
        owl_rect 2,2,76,20
        owl_b    WF_BLUE
        mov  si,title_buf
        call owl_zstr
        owl_w    CM_CLOSE
        owl_send
        mov  bx,ax
        mov  si,file_buf
        mov  cx,[file_n]
        mov  al,1               ; read-only: a viewer
        call owl_text_dos
        ; With Find, Word wrap and the bytes in hex on its Edit menu.
        mov  bx,ax
        mov  ax,(EDIT_READONLY shl 8) or OFFER_FIND or OFFER_WRAP or OFFER_HEX
        call owl_editor
        clc
        ret
.close_fail:
        mov  ah,3Eh
        int  21h
.fail:  stc
        ret

; What the panel reports is not a command: asked for after every event.
open_poll:
        cmp  word [open_dialog],0
        je   .none
        mov  bx,[open_panel]
        call owl_take_files     ; AL: what happened; the text at OWL_TEXT
        cmp  al,1
        je   .name
        cmp  al,2
        jne  .none
        mov  si,OWL_TEXT        ; a path typed: made whole first
        call join
        mov  si,path_buf
        call has_mask
        jnc  .whole
        jmp  go_to
.whole: mov  di,name_buf
        call copy_z
        mov  si,name_buf
        jmp  chosen
.name:  mov  si,OWL_TEXT
        mov  di,name_buf
        call copy_z
        mov  si,name_buf
        jmp  chosen
.none:  ret

; The Open button: whatever is under the cursor.
open_button:
        mov  bx,[open_panel]
        or   bx,bx
        jz   .none
        call owl_current_name
        jc   .none
        mov  si,OWL_TEXT
        mov  di,name_buf
        call copy_z
        mov  si,name_buf
        jmp  chosen
.none:  ret

; ============================================================================
;  the calculator
; ============================================================================

calc_show:
        mov  bx,[calc_win]
        or   bx,bx
        jz   .new
        jmp  activate
.new:   owl_op   OP_WINDOW
        owl_w    0
        owl_rect -1,-1,46,13
        owl_b    WF_DIALOG
        owl_s    'Calculator'
        owl_w    CM_CLOSE
        owl_send
        mov  [calc_win],ax
        mov  bx,ax

        ; The display: a canvas one line high, so the number can stand at
        ; its right-hand end - a static line would fold the spaces away.
        owl_op   OP_CANVAS
        owl_w    bx
        owl_rect 2,1,40,1
        owl_send
        mov  [calc_display],ax

        ; The keypad: four rows, each a BUTTON_ROW, labels padded to three
        ; so the columns line up. Bit 0 of the row's flags keeps it out of
        ; the Tab ring: a keypad is for the mouse and for typing.
        owl_op   OP_BUTTON_ROW
        owl_w    bx
        owl_rect 2,3,0,0
        owl_b    1
        owl_b    5
        owl_button CM_DIGIT+7,0,' 7 '
        owl_button CM_DIGIT+8,0,' 8 '
        owl_button CM_DIGIT+9,0,' 9 '
        owl_button CM_DIV,BF_ACCENT,' / '
        owl_button CM_CLEAR,BF_DANGER,' C '
        owl_send
        owl_op   OP_BUTTON_ROW
        owl_w    bx
        owl_rect 2,5,0,0
        owl_b    1
        owl_b    4
        owl_button CM_DIGIT+4,0,' 4 '
        owl_button CM_DIGIT+5,0,' 5 '
        owl_button CM_DIGIT+6,0,' 6 '
        owl_button CM_MUL,BF_ACCENT,' * '
        owl_send
        owl_op   OP_BUTTON_ROW
        owl_w    bx
        owl_rect 2,7,0,0
        owl_b    1
        owl_b    4
        owl_button CM_DIGIT+1,0,' 1 '
        owl_button CM_DIGIT+2,0,' 2 '
        owl_button CM_DIGIT+3,0,' 3 '
        owl_button CM_SUB,BF_ACCENT,' - '
        owl_send
        owl_op   OP_BUTTON_ROW
        owl_w    bx
        owl_rect 2,9,0,0
        owl_b    1
        owl_b    4
        owl_button CM_DIGIT+0,0,' 0 '
        owl_button CM_NEGATE,BF_ACCENT,'+/-'
        owl_button CM_EQUALS,BF_ACCENT or BF_DEFAULT,' = '
        owl_button CM_ADD,BF_ACCENT,' + '
        owl_send

        ; The keys the window carries: while it is in front, typing a digit
        ; or an operator is its command. No labels, so the status line
        ; keeps its own words; Enter is the default button, =.
        owl_op   OP_WINDOW_STATUS
        owl_w    bx
        owl_b    17
        mov  cx,10
        mov  dx,'0'
.digit_keys:
        mov  ax,dx
        sub  ax,'0'-CM_DIGIT
        call owl_word           ; cmd
        owl_b    KEY_CHAR
        mov  ax,dx
        call owl_word           ; the digit
        owl_b    0
        owl_empty
        inc  dx
        loop .digit_keys
        owl_status_item ,CM_ADD,KEY_CHAR,'+',0
        owl_status_item ,CM_SUB,KEY_CHAR,'-',0
        owl_status_item ,CM_MUL,KEY_CHAR,'*',0
        owl_status_item ,CM_DIV,KEY_CHAR,'/',0
        owl_status_item ,CM_EQUALS,KEY_CHAR,'=',0
        owl_status_item ,CM_CLEAR,KEY_NAMED,1,0         ; Esc
        owl_status_item ,CM_CLEAR,KEY_CHAR,'c',0
        owl_send

        mov  bx,CM_CLEAR
        mov  ax,bx
        jmp  calc_key

; One key of the calculator: AX. A pocket calculator's arithmetic, on 32-bit
; integers: an accumulator, the operation waiting, and the number being typed.
calc_key:
        cmp  word [calc_win],0
        je   .none
        cmp  ax,CM_CLEAR
        je   .clear
        cmp  byte [calc_error],0
        jne  .none              ; after an error only C does anything
        cmp  ax,CM_ADD
        jb   .digit
        cmp  ax,CM_EQUALS
        je   .equals
        cmp  ax,CM_NEGATE
        je   .negate
        ; + - * /: what was waiting is done, and this one waits
        push ax
        call calc_apply
        pop  ax
        mov  [calc_op],al
        jmp  calc_show_acc
.equals:
        call calc_apply
        mov  byte [calc_op],0
        mov  eax,[calc_acc]
        mov  [calc_entry],eax
        jmp  calc_show_acc
.negate:
        cmp  byte [calc_typing],0
        je   .negate_acc
        neg  dword [calc_entry]
        jmp  calc_show_entry
.negate_acc:
        neg  dword [calc_acc]
        mov  eax,[calc_acc]
        mov  [calc_entry],eax
        jmp  calc_show_acc
.digit:
        sub  ax,CM_DIGIT
        movzx ebx,ax
        cmp  byte [calc_typing],0
        jne  .more
        mov  dword [calc_entry],0
        mov  byte [calc_typing],1
.more:  mov  eax,[calc_entry]
        cmp  eax,99999999       ; nine digits are enough for a display
        jg   calc_show_entry
        cmp  eax,-99999999
        jl   calc_show_entry
        imul eax,10
        or   eax,eax
        jns  .plus
        sub  eax,ebx
        jmp  .typed
.plus:  add  eax,ebx
.typed: mov  [calc_entry],eax
        jmp  calc_show_entry
.clear:
        xor  eax,eax
        mov  [calc_acc],eax
        mov  [calc_entry],eax
        mov  byte [calc_op],0
        mov  byte [calc_typing],0
        mov  byte [calc_error],0
        jmp  calc_show_acc
.none:  ret

; acc = acc (op) entry, if something was typed or waits; an overflow or a
; division by nought is an error.
calc_apply:
        mov  eax,[calc_acc]
        mov  ebx,[calc_entry]
        mov  cl,[calc_op]
        cmp  cl,0
        jne  .op
        mov  eax,ebx            ; nothing waiting: the number is the total
        jmp  .keep
.op:    cmp  byte [calc_typing],0
        je   .keep              ; two operators in a row: the second wins
        cmp  cl,CM_ADD and 0FFh
        jne  .sub
        add  eax,ebx
        jo   .error
        jmp  .keep
.sub:   cmp  cl,CM_SUB and 0FFh
        jne  .mul
        sub  eax,ebx
        jo   .error
        jmp  .keep
.mul:   cmp  cl,CM_MUL and 0FFh
        jne  .div
        imul eax,ebx
        jo   .error
        jmp  .keep
.div:   or   ebx,ebx
        jz   .error
        cmp  ebx,-1             ; the one quotient that does not fit
        jne  .divide
        cmp  eax,80000000h
        je   .error
.divide:
        cdq
        idiv ebx
.keep:  mov  [calc_acc],eax
        mov  byte [calc_typing],0
        ret
.error: mov  byte [calc_error],1
        mov  byte [calc_typing],0
        ret

calc_show_acc:
        mov  eax,[calc_acc]
        jmp  calc_print
calc_show_entry:
        mov  eax,[calc_entry]
calc_print:
        mov  di,num_buf
        cmp  byte [calc_error],0
        je   .number
        mov  si,s_error
        call copy_z
        jmp  .pad
.number:
        call itoa
.pad:   ; right-aligned in forty columns, white on blue
        mov  si,num_buf
        mov  di,cells
        mov  cx,40
        call right_align
        mov  di,cells+40
        mov  cx,40
        mov  al,1Fh
        rep  stosb
        mov  bx,[calc_display]
        mov  dx,40
        mov  cx,1
        jmp  blit_cells

; ============================================================================
;  the calendar
; ============================================================================

calendar_show:
        mov  bx,[cal_win]
        or   bx,bx
        jz   .new
        jmp  activate
.new:   mov  ah,2Ah             ; today, from DOS
        int  21h
        mov  [today_year],cx
        mov  [today_month],dh
        mov  [today_day],dl
        mov  [cal_year],cx
        mov  [cal_month],dh

        owl_op   OP_WINDOW
        owl_w    0
        owl_rect -1,-1,40,15
        owl_b    WF_DIALOG
        owl_s    'Calendar'
        owl_w    CM_CLOSE
        owl_send
        mov  [cal_win],ax
        mov  bx,ax
        owl_op   OP_STATIC
        owl_w    bx
        owl_rect 9,1,22,1
        owl_s    ' '
        owl_send
        mov  [cal_title],ax
        owl_op   OP_CANVAS
        owl_w    bx
        owl_rect 8,3,21,7
        owl_send
        mov  [cal_canvas],ax
        owl_op   OP_BUTTONS
        owl_w    bx
        owl_b    3
        owl_button CM_MONTH_BACK,0,'< ~B~ack'
        owl_button CM_TODAY,BF_DEFAULT,'~T~oday'
        owl_button CM_MONTH_ON,0,'~O~n >'
        owl_send
        owl_op   OP_WINDOW_STATUS
        owl_w    bx
        owl_b    3
        owl_status_item ,CM_MONTH_BACK,KEY_NAMED,9,0    ; PgUp
        owl_status_item ,CM_MONTH_ON,KEY_NAMED,10,0     ; PgDn
        owl_status_item ,CM_TODAY,KEY_NAMED,7,0         ; Home
        owl_send
        jmp  calendar_draw

calendar_key:
        cmp  word [cal_win],0
        je   .none
        cmp  ax,CM_TODAY
        jne  .back
        mov  ax,[today_year]
        mov  [cal_year],ax
        mov  al,[today_month]
        mov  [cal_month],al
        jmp  calendar_draw
.back:  cmp  ax,CM_MONTH_BACK
        jne  .on
        dec  byte [cal_month]
        jnz  calendar_draw
        mov  byte [cal_month],12
        dec  word [cal_year]
        jmp  calendar_draw
.on:    inc  byte [cal_month]
        cmp  byte [cal_month],13
        jb   calendar_draw
        mov  byte [cal_month],1
        inc  word [cal_year]
        jmp  calendar_draw
.none:  ret

; The month into the canvas: a row of day names, then up to six weeks.
calendar_draw:
        ; the title: "September 2026"
        movzx bx,byte [cal_month]
        dec  bx
        shl  bx,1
        mov  si,[month_names+bx]
        mov  di,num_buf
        call copy_z
        mov  byte [di],' '
        inc  di
        movzx eax,word [cal_year]
        call itoa
        owl_op   OP_SET_TEXT
        owl_w    [cal_title]
        mov  si,num_buf
        call owl_zstr
        owl_send

        ; the cells: blank, then the day names
        mov  di,cells
        mov  cx,21*7
.blank: mov  byte [di],' '
        mov  byte [di+21*7],70h ; black on grey
        inc  di
        loop .blank
        mov  si,s_daynames
        mov  di,cells
        mov  cx,21
.names: lodsb
        mov  [di],al
        mov  byte [di+21*7],71h ; blue on grey
        inc  di
        loop .names

        ; where the 1st falls, and how many days
        mov  ax,[cal_year]
        mov  bl,[cal_month]
        mov  bh,1
        call day_of_week        ; AX = 0 Sunday .. 6
        mov  [cal_first],al
        call days_in_month      ; AX
        mov  [cal_days],al

        mov  cl,1               ; the day
.day:   movzx ax,byte [cal_first]
        add  al,cl
        dec  al                 ; its place in the grid, from the first Sunday
        mov  bl,7
        div  bl                 ; AL = the week, AH = the weekday
        mov  dl,70h             ; black on grey; Sundays red
        or   ah,ah
        jnz  .weekday
        mov  dl,74h
.weekday:
        movzx di,al
        inc  di                 ; below the names
        imul di,di,21
        movzx bx,ah
        imul bx,bx,3
        add  di,bx
        add  di,cells
        movzx ax,cl             ; two digits, right-aligned
        mov  bl,10
        div  bl                 ; AL = tens, AH = units
        add  ax,3030h
        cmp  al,'0'
        jne  .tens
        mov  al,' '
.tens:  mov  [di],al
        mov  [di+1],ah
        mov  ax,[cal_year]      ; today: white on green
        cmp  ax,[today_year]
        jne  .colour
        mov  al,[cal_month]
        cmp  al,[today_month]
        jne  .colour
        cmp  cl,[today_day]
        jne  .colour
        mov  dl,2Fh
.colour:
        mov  [di+21*7],dl
        mov  [di+21*7+1],dl
        inc  cl
        cmp  cl,[cal_days]
        jbe  .day

        mov  bx,[cal_canvas]
        mov  dx,21
        mov  cx,7
        jmp  blit_cells

; AX year, BL month, BH day -> AX: 0 Sunday .. 6 Saturday. Sakamoto's:
; (y + y/4 - y/100 + y/400 + t[m] + d) mod 7, with January and February
; counted in the year before.
day_of_week:
        push bx
        push cx
        push dx
        push si
        movzx cx,bh
        push cx                 ; the day
        cmp  bl,3
        jae  .late
        dec  ax
.late:  mov  si,ax
        mov  cx,ax
        shr  cx,2
        add  si,cx              ; + y/4
        push ax
        xor  dx,dx
        mov  cx,100
        div  cx
        sub  si,ax              ; - y/100
        shr  ax,2
        add  si,ax              ; + y/400
        pop  ax
        movzx bx,bl
        movzx ax,byte [dow_table+bx-1]
        add  si,ax
        pop  cx
        add  si,cx
        mov  ax,si
        xor  dx,dx
        mov  cx,7
        div  cx
        mov  ax,dx
        pop  si
        pop  dx
        pop  cx
        pop  bx
        ret
dow_table db 0,3,2,5,0,3,5,1,4,6,2,4

; [cal_year], [cal_month] -> AX: the days in that month.
days_in_month:
        movzx bx,byte [cal_month]
        movzx ax,byte [month_days+bx-1]
        cmp  bx,2
        jne  .done
        mov  cx,[cal_year]
        call is_leap
        jnc  .done
        inc  ax
.done:  ret

; CX: a year. CF set if it is a leap year: by 4, not by 100 unless by 400.
is_leap:
        push ax
        push bx
        push dx
        test cx,3
        jnz  .no
        mov  ax,cx
        xor  dx,dx
        mov  bx,100
        div  bx
        or   dx,dx
        jnz  .yes
        test ax,3
        jnz  .no
.yes:   pop  dx
        pop  bx
        pop  ax
        stc
        ret
.no:    pop  dx
        pop  bx
        pop  ax
        clc
        ret

; ============================================================================
;  the ASCII table
; ============================================================================

ascii_show:
        mov  bx,[ascii_win]
        or   bx,bx
        jz   .new
        jmp  activate
.new:   owl_op   OP_WINDOW
        owl_w    0
        owl_rect -1,-1,39,21
        owl_b    WF_DIALOG
        owl_s    'ASCII table'
        owl_w    CM_CLOSE
        owl_send
        mov  [ascii_win],ax
        mov  bx,ax
        owl_op   OP_CANVAS
        owl_w    bx
        owl_rect 1,1,35,17
        owl_send
        mov  [ascii_canvas],ax
        owl_op   OP_STATIC
        owl_w    bx
        owl_rect 2,18,33,1
        owl_s    'Click a glyph'
        owl_send
        mov  [ascii_info],ax

        ; 35 by 17: the column numbers across the top, the row numbers
        ; down the side, and every glyph two columns apart.
        mov  di,cells
        mov  cx,35*17
.blank: mov  byte [di],' '
        mov  byte [di+35*17],70h
        inc  di
        loop .blank
        xor  bx,bx              ; the column numbers
.cols:  mov  di,bx
        shl  di,1
        add  di,cells+3
        mov  al,[hex_digits+bx]
        mov  [di],al
        mov  byte [di+35*17],74h
        inc  bx
        cmp  bx,16
        jb   .cols
        xor  bx,bx              ; each row: its number, then sixteen glyphs
.rows:  mov  di,bx
        inc  di
        imul di,35
        add  di,cells
        mov  al,[hex_digits+bx]
        mov  [di],al
        mov  byte [di+1],'0'
        mov  byte [di+35*17],74h
        mov  byte [di+35*17+1],74h
        add  di,3
        mov  cx,16
.glyph: mov  byte [di],0FFh     ; a marker: the glyph goes here (see blit)
        mov  byte [di+35*17],1Eh ; yellow on blue
        add  di,2
        loop .glyph
        inc  bx
        cmp  bx,16
        jb   .rows
        mov  bx,[ascii_canvas]
        mov  dx,35
        mov  cx,17
        mov  byte [blit_glyphs],1
        call blit_cells
        mov  byte [blit_glyphs],0
        ret

; A click on the table: which glyph, in every base.
ascii_poll:
        mov  bx,[ascii_win]
        or   bx,bx
        jz   .none
        owl_op   OP_GET_CLICK
        owl_w    [ascii_canvas]
        owl_send
        cmp  byte [OWL_REPLY+3],0
        je   .none
        mov  ax,[OWL_REPLY+4]   ; x
        mov  dx,[OWL_REPLY+6]   ; y
        sub  ax,3
        jb   .none
        test ax,1
        jnz  .none
        shr  ax,1
        cmp  ax,16
        jae  .none
        dec  dx
        cmp  dx,16
        jae  .none
        shl  dx,4
        add  ax,dx              ; the glyph's number
        mov  [ascii_code],ax
        ; "dec 65  hex 41  A"
        mov  di,num_buf
        mov  si,s_dec
        call copy_z
        movzx eax,word [ascii_code]
        call itoa
        mov  si,s_hex
        call copy_z
        mov  bx,[ascii_code]
        shr  bx,4
        mov  al,[hex_digits+bx]
        stosb
        mov  bx,[ascii_code]
        and  bx,15
        mov  al,[hex_digits+bx]
        stosb
        mov  ax,'  '
        stosw
        mov  bx,[ascii_code]
        shl  bx,1
        mov  ax,[OWL_GLYPH_TABLE+bx]     ; the glyph as Unicode, then as UTF-8
        call utf8
        mov  byte [di],0
        owl_op   OP_SET_TEXT
        owl_w    [ascii_info]
        mov  si,num_buf
        call owl_zstr
        owl_send
.none:  ret

; ============================================================================
;  the fifteen puzzle
; ============================================================================

puzzle_show:
        mov  bx,[puz_win]
        or   bx,bx
        jz   .new
        jmp  activate
.new:   owl_op   OP_WINDOW
        owl_w    0
        owl_rect -1,-1,30,18
        owl_b    WF_DIALOG
        owl_s    'Puzzle'
        owl_w    CM_CLOSE
        owl_send
        mov  [puz_win],ax
        mov  bx,ax
        owl_op   OP_CANVAS
        owl_w    bx
        owl_rect 3,1,24,12
        owl_send
        mov  [puz_canvas],ax
        owl_op   OP_STATIC
        owl_w    bx
        owl_rect 3,13,24,1
        owl_s    ' '
        owl_send
        mov  [puz_moves_text],ax
        owl_op   OP_BUTTONS
        owl_w    bx
        owl_b    1
        owl_button CM_SCRAMBLE,BF_DEFAULT,'~S~cramble'
        owl_send
        owl_op   OP_WINDOW_STATUS
        owl_w    bx
        owl_b    4
        owl_status_item ,CM_UP,KEY_NAMED,11,0
        owl_status_item ,CM_DOWN,KEY_NAMED,12,0
        owl_status_item ,CM_LEFT,KEY_NAMED,13,0
        owl_status_item ,CM_RIGHT,KEY_NAMED,14,0
        owl_send
        mov  ax,CM_SCRAMBLE
        ; fall through

puzzle_key:
        cmp  word [puz_win],0
        je   .none
        cmp  ax,CM_SCRAMBLE
        jne  .arrow
        call puzzle_scramble
        jmp  puzzle_draw
.arrow: call puzzle_arrow
        jmp  puzzle_draw
.none:  ret

; An arrow: the tile beside the hole on the far side moves into it.
puzzle_arrow:
        call puzzle_hole        ; BX = the hole
        mov  si,bx
        cmp  ax,CM_UP
        jne  .down
        add  si,4
        cmp  si,16
        jae  .none
        jmp  puzzle_slide
.down:  cmp  ax,CM_DOWN
        jne  .left
        sub  si,4
        jb   .none
        jmp  puzzle_slide
.left:  cmp  ax,CM_LEFT
        jne  .right
        mov  ax,bx
        and  ax,3
        cmp  ax,3
        je   .none
        inc  si
        jmp  puzzle_slide
.right: mov  ax,bx
        and  ax,3
        jz   .none
        dec  si
        jmp  puzzle_slide
.none:  ret

; SI: a tile. If it is next to the hole, it slides into it: CF clear.
puzzle_slide:
        call puzzle_hole
        mov  ax,si              ; same row, one apart, or one row apart
        sub  ax,bx
        cmp  ax,4
        je   .swap
        cmp  ax,-4
        je   .swap
        cmp  ax,1
        je   .row
        cmp  ax,-1
        jne  .no
.row:   mov  ax,si
        shr  ax,2
        mov  dx,bx
        shr  dx,2
        cmp  ax,dx
        jne  .no
.swap:  mov  al,[board+si]
        mov  [board+bx],al
        mov  byte [board+si],0
        inc  word [puz_moves]
        clc
        ret
.no:    stc
        ret

; BX = where the hole is.
puzzle_hole:
        xor  bx,bx
.find:  cmp  byte [board+bx],0
        je   .found
        inc  bx
        cmp  bx,15
        jb   .find
.found: ret

; Solved, then three hundred random slides: every scramble can be undone.
puzzle_scramble:
        xor  bx,bx
.solve: lea  ax,[bx+1]
        mov  [board+bx],al
        inc  bx
        cmp  bx,15
        jb   .solve
        mov  byte [board+15],0
        push es                 ; the seed: the BIOS clock
        xor  ax,ax
        mov  es,ax
        mov  ax,[es:046Ch]
        pop  es
        mov  [seed],ax
        mov  cx,300
.shuffle:
        mov  ax,[seed]          ; a 16-bit LCG is plenty for a shuffle
        mov  dx,25173
        mul  dx
        add  ax,13849
        mov  [seed],ax
        mov  al,ah
        and  ax,3
        add  ax,CM_UP
        push cx
        call puzzle_arrow
        pop  cx
        loop .shuffle
        mov  word [puz_moves],0
        ret

; A click on the board: the tile under it, slid if it can go.
puzzle_poll:
        cmp  word [puz_win],0
        je   .none
        owl_op   OP_GET_CLICK
        owl_w    [puz_canvas]
        owl_send
        cmp  byte [OWL_REPLY+3],0
        je   .none
        mov  ax,[OWL_REPLY+4]   ; x: six columns a tile
        mov  bl,6
        div  bl
        movzx si,al
        mov  ax,[OWL_REPLY+6]   ; y: three rows a tile
        mov  bl,3
        div  bl
        movzx ax,al
        shl  ax,2
        add  si,ax
        cmp  si,16
        jae  .none
        call puzzle_slide
        jmp  puzzle_draw
.none:  ret

; The board into the canvas: a tile is five by two in blue, with a column
; and a row of the window's grey between tiles.
puzzle_draw:
        xor  si,si              ; the tile
.tile:  mov  ax,si
        and  ax,3
        imul di,ax,6            ; its column
        mov  ax,si
        shr  ax,2
        imul ax,ax,3*24         ; its row
        add  di,ax
        add  di,cells
        mov  al,[board+si]
        mov  dx,0               ; DH: the row within the tile
.cell_row:
        xor  bx,bx              ; BX: the column within the tile
.cell:  mov  byte [di+bx],' '
        mov  byte [di+bx+24*12],70h
        cmp  bx,5
        je   .next_cell
        cmp  dh,2
        je   .next_cell
        or   al,al
        jz   .next_cell         ; the hole: grey
        mov  byte [di+bx+24*12],1Fh
        cmp  dh,1
        jne  .next_cell
        cmp  bx,2               ; the number, on the second row
        jne  .units
        cmp  al,10
        jb   .next_cell
        mov  byte [di+bx],'1'
        jmp  .next_cell
.units: cmp  bx,3
        jne  .next_cell
        push ax
        xor  ah,ah
        mov  cl,10
        div  cl
        add  ah,'0'
        mov  [di+bx],ah
        pop  ax
.next_cell:
        inc  bx
        cmp  bx,6
        jb   .cell
        add  di,24
        inc  dh
        cmp  dh,3
        jb   .cell_row
        inc  si
        cmp  si,16
        jb   .tile

        mov  bx,[puz_canvas]
        mov  dx,24
        mov  cx,12
        call blit_cells

        ; Moves: n, or Solved.
        mov  di,num_buf
        mov  si,s_moves
        call puzzle_solved
        jnc  .unsolved
        mov  si,s_solved
.unsolved:
        call copy_z
        movzx eax,word [puz_moves]
        call itoa
        owl_op   OP_SET_TEXT
        owl_w    [puz_moves_text]
        mov  si,num_buf
        call owl_zstr
        owl_send
        ret

; CF set if every tile is home.
puzzle_solved:
        push bx
        xor  bx,bx
.check: lea  ax,[bx+1]
        cmp  [board+bx],al
        jne  .no
        inc  bx
        cmp  bx,15
        jb   .check
        pop  bx
        stc
        ret
.no:    pop  bx
        clc
        ret

; ============================================================================
;  helpers
; ============================================================================

; BX: a window to bring to the front.
activate:
        owl_op   OP_ACTIVATE
        owl_w    bx
        owl_send
        ret

; The block in `cells` (DX wide, CX high; its attributes DX*CX further on)
; into canvas BX, as BLIT wants it: a character and an attribute a cell.
; With [blit_glyphs] set, a character FFh is "the glyph whose number is
; its place in the row", for the ASCII table.
blit_cells:
        owl_op   OP_BLIT
        owl_w    bx
        owl_w    0
        owl_w    0
        mov  ax,dx
        call owl_word
        mov  ax,cx
        call owl_word
        mov  ax,dx
        mul  cx
        mov  [blit_n],ax        ; the number of cells
        mov  cx,ax
        mov  si,cells
        xor  bx,bx              ; the next glyph, for the table
.cell:  movzx ax,byte [si]
        cmp  byte [blit_glyphs],0
        je   .plain
        cmp  al,0FFh
        jne  .plain
        push bx
        shl  bx,1
        mov  ax,[OWL_GLYPH_TABLE+bx]
        pop  bx
        inc  bx
.plain: call owl_word           ; the character
        mov  di,si
        add  di,[blit_n]
        mov  al,[di]
        call owl_byte           ; its colour
        inc  si
        loop .cell
        owl_send
        ret

; EAX as a signed decimal at DI; DI is left on the zero after it.
itoa:
        push eax
        push ebx
        push ecx
        push edx
        test eax,eax
        jns  .positive
        mov  byte [di],'-'
        inc  di
        neg  eax
.positive:
        mov  ebx,10
        xor  cx,cx
.divide:
        xor  edx,edx
        div  ebx
        push dx
        inc  cx
        test eax,eax
        jnz  .divide
.digit: pop  ax
        add  al,'0'
        mov  [di],al
        inc  di
        loop .digit
        mov  byte [di],0
        pop  edx
        pop  ecx
        pop  ebx
        pop  eax
        ret

; The zero-ended string at SI onto DI; DI is left on the zero.
copy_z:
        lodsb
        mov  [di],al
        or   al,al
        jz   .done
        inc  di
        jmp  copy_z
.done:  ret

; SI: a string; DI: CX columns of it right-aligned, zero-ended.
right_align:
        push si
        xor  bx,bx
.len:   cmp  byte [si+bx],0
        je   .measured
        inc  bx
        jmp  .len
.measured:
        mov  ax,cx
        sub  ax,bx
        jbe  .copy
        push cx
        mov  cx,ax
        mov  al,' '
        rep  stosb
        pop  cx
.copy:  pop  si
        jmp  copy_z

; AX, a character, onto DI as UTF-8 - one, two or three bytes.
utf8:
        cmp  ax,80h
        jae  .two
        stosb
        ret
.two:   cmp  ax,800h
        jae  .three
        push ax
        shr  ax,6
        or   al,0C0h
        stosb
        pop  ax
        and  al,3Fh
        or   al,80h
        stosb
        ret
.three: push ax
        shr  ax,12
        or   al,0E0h
        stosb
        pop  ax
        push ax
        shr  ax,6
        and  al,3Fh
        or   al,80h
        stosb
        pop  ax
        and  al,3Fh
        or   al,80h
        stosb
        ret

; ============================================================================
;  data
; ============================================================================

s_untitled  db 'UNTITLED',0
s_no_folder db 'Folder not found: ',0
s_not_found db 'Not found: ',0
s_cannot_read db 'Cannot read: ',0
s_first     db ' - the first ',0
s_bytes     db ' bytes',0
s_txt       db '.TXT',0
s_error     db 'Error',0
s_dec       db 'dec ',0
s_hex       db '  hex ',0
s_moves     db 'Moves: ',0
s_solved    db 'Solved! Moves: ',0
s_daynames  db 'Su Mo Tu We Th Fr Sa '
hex_digits  db '0123456789ABCDEF'
month_days  db 31,28,31,30,31,30,31,31,30,31,30,31
month_names dw m1,m2,m3,m4,m5,m6,m7,m8,m9,m10,m11,m12
m1  db 'January',0
m2  db 'February',0
m3  db 'March',0
m4  db 'April',0
m5  db 'May',0
m6  db 'June',0
m7  db 'July',0
m8  db 'August',0
m9  db 'September',0
m10 db 'October',0
m11 db 'November',0
m12 db 'December',0

box          dw 0
editors      dw 0
calc_win     dw 0
calc_display dw 0
calc_acc     dd 0
calc_entry   dd 0
calc_op      db 0
calc_typing  db 0
calc_error   db 0
cal_win      dw 0
cal_title    dw 0
cal_canvas   dw 0
cal_year     dw 0
cal_month    db 0
cal_first    db 0
cal_days     db 0
today_year   dw 0
today_month  db 0
today_day    db 0
ascii_win    dw 0
ascii_canvas dw 0
ascii_info   dw 0
ascii_code   dw 0
puz_win      dw 0
puz_canvas   dw 0
puz_moves_text dw 0
puz_moves    dw 0
board        db 16 dup 0
seed         dw 0
blit_glyphs  db 0
blit_n       dw 0

open_dialog  dw 0
open_panel   dw 0
file_n       dw 0
file_size    dd 0

title_buf    rb 48
num_buf      rb 80
cells        rb 2*35*17         ; characters, then their attributes
FILE_BUF_SIZE = 8192
open_dir     rb 80              ; the folder the panel shows
up_path      rb 80
path_buf     rb 128
temp_path    rb 128
name_buf     rb 128
msg_buf      rb 160
file_buf     rb FILE_BUF_SIZE   ; a file's first 8 KB, for the viewer

        assert $ <= OWL_REQUEST ; the binding's buffers start there
