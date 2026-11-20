use std::env;
use std::fs;

const FLASH_BASE: u32 = 0x1000_0000;
const FLASH_SIZE: usize = 4 * 1024 * 1024;

const UF2_BLOCK_SIZE: usize = 512;
const UF2_PAYLOAD_SIZE: usize = 256;

const UF2_MAGIC_START0: u32 = 0x0A32_4655;
const UF2_MAGIC_START1: u32 = 0x9E5D_5157;
const UF2_MAGIC_END: u32 = 0x0AB1_6F30;

const UF2_FLAG_FAMILY_ID: u32 = 0x0000_2000;
const RP2350_ARM_S_FAMILY: u32 = 0xE48B_FF59;

fn read_u16(data: &[u8], offset: usize) -> u16 {
    let bytes: [u8; 2] = data
        .get(offset..offset + 2)
        .expect("Truncated ELF")
        .try_into()
        .unwrap();

    u16::from_le_bytes(bytes)
}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    let bytes: [u8; 4] = data
        .get(offset..offset + 4)
        .expect("Truncated ELF")
        .try_into()
        .unwrap();

    u32::from_le_bytes(bytes)
}

fn write_u32(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn main() {
    let args: Vec<String> = env::args().collect();

    assert_eq!(args.len(), 3, "Usage: elf2uf2 <input.elf> <output.uf2>");

    let elf = fs::read(&args[1]).expect("Cannot read input ELF");

    assert!(elf.len() >= 52, "ELF header is truncated");
    assert!(elf.starts_with(b"\x7FELF"), "Not an ELF file");

    // ELF32, little-endian, ARM.
    assert_eq!(elf[4], 1, "Expected ELF32");
    assert_eq!(elf[5], 1, "Expected little-endian ELF");
    assert_eq!(read_u16(&elf, 18), 40, "Expected ARM ELF");

    let program_header_offset = read_u32(&elf, 28) as usize;
    let program_header_size = read_u16(&elf, 42) as usize;
    let program_header_count = read_u16(&elf, 44) as usize;

    assert!(program_header_size >= 32, "Invalid ELF program header size");

    let mut flash = vec![0xFFu8; FLASH_SIZE];
    let mut used = 0usize;

    // Copy loadable flash segments into a flash image.
    for index in 0..program_header_count {
        let header_offset = program_header_offset
            .checked_add(
                index
                    .checked_mul(program_header_size)
                    .expect("ELF header overflow"),
            )
            .expect("ELF header overflow");

        let segment_type = read_u32(&elf, header_offset);

        // PT_LOAD
        if segment_type != 1 {
            continue;
        }

        let file_offset = read_u32(&elf, header_offset + 4) as usize;

        let virtual_address = read_u32(&elf, header_offset + 8);

        let physical_address = read_u32(&elf, header_offset + 12);

        let file_size = read_u32(&elf, header_offset + 16) as usize;

        if file_size == 0 {
            continue;
        }

        // Prefer physical/load address when available.
        let address = if physical_address != 0 {
            physical_address
        } else {
            virtual_address
        };

        if address < FLASH_BASE {
            continue;
        }

        let start = (address - FLASH_BASE) as usize;

        if start >= FLASH_SIZE {
            continue;
        }

        let end = start.checked_add(file_size).expect("Segment size overflow");

        assert!(end <= FLASH_SIZE, "Flash segment exceeds 4 MB");

        let source_end = file_offset
            .checked_add(file_size)
            .expect("ELF file offset overflow");

        let source = elf
            .get(file_offset..source_end)
            .expect("ELF segment outside input file");

        flash[start..end].copy_from_slice(source);
        used = used.max(end);
    }

    assert!(used >= 20, "No usable flash image found");

    // Validate the initial ARM vector words.
    let stack_pointer = read_u32(&flash, 0);
    let reset_vector = read_u32(&flash, 4);

    assert!(
        (0x2000_0000..=0x2008_2000).contains(&stack_pointer) && stack_pointer % 8 == 0,
        "Invalid initial stack pointer"
    );

    assert!(
        (reset_vector & 1) == 1
            && (FLASH_BASE..FLASH_BASE + FLASH_SIZE as u32).contains(&(reset_vector & !1)),
        "Invalid Thumb reset vector"
    );

    // Verify the mandatory RP2350 image definition.
    let image_definition: [u32; 5] = [
        0xFFFF_DED3,
        0x1021_0142,
        0x0000_01FF,
        0x0000_0000,
        0xAB12_3579,
    ];

    let signature: Vec<u8> = image_definition
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .collect();

    let search_end = used.min(4096);

    assert!(
        flash[..search_end]
            .windows(signature.len())
            .any(|window| window == signature.as_slice()),
        "RP2350 IMAGE_DEF not found in first 4 KiB"
    );

    let block_count = used.div_ceil(UF2_PAYLOAD_SIZE);

    let mut uf2 = Vec::with_capacity(block_count * UF2_BLOCK_SIZE);

    for block_number in 0..block_count {
        let mut block = [0u8; UF2_BLOCK_SIZE];

        let offset = block_number * UF2_PAYLOAD_SIZE;

        write_u32(&mut block, 0, UF2_MAGIC_START0);
        write_u32(&mut block, 4, UF2_MAGIC_START1);
        write_u32(&mut block, 8, UF2_FLAG_FAMILY_ID);

        write_u32(&mut block, 12, FLASH_BASE + offset as u32);

        write_u32(&mut block, 16, UF2_PAYLOAD_SIZE as u32);

        write_u32(&mut block, 20, block_number as u32);

        write_u32(&mut block, 24, block_count as u32);

        write_u32(&mut block, 28, RP2350_ARM_S_FAMILY);

        block[32..32 + UF2_PAYLOAD_SIZE].copy_from_slice(&flash[offset..offset + UF2_PAYLOAD_SIZE]);

        write_u32(&mut block, UF2_BLOCK_SIZE - 4, UF2_MAGIC_END);

        uf2.extend_from_slice(&block);
    }

    fs::write(&args[2], &uf2).expect("Failed to write UF2");

    println!("UF2 generated successfully!");
    println!("Input ELF: {}", args[1]);
    println!("Output UF2: {}", args[2]);
    println!("Flash image: {} bytes", used);
    println!("UF2 blocks: {}", block_count);
    println!("UF2 size: {} bytes", uf2.len());
    println!("Family: RP2350 ARM Secure");
}
