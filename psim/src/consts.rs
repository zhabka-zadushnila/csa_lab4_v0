pub const IN_PORT: u32 = 0x0000;
pub const OUT_PORT: u32 = 0x0004;

pub const BUS_OUT_AC: u8 = 0b100000;
pub const BUS_OUT_DR: u8 = 0b010000;
pub const BUS_OUT_IP: u8 = 0b001000;
pub const BUS_OUT_SP: u8 = 0b000100;
pub const BUS_OUT_CR: u8 = 0b000010;
pub const BUS_OUT_ALU: u8 = 0b000001;

pub const BUS_IN_AC: u8 = 0b1000000;
pub const BUS_IN_DR: u8 = 0b0100000;
pub const BUS_IN_AR: u8 = 0b0010000;
pub const BUS_IN_IP: u8 = 0b0001000;
pub const BUS_IN_SP: u8 = 0b0000100;
pub const BUS_IN_CR: u8 = 0b0000010;
pub const BUS_IN_PS: u8 = 0b0000001;

pub const CNT_IP_INC: u8 = 0b100;
pub const CNT_SP_INC: u8 = 0b010;
pub const CNT_SP_DEC: u8 = 0b001;

pub const MEM_READ: u8 = 0b100;
pub const MEM_WRITE: u8 = 0b010;
pub const MEM_WAIT: u8 = 0b001;

pub const ALU_PASS: u8 = 0x00;
pub const ALU_ADD: u8 = 0x01;
pub const ALU_SUB: u8 = 0x02;
pub const ALU_MUL: u8 = 0x03;
pub const ALU_DIV: u8 = 0x04;
pub const ALU_MOD: u8 = 0x05;
pub const ALU_CMP: u8 = 0x06;
pub const ALU_AND: u8 = 0x07;
pub const ALU_OR: u8 = 0x08;
pub const ALU_XOR: u8 = 0x09;
pub const ALU_INV: u8 = 0x0A;
pub const ALU_ABS: u8 = 0x0B;
pub const ALU_INC: u8 = 0x0C;
pub const ALU_DEC: u8 = 0x0D;
pub const ALU_EXT8: u8 = 0x0E;
pub const ALU_EXT16: u8 = 0x0F;
pub const ALU_ADC: u8 = 0x10;

pub const SEQ_NEXT: u8 = 0x00;
pub const SEQ_JUMP: u8 = 0x01;
pub const SEQ_MAP: u8 = 0x02;

pub const COND_UNCOND: u8 = 0x00;
pub const COND_Z: u8 = 0x01;
pub const COND_N: u8 = 0x02;
pub const COND_C: u8 = 0x03;
pub const COND_V: u8 = 0x04;
pub const COND_INP: u8 = 0x05;

pub const COND_MODE1: u8 = 0x09;
pub const COND_MODE2: u8 = 0x0A;
pub const COND_MODE4: u8 = 0x0C;
pub const COND_MODE5: u8 = 0x0D;

pub const CACHE_LINES: usize = 128;
pub const CACHE_LINE_WORDS: usize = 16;
pub const RAM_WORDS: usize = 4194304;
pub const HIT_CYCLES: u32 = 1;
pub const MISS_CYCLES: u32 = 10;

pub const ADDR_INDEX_SHIFT: u32 = 6;
pub const ADDR_TAG_SHIFT: u32 = 13;
pub const ADDR_INDEX_MASK: u32 = 0x7F;
pub const ADDR_LINE_MASK: u32 = 0x3F;

pub struct ProcessorStatus {
    pub inp: bool,
    pub z: bool,
    pub n: bool,
    pub c: bool,
    pub v: bool,
}
