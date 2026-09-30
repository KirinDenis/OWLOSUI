; ============================================================================
;  LOADER.INC - the DOS loader for an OWLOSUI program in extended memory.
;
;  A .COM includes this after saying what it is:
;
;      LOG = 1                          ; LOADER.LOG, a mark per stage
;      include 'LOADER.INC'
;
;  and loads the flat 32-bit binary of its own name beside it - OWLOS.COM
;  loads OWLOS.BIN, OWLOSRES.COM loads OWLOSRES.BIN - from the folder the
;  .COM is in, whatever the current one is.
;
;  What runs is that binary: Rust, linked at address 0. DOS cannot load
;  it, so this does: it asks DPMI for four megabytes of extended memory,
;  reads the file into them, zeroes the rest (the binary's .bss is not in
;  the file), builds a 32-bit code selector and a 32-bit data selector
;  with the block as their base, and jumps to offset 0 with three numbers
;  on the stack: where the block is (linear), the flags (/SHOT bit 0,
;  /QUIT bit 1, /STRESS bit 2), and this program's PSP segment, whose
;  command tail the binary may read.
;
;  Everything the program then touches below a megabyte - the video card
;  at B8000, the BIOS data area at 400 - it reaches through that data
;  selector as the linear address minus the base, wrapping: the limit is
;  4 GB and DPMI hosts leave the first megabyte where it is.
;
;  Needs a DPMI host. CWSDPMI.EXE, beside this, is one: run it first with
;  no arguments and it stays for exactly one program. Plain DOSBox has no
;  32-bit DPMI even then; DOSBox-X has.
;
;  The program leaves through INT 21h AH=4Ch on its own; nothing here
;  runs after the jump.
; ============================================================================

        org 100h
        use16

CHUNK   = 8000h                 ; how much of the file is read at a time
BLOCK   = 00400000h             ; four megabytes for the program

start:
        ; ---- our own memory: a COM owns everything; give back all but 64K
        mov  ah,4Ah
        mov  bx,1000h
        int  21h

        ; ---- the flags: /SHOT on the command line
        mov  word [flags],0
        movzx cx,byte [80h]     ; the length of the tail
        mov  si,81h
.arg:   jcxz .args_done
        lodsb
        dec  cx
        cmp  al,'/'
        jne  .arg
        cmp  cx,4
        jb   .arg
        cmp  dword [si],'SHOT'
        je   .shot
        cmp  dword [si],'shot'
        je   .shot
        cmp  dword [si],'QUIT'
        je   .quit
        cmp  dword [si],'quit'
        je   .quit
        cmp  dword [si],'STRE'
        je   .stress
        cmp  dword [si],'stre'
        jne  .arg
.quit:  or   word [flags],2          ; in and straight out: the smoke test
        jmp  .arg
.stress: or  word [flags],4          ; a while of dragging, done by itself
        jmp  .arg
.shot:  or   word [flags],1
.args_done:

        ; ---- a trace for whoever cannot see the screen: LOADER.LOG gets a
        ;      character per stage. When it stops, that is where we died.
if LOG
        mov  ah,3Ch
        xor  cx,cx
        mov  dx,logname
        int  21h
        jc   .no_log
        mov  [logh],ax
.no_log:
end if
        mov  al,'1'
        call mark_rm

        ; ---- the binary's name: this program's own path, from the end of
        ;      its environment, with .COM made .BIN. The environment is
        ;      strings, an empty one, a word, then the path.
        mov  es,[2Ch]
        xor  di,di
        xor  al,al
.env:   cmp  byte [es:di],0
        je   .env_end
        mov  cx,0FFFFh
        repne scasb
        jmp  .env
.env_end:
        lea  si,[di+3]
        mov  di,fname
.name:  mov  al,[es:si]
        mov  [di],al
        inc  si
        inc  di
        or   al,al
        jnz  .name
        mov  word [di-4],'BI'
        mov  byte [di-2],'N'
        push cs
        pop  es

        ; ---- the file, opened in real mode; the handle survives the switch
        mov  ax,3D00h
        mov  dx,fname
        int  21h
        jc   no_file
        mov  [handle],ax
        mov  al,'2'
        call mark_rm

        ; ---- is there a 32-bit DPMI host?
        mov  ax,1687h
        int  2Fh
        or   ax,ax
        jnz  no_dpmi
        test bl,1
        jz   no_dpmi
        mov  word [dpmi_entry],di
        mov  word [dpmi_entry+2],es
        mov  [realseg],cs
        or   si,si
        jz   .no_host_mem
        mov  bx,si              ; the host wants that many paragraphs
        mov  ah,48h
        int  21h
        jc   no_mem
        mov  es,ax
.no_host_mem:

        ; ---- into protected mode as a 32-bit client. From here CS, DS and
        ;      SS are selectors mapping the segments they were.
        mov  al,'3'
        call mark_rm
        mov  ax,1
        call far [dpmi_entry]
        jc   no_dpmi
        mov  al,'4'
        call mark_pm

        ; ---- four megabytes
        mov  ax,0501h
        mov  bx,BLOCK shr 16
        mov  cx,BLOCK and 0FFFFh
        int  31h
        jc   no_mem
        mov  word [base],cx
        mov  word [base+2],bx
        mov  al,'5'
        call mark_pm
        mov  eax,[base]         ; the block's linear address, in hex
        call log_hex

        ; ---- two descriptors: code and data, both 32-bit, base = block,
        ;      limit 4 GB. Rights first, then the limit: setting a limit
        ;      above a megabyte makes the host set the granularity bit.
        mov  ax,0000h
        mov  cx,2
        int  31h
        jc   no_mem
        mov  [code_sel],ax
        add  ax,8
        mov  [data_sel],ax

        mov  ax,0009h
        mov  bx,[code_sel]
        mov  cx,40FAh           ; present, DPL 3, code, readable; 32-bit
        int  31h
        mov  ax,0009h
        mov  bx,[data_sel]
        mov  cx,40F2h           ; present, DPL 3, data, writable; 32-bit
        int  31h
        mov  bx,[code_sel]
        call set_base_limit
        mov  bx,[data_sel]
        call set_base_limit
        mov  al,'6'
        call mark_pm
        movzx eax,[code_sel]    ; and the two selectors
        call log_hex
        movzx eax,[data_sel]
        call log_hex
        mov  ax,0006h           ; and what the host says their bases are
        mov  bx,[code_sel]
        int  31h
        push dx
        push cx
        pop  eax
        call log_hex
        mov  ax,0006h
        mov  bx,[data_sel]
        int  31h
        push dx
        push cx
        pop  eax
        call log_hex
        mov  bx,[code_sel]
        lsl  eax,bx
        call log_hex

        ; ---- read the file into the block, a chunk at a time, through a
        ;      real-mode INT 21h: DOS reads into our own segment, and we
        ;      copy from there into the block through the data selector.
        mov  es,[data_sel]
        xor  edi,edi            ; where the next chunk goes, in the block
.read:  mov  dword [rm_eax],3F00h
        movzx eax,word [handle]
        mov  [rm_ebx],eax
        mov  dword [rm_ecx],CHUNK
        mov  dword [rm_edx],buf
        call rm_int21
        jc   read_failed
        test word [rm_flags],1
        jnz  read_failed
        mov  ecx,[rm_eax]
        and  ecx,0FFFFh
        jecxz .read_done
        mov  si,buf
.copy:  mov  al,[si]
        mov  [es:edi],al
        inc  si
        inc  edi
        dec  ecx
        jnz  .copy
        cmp  word [rm_eax],CHUNK
        je   .read
.read_done:
        mov  dword [rm_eax],3E00h     ; close the file
        movzx eax,word [handle]
        mov  [rm_ebx],eax
        call rm_int21
        mov  al,'7'
        call mark_pm

        ; ---- zero the rest of the block: the program's .bss and its heap
        ;      live there, and Rust counts on statics starting at nought.
        mov  ecx,BLOCK
        sub  ecx,edi
        shr  ecx,2
        xor  eax,eax
        rep  stos dword [es:edi]

        ; ---- the log is closed before the jump: nothing here runs after it
        mov  al,'8'
        call mark_pm
        mov  eax,[base]
        call log_hex
        mov  eax,[es:0]         ; the first bytes of the program, as loaded
        call log_hex
        cmp  word [logh],0
        je   .no_close
        mov  dword [rm_eax],3E00h
        movzx eax,word [logh]
        mov  [rm_ebx],eax
        call rm_int21
.no_close:

        ; ---- the program's stack at the top of the block, and its
        ;      arguments the way a C call leaves them: the PSP, the flags,
        ;      the base, and a return address nobody will use.
        mov  eax,[base]
        mov  ebp,eax            ; kept in a register past the last DOS call
        movzx edx,word [flags]
        movzx ebx,word [realseg]
        mov  ss,[data_sel]
        mov  esp,BLOCK-16
        push ebx                ; the PSP: a .COM's segment is its PSP
        push edx                ; flags
        push ebp                ; base
        push dword 0            ; the "return address"
        mov  ax,[code_sel]
        mov  word [pm_entry+4],ax
        mov  ax,[data_sel]
        mov  ds,ax
        mov  es,ax
        jmp  fword [cs:pm_entry]

; ---- INT 21h in real mode, through DPMI, with the RMCS filled in --------
;  EAX, EBX, ECX, EDX of the structure are the caller's; DS is our real
;  segment and the stack is the host's. CF is the call's own on return.
rm_int21:
        push edi                ; the copy loop keeps its place in EDI
        mov  ax,[realseg]
        mov  [rm_ds],ax
        mov  word [rm_ss],0
        mov  word [rm_sp],0
        push es
        push ds
        pop  es
        mov  ax,0300h
        mov  bx,0021h
        xor  cx,cx
        mov  edi,rmcs           ; ES:EDI - a 32-bit client, so all of EDI
        int  31h
        pop  es
        pop  edi
        jc   .fail
        test word [rm_flags],1
        jnz  .fail
        clc
        ret
.fail:  stc
        ret

; ---- EAX as eight hex digits and a space, into the log -------------------
log_hex:
        mov  cx,8
.hex:   rol  eax,4
        push eax
        and  al,0Fh
        add  al,'0'
        cmp  al,'9'
        jbe  .dig
        add  al,7
.dig:   push cx
        call mark_pm
        pop  cx
        pop  eax
        loop .hex
        mov  al,' '
        call mark_pm
        ret

; ---- one character into the log ------------------------------------------
mark_rm:
        cmp  word [logh],0
        je   .none
        mov  [mark_ch],al
        mov  ah,40h
        mov  bx,[logh]
        mov  cx,1
        mov  dx,mark_ch
        int  21h
.none:  ret
mark_pm:
        cmp  word [logh],0
        je   .none
        mov  [mark_ch],al
        mov  dword [rm_eax],4000h
        movzx eax,word [logh]
        mov  [rm_ebx],eax
        mov  dword [rm_ecx],1
        mov  dword [rm_edx],mark_ch
        call rm_int21
.none:  ret

; ---- a selector's base to the block and its limit to 4 GB ----------------
set_base_limit:
        push bx
        mov  ax,0007h
        mov  cx,word [base+2]
        mov  dx,word [base]
        int  31h
        pop  bx
        mov  ax,0008h
        mov  cx,0FFFFh
        mov  dx,0FFFFh
        int  31h
        ret

; ---- the ways out, all in real mode still ---------------------------------
no_file:
        mov  dx,m_nofile
        jmp  say_and_exit
no_dpmi:
        mov  dx,m_nodpmi
        jmp  say_and_exit
no_mem:
        mov  dx,m_nomem
        jmp  say_and_exit
read_failed:
        ; In protected mode by now; INT 21h still prints through the host.
        mov  dx,m_read
say_and_exit:
        mov  ah,9
        int  21h
        mov  ax,4C01h
        int  21h

; ---- data ----------------------------------------------------------------
fname    rb 128
logname  db 'LOADER.LOG',0
logh     dw 0
mark_ch  db 0
m_nofile db 'The .BIN is not beside the .COM.',13,10,'$'
m_nodpmi db 'No 32-bit DPMI host. Run CWSDPMI first.',13,10,'$'
m_nomem  db 'Not enough extended memory (4 MB).',13,10,'$'
m_read   db 'Could not read the .BIN.',13,10,'$'

flags    dw 0
handle   dw 0
dpmi_entry dd 0
realseg  dw 0
base     dd 0
code_sel dw 0
data_sel dw 0
pm_entry dd 0                   ; offset 0 of the block...
         dw 0                   ; ...through the code selector, set below

; The real-mode call structure DPMI function 0300h reads and fills.
rmcs:
rm_edi   dd 0
rm_esi   dd 0
rm_ebp   dd 0
rm_res   dd 0
rm_ebx   dd 0
rm_edx   dd 0
rm_ecx   dd 0
rm_eax   dd 0
rm_flags dw 0
rm_es    dw 0
rm_ds    dw 0
rm_fs    dw 0
rm_gs    dw 0
rm_ip    dw 0
rm_cs    dw 0
rm_sp    dw 0
rm_ss    dw 0

buf:                            ; CHUNK bytes from here to the end of the segment
