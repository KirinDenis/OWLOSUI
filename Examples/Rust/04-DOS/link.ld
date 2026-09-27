/* A flat binary at address 0: the loader gives it a code and a data
   selector whose base is wherever the block landed, so every address in
   here is an offset from that base. `_start` is first, so the loader can
   jump to offset 0 and know what it will find. */
ENTRY(_start)
SECTIONS
{
  . = 0;
  .text : { KEEP(*(.text.start)) *(.text .text.*) }
  .rodata : { *(.rodata .rodata.*) }
  .data : { *(.data .data.*) }
  .bss : { *(.bss .bss.*) *(COMMON) }
  /DISCARD/ : { *(.eh_frame*) *(.note*) *(.comment) *(.debug*) }
}
