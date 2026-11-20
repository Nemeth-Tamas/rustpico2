ENTRY(Reset_Handler)

MEMORY
{
    FLASH (rx)  : ORIGIN = 0x10000000, LENGTH = 4M
    RAM   (rwx) : ORIGIN = 0x20000000, LENGTH = 520K
}

__stack_top = ORIGIN(RAM) + LENGTH(RAM);

SECTIONS
{
    .vector_table ORIGIN(FLASH) :
    {
        . = ALIGN(512);
        KEEP(*(.vector_table))
    } > FLASH

    .image_def :
    {
        . = ALIGN(4);
        KEEP(*(.image_def))
    } > FLASH

    .text :
    {
        . = ALIGN(4);
        *(.text .text.*)
        *(.rodata .rodata.*)
    } > FLASH

    .ARM.exidx :
    {
        *(.ARM.exidx*)
        *(.ARM.extab*)
    } > FLASH

    .data :
    {
        . = ALIGN(4);
        __sdata = .;
        *(.data .data.*)
        . = ALIGN(4);
        __edata = .;
    } > RAM AT > FLASH

    __sidata = LOADADDR(.data);

    .bss (NOLOAD) :
    {
        . = ALIGN(4);
        __sbss = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(4);
        __ebss = .;
    } > RAM

    /DISCARD/ :
    {
        *(.comment)
        *(.eh_frame*)
    }
}

ASSERT(
    ADDR(.vector_table) == ORIGIN(FLASH),
    "Vector table must start at flash base"
);

ASSERT(
    SIZEOF(.image_def) == 20,
    "RP2350 image definition must be 20 bytes"
);

ASSERT(
    SIZEOF(.data) == 0,
    "Milestone 1 does not initialize .data"
);

ASSERT(
    SIZEOF(.bss) == 0,
    "Milestone 1 does not initialize .bss"
);