; ============================================================================
;  OWLOSUI.INC - the OWLOSUI toolkit, for a FASM .COM program.
;
;  The toolkit is not in here. It is OWLOSRES, the resident, which runs a
;  program with the toolkit behind INT 60h; this file is how an assembler
;  program talks to it. A request is built field by field, sent, and the
;  reply read back - the bytes of lib/PROTOCOL.md, the same ones the C#
;  client sends down a pipe:
;
;          owl_op   OP_STATIC           ; a new request
;          owl_w    [window]            ; parent: a u16
;          owl_rect 2,1,34,1            ; x y w h: four i16
;          owl_s    'Hello, world!'     ; a str: a u16 length, the bytes
;          owl_send                     ; AX = the reply's first word: the id
;
;  and a program's main loop is
;
;  .loop:  call owl_wait                ; AX = a button pressed, DX = a
;          ...                          ; menu or status command, or 0
;          jmp  .loop
;
;  Include it after a JMP over it, at the top of a .COM:
;
;          org  100h
;          jmp  start
;          include '...\lib\dos\asm\OWLOSUI.INC'
;
;  Memory: the request is built at OWL_REQUEST, the reply lands at
;  OWL_REPLY, and the routines keep their tables in OWL_WORK - all high in
;  the .COM's 64K, above the program (it must end below 6000h: put
;  `assert $ <= OWL_REQUEST` last) and below the stack, so none of it is
;  in the .COM file. Registers: the macros and routines keep every
;  register but AX (and DX for owl_wait, AL for owl_take_files).
; ============================================================================

OWL_INT     = 60h
OWL_REQUEST = 06000h            ; 32 KB: the request being built - a file's text fits
OWL_REQ_END = 0E000h
OWL_REPLY   = 0E000h            ; 3 KB: the reply
OWL_ROOM    = 0C00h
OWL_WORK    = 0EC00h            ; the routines' own tables, below:
OWL_GLYPH_TABLE = OWL_WORK      ; 512: each glyph as Unicode (owl_load_glyphs)
OWL_DTA     = OWL_WORK+200h     ; 64: DOS's FindFirst and FindNext write here
OWL_PATTERN = OWL_WORK+240h     ; 128: a folder with \*.* on the end
OWL_TEXT    = OWL_WORK+2C0h     ; 128: a name or path the panel reported
                                ; ... to EF40h; the stack is above

; ---- the requests: lib/PROTOCOL.md ------------------------------------------
OP_QUIT         = 00h
OP_INIT         = 01h
OP_WINDOW       = 10h
OP_TEXT         = 11h
OP_STATIC       = 12h
OP_INPUT        = 13h
OP_BUTTONS      = 14h
OP_MESSAGE_BOX  = 15h
OP_STATUS       = 16h
OP_LABEL        = 17h
OP_LIST         = 19h
OP_CANVAS       = 1Bh
OP_MENU_BAR     = 1Ch
OP_CLUSTER      = 1Dh
OP_BUTTON_ROW   = 1Eh
OP_CLOSE        = 20h
OP_GET_TEXT     = 21h
OP_ACTIVATE     = 27h
OP_ACTIVE       = 2Ah
OP_SET_TEXT     = 2Bh
OP_FILES        = 1Ah
OP_SET_FILES    = 25h
OP_TAKE_FILES   = 26h
OP_MARKED_NAMES = 28h
OP_SET_FILES_ERROR = 29h
OP_ADD_FILES    = 5Ah
OP_BLIT         = 2Ch
OP_MENU_CHECK   = 2Dh
OP_GET_CLICK    = 2Fh
OP_GET_GLYPHS   = 42h
OP_CYCLE        = 43h
OP_ZOOM         = 44h
OP_SET_BUTTON   = 45h
OP_FOCUS        = 46h
OP_CASCADE      = 47h
OP_TILE         = 48h
OP_WINDOW_STATUS = 4Ah
OP_WINDOW_LIST  = 4Ch
OP_CYCLE_BACK   = 4Dh
OP_WAIT         = 5Ch
OP_EDITOR       = 5Dh
OP_GET_EDITOR   = 5Eh
OP_SYNTAX       = 5Fh

; ---- what an editor offers (owl_editor): the core then runs each by itself --
OFFER_EDIT      = 01h           ; Undo, Redo, Cut, Copy, Paste, Select all
OFFER_FIND      = 02h           ; Find... Ctrl+F, Find next Ctrl+L
OFFER_REPLACE   = 04h           ; Replace... Ctrl+H
OFFER_WRAP      = 08h           ; Word wrap
OFFER_READONLY  = 10h           ; Read only
OFFER_HEX       = 20h           ; the same text as bytes
OFFER_KEYS      = 40h           ; Classic keys: the WordStar arrangement
OFFER_SYNTAX    = 80h           ; Syntax: the language the text is coloured as
OFFER_ALL       = 0FFh
; ---- and how it is set --------------------------------------------------------
EDIT_WRAP       = 01h
EDIT_READONLY   = 02h
EDIT_CLASSIC    = 04h
EDIT_HEX        = 08h
EDIT_SYNTAX     = 10h           ; read back only: set a language with owl_syntax

; ---- WINDOW flags -------------------------------------------------------------
WF_MODAL        = 01h
WF_FIXED        = 02h           ; not resizable
WF_NO_ZOOM      = 04h
WF_NO_CLOSE     = 08h
WF_BLUE         = 00h           ; a document
WF_CYAN         = 10h           ; help
WF_GREY         = 20h           ; a dialog
WF_DIALOG       = WF_GREY or WF_FIXED or WF_NO_ZOOM

; ---- a button's flags --------------------------------------------------------
BF_DEFAULT      = 01h           ; the one Enter presses
BF_ACCENT       = 02h           ; cyan
BF_DANGER       = 04h           ; red
BF_DISABLED     = 08h
BF_CANCEL       = 10h           ; the one Escape presses

; ---- a key, for the status line: kind, value, mods --------------------------
KEY_CHAR        = 0             ; value: the character
KEY_F           = 1             ; value: 1 for F1
KEY_NAMED       = 2             ; value: 0 Enter, 1 Esc, 2 Tab ...
KEY_NONE        = 0FFh
MOD_SHIFT       = 1
MOD_CTRL        = 2
MOD_ALT         = 4

; ---- the macros --------------------------------------------------------------

macro owl_op op {               ; a new request
        mov  al,op
        call owl_start
}
macro owl_b v {                 ; a u8
        mov  al,v
        call owl_byte
}
macro owl_w v {                 ; a u16 or an i16
        mov  ax,v
        call owl_word
}
macro owl_s text {              ; a str, from the text in the macro
        call owl_istr
        db   text,0
}
macro owl_rect x,y,w,h {        ; a rect
        owl_w x
        owl_w y
        owl_w w
        owl_w h
}
macro owl_key kind,value,mods { ; a key, for a status item
        owl_b kind
        owl_w value
        owl_b mods
}
macro owl_send {                ; send it: AX = the reply's first word, CF on error
        call owl_call
}
macro owl_empty {               ; an empty str
        owl_w 0
}

; The parts that repeat inside a request: a button of BUTTONS, BUTTON_ROW
; or MESSAGE_BOX; an item of STATUS or WINDOW_STATUS (an empty label binds
; the key without showing it); a menu of MENU_BAR and its items.

macro owl_button cmd,flags,label {
        owl_w cmd
        owl_b flags
        owl_s label
}
macro owl_status_item label,cmd,kind,value,mods {
        owl_w cmd
        owl_key kind,value,mods
  if label eq
        owl_empty
  else
        owl_s label
  end if
}
macro owl_menu text,count {     ; a menu on the bar; its items follow
        owl_b 0
        owl_w 0
        owl_s text
        owl_empty
        owl_empty
        owl_b count
}
macro owl_item text,shortcut,hint,cmd {
        owl_b 0
        owl_w cmd
        owl_s text
  if shortcut eq
        owl_empty
  else
        owl_s shortcut
  end if
        owl_s hint
        owl_b 0                 ; no items of its own
}
macro owl_line {                ; a line between items
        owl_b 1
        owl_w 0
        owl_empty
        owl_empty
        owl_empty
        owl_b 0
}

; ---- the routines ------------------------------------------------------------

owl_at     dw 0                 ; where the next field goes
owl_status db 0                 ; the last reply's status

; Is OWLOSRES behind INT 60h? Its handler has OWLOSUI and a zero in the
; eight bytes before it. If not, say how to start this and leave.
owl_check:
        push es
        push si
        push di
        push cx
        mov  ax,3500h+OWL_INT
        int  21h                ; ES:BX: the vector
        cmp  bx,8
        jb   .absent
        lea  di,[bx-8]
        mov  si,owl_signature
        mov  cx,8
        cld
        repe cmpsb
        jne  .absent
        pop  cx
        pop  di
        pop  si
        pop  es
        ret
.absent:
        mov  dx,owl_absent
        mov  ah,9
        int  21h
        mov  ax,4C01h
        int  21h
owl_signature db 'OWLOSUI',0
owl_absent db 'This program draws with OWLOSUI: start it with OWLOSRES, as RUN.BAT does.',13,10,'$'

; AL: the op of a new request.
owl_start:
        mov  [OWL_REQUEST],al
        mov  word [owl_at],OWL_REQUEST+3
        ret

; AL: one byte onto the request.
owl_byte:
        push di
        mov  di,[owl_at]
        mov  [di],al
        inc  word [owl_at]
        pop  di
        ret

; AX: one word onto the request.
owl_word:
        push di
        mov  di,[owl_at]
        mov  [di],ax
        add  word [owl_at],2
        pop  di
        ret

; DS:SI: a zero-ended string onto the request, as a str.
owl_zstr:
        push si
        call owl_append
        pop  si
        ret

; The zero-ended string right after the CALL, as a str; returns past it.
owl_istr:
        push bp
        mov  bp,sp
        push si
        mov  si,[bp+2]
        call owl_append
        mov  [bp+2],si
        pop  si
        pop  bp
        ret

; The string at SI onto the request; SI is left past its zero.
owl_append:
        push di
        push cx
        mov  di,[owl_at]
        add  di,2
        xor  cx,cx
.char:  lodsb
        or   al,al
        jz   .end
        mov  [di],al
        inc  di
        inc  cx
        jmp  .char
.end:   push di
        mov  di,[owl_at]
        mov  [di],cx
        pop  di
        mov  [owl_at],di
        pop  cx
        pop  di
        ret

; Send the request. AX = the reply body's first word (an id, usually),
; CF set if the status was not OK - the body is then the error as a str.
owl_call:
        push cx
        push si
        push di
        push es
        mov  ax,[owl_at]
        sub  ax,OWL_REQUEST+3
        mov  [OWL_REQUEST+1],ax
        push ds
        pop  es
        mov  si,OWL_REQUEST
        mov  di,OWL_REPLY
        mov  cx,OWL_ROOM
        int  OWL_INT
        mov  [owl_status],al
        mov  ax,[OWL_REPLY+3]
        pop  es
        pop  di
        pop  si
        pop  cx
        cmp  byte [owl_status],0
        je   .ok
        stc
        ret
.ok:    clc
        ret

; Wait for the person: one key or click. AX = the button it pressed, DX =
; the menu or status command it chose; either may be 0.
owl_wait:
        owl_op OP_WAIT
        call owl_call
        mov  dx,[OWL_REPLY+5]
        ret

; The glyph table into OWL_GLYPH_TABLE: what each of the 256 glyphs looks
; like, as Unicode. A canvas cell and a text from DOS are sent through it.
owl_load_glyphs:
        push cx
        push si
        push di
        owl_op OP_GET_GLYPHS
        call owl_call
        mov  si,OWL_REPLY+6
        mov  di,OWL_GLYPH_TABLE
        mov  cx,256
        cld
        rep  movsw
        mov  byte [owl_glyphs_loaded],1
        pop  di
        pop  si
        pop  cx
        ret

; ---- files: DOS reads the folder (FindFirst, FindNext), the panel shows it --

owl_glyphs_loaded db 0
owl_more    db 0                ; FindFirst or FindNext left an entry in the DTA
owl_new     db 0                ; the panel is being made: its id is the first reply
owl_panel   dw 0
owl_count   dw 0                ; entries in the part being built
owl_count_at dw 0               ; where that count goes
owl_len_at  dw 0

; SI: a folder, zero-ended. OWL_PATTERN: it with \*.* on the end.
owl_make_pattern:
        push si
        push di
        mov  di,OWL_PATTERN
.copy:  lodsb
        mov  [di],al
        inc  di
        or   al,al
        jnz  .copy
        dec  di                 ; on the zero
        cmp  di,OWL_PATTERN
        je   .slash
        cmp  byte [di-1],'\'
        je   .mask
.slash: mov  byte [di],'\'
        inc  di
.mask:  mov  dword [di],'*.*'   ; and the zero after it
        pop  di
        pop  si
        ret

; FindFirst on OWL_PATTERN - everything but a volume label - into OWL_DTA.
; [owl_more] says whether it found anything; CF and AX are DOS's.
owl_find_first:
        push cx
        push dx
        mov  dx,OWL_DTA
        mov  ah,1Ah
        int  21h
        mov  dx,OWL_PATTERN
        mov  cx,37h
        mov  ah,4Eh
        int  21h
        mov  byte [owl_more],0
        jc   .none
        mov  byte [owl_more],1
.none:  pop  dx
        pop  cx
        ret

; The folder's entries after a request's header, as FILES, SET_FILES and
; ADD_FILES want them: a count, then a name, size, date, time and DOS
; attributes each, straight from the DTA. "." is left out; ".." is kept,
; as DOS gives it. A folder too big for one request goes on in ADD_FILES
; to the same panel. AX: the panel.
owl_listing:
        push si
        mov  ax,[owl_at]
        mov  [owl_count_at],ax
        mov  word [owl_count],0
        owl_w 0
        cmp  byte [owl_more],0
        je   .done
.entry: cmp  word [OWL_DTA+30],'.' ; ".", then its zero
        je   .next
        mov  ax,[owl_at]
        add  ax,64
        cmp  ax,OWL_REQ_END
        jb   .room
        call owl_part_done
.room:  mov  si,OWL_DTA+30      ; the name
        call owl_zstr
        mov  ax,[OWL_DTA+26]    ; the size, low word then high
        call owl_word
        mov  ax,[OWL_DTA+28]
        call owl_word
        mov  ax,[OWL_DTA+24]    ; the date: year from 1980, month, day
        shr  ax,9
        add  ax,1980
        call owl_word
        mov  ax,[OWL_DTA+24]
        shr  ax,5
        and  al,15
        call owl_byte
        mov  al,[OWL_DTA+24]
        and  al,31
        call owl_byte
        mov  ax,[OWL_DTA+22]    ; the time: hour, minute
        shr  ax,11
        call owl_byte
        mov  ax,[OWL_DTA+22]
        shr  ax,5
        and  al,63
        call owl_byte
        mov  al,[OWL_DTA+21]    ; the attributes
        call owl_byte
        inc  word [owl_count]
.next:  mov  ah,4Fh             ; FindNext
        int  21h
        jnc  .entry
.done:  call owl_patch_count
        call owl_call
        cmp  byte [owl_new],0
        je   .old
        mov  ax,[OWL_REPLY+3]
        mov  [owl_panel],ax
.old:   mov  ax,[owl_panel]
        pop  si
        ret

owl_patch_count:
        push di
        mov  di,[owl_count_at]
        mov  ax,[owl_count]
        mov  [di],ax
        pop  di
        ret

; A part is full: send it, and go on in ADD_FILES.
owl_part_done:
        call owl_patch_count
        call owl_call
        cmp  byte [owl_new],0
        je   .old
        mov  ax,[OWL_REPLY+3]
        mov  [owl_panel],ax
        mov  byte [owl_new],0
.old:   owl_op OP_ADD_FILES
        owl_w [owl_panel]
        mov  ax,[owl_at]
        mov  [owl_count_at],ax
        mov  word [owl_count],0
        owl_w 0
        ret

; BX: the window, DX: rows left free above the panel, SI: the folder.
; AX: a file panel filling the window, the folder in it. A folder DOS
; cannot read comes out an empty panel.
owl_files:
        call owl_make_pattern
        call owl_find_first
        push si
        owl_op OP_FILES
        owl_w bx
        owl_w 0
        owl_w dx
        owl_w 0
        owl_w 0
        owl_b 0
        owl_s '*.*'
        mov  si,OWL_PATTERN
        call owl_zstr
        pop  si
        mov  byte [owl_new],1
        jmp  owl_listing

; BX: a panel, SI: the folder to show in it. CF set if DOS cannot read it.
owl_set_files:
        call owl_make_pattern
        call owl_find_first
        jnc  .read
        cmp  ax,18              ; no more files: an empty folder is still one
        jne  .fail
.read:  push si
        owl_op OP_SET_FILES
        owl_w bx
        mov  si,OWL_PATTERN
        call owl_zstr
        owl_s '*.*'
        pop  si
        mov  [owl_panel],bx
        mov  byte [owl_new],0
        call owl_listing
        clc
        ret
.fail:  stc
        ret

; BX: a panel, SI: a line saying what went wrong, shown in it.
owl_files_error:
        owl_op OP_SET_FILES_ERROR
        owl_w bx
        call owl_zstr
        owl_send
        ret

; BX: a panel. AL: 0 nothing, 1 a name entered, 2 a path typed - which is
; at OWL_TEXT, zero-ended.
owl_take_files:
        owl_op OP_TAKE_FILES
        owl_w bx
        call owl_call
        push si
        mov  si,OWL_REPLY+4
        call owl_reply_text
        pop  si
        mov  al,[OWL_REPLY+3]
        ret

; BX: a panel. The name under the cursor at OWL_TEXT; CF set if none.
owl_current_name:
        owl_op OP_MARKED_NAMES
        owl_w bx
        call owl_call
        cmp  word [OWL_REPLY+3],0
        je   .none
        push si
        mov  si,OWL_REPLY+5
        call owl_reply_text
        pop  si
        clc
        ret
.none:  stc
        ret

; A str of the reply, at SI, to OWL_TEXT, zero-ended; at most 127 bytes.
owl_reply_text:
        push cx
        push di
        mov  cx,[si]
        add  si,2
        cmp  cx,127
        jbe  .fits
        mov  cx,127
.fits:  mov  di,OWL_TEXT
        cld
        rep  movsb
        mov  byte [di],0
        pop  di
        pop  cx
        ret

; BX: a text. AL: what it offers (OFFER_ bits), AH: how it is set (EDIT_
; bits). From then on the core answers its Edit menu, its keys and its
; Find and Replace dialogs by itself; nothing comes back to the program.
owl_editor:
        push ax
        owl_op OP_EDITOR
        owl_w bx
        pop  ax
        call owl_byte           ; the offers
        mov  al,ah
        call owl_byte           ; the state
        owl_send
        ret

; BX: a text. AL: its EDIT_ bits as the person has left them.
owl_editor_state:
        owl_op OP_GET_EDITOR
        owl_w bx
        call owl_call
        mov  al,[OWL_REPLY+4]
        ret

; BX: a text. SI: a zero-ended language - its name, an extension or a
; file name: 'ASM', 'DEMO.PAS'. AL: 1 if there is such a language and the
; text is now coloured as it, 0 if not and the text is plain.
owl_syntax:
        owl_op OP_SYNTAX
        owl_w bx
        call owl_zstr
        call owl_call
        mov  al,[OWL_REPLY+3]
        ret

; An editor holding bytes in DOS's code page. BX: the window, SI: the
; bytes, CX: how many, AL: its flags (1 read-only). Each byte goes through
; the glyph table into Unicode and on as UTF-8, which is what the wire
; carries. A line ends at LF; CR is dropped; a tab is a space. What does
; not fit in one request is left out. AX: the editor.
owl_text_dos:
        push bx
        push cx
        push si
        push di
        push ax
        cmp  byte [owl_glyphs_loaded],0
        jne  .loaded
        call owl_load_glyphs
.loaded:
        owl_op OP_TEXT
        owl_w bx
        owl_w 0
        owl_w 0
        owl_w 0
        owl_w 0
        owl_b 0                 ; fill the window
        pop  ax
        call owl_byte           ; the flags
        mov  di,[owl_at]
        mov  [owl_len_at],di
        add  di,2
        cld
.byte:  jcxz .done
        cmp  di,OWL_REQ_END-4
        jae  .done
        lodsb
        dec  cx
        cmp  al,13
        je   .byte
        cmp  al,10
        je   .one
        cmp  al,9
        jne  .glyph
        mov  al,' '
.one:   stosb
        jmp  .byte
.glyph: xor  bh,bh
        mov  bl,al
        shl  bx,1
        mov  ax,[OWL_GLYPH_TABLE+bx]
        cmp  ax,80h
        jb   .one
        cmp  ax,800h
        jae  .three
        push ax                 ; two bytes: 110xxxxx 10xxxxxx
        shr  ax,6
        or   al,0C0h
        stosb
        pop  ax
        and  al,3Fh
        or   al,80h
        jmp  .one
.three: push ax                 ; three: 1110xxxx 10xxxxxx 10xxxxxx
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
        jmp  .one
.done:  mov  [owl_at],di
        mov  ax,di
        mov  bx,[owl_len_at]
        sub  ax,bx
        sub  ax,2
        mov  [bx],ax
        call owl_call
        pop  di
        pop  si
        pop  cx
        pop  bx
        ret
