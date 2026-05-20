use crate::consts::*;

pub struct MicroInstruction {
    pub bus_out: u8,
    pub bus_in: u8,
    pub cnt: u8,
    pub mem: u8,
    pub alu: u8,
    pub seq_ctrl: u8,
    pub seq_cond: u8,
    pub next_addr: u16,
}

impl From<u64> for MicroInstruction {
    fn from(word: u64) -> Self {
        Self {
            bus_out: ((word >> 34) & 0x3F) as u8,
            bus_in: ((word >> 27) & 0x7F) as u8,
            cnt: ((word >> 24) & 0x07) as u8,
            mem: ((word >> 21) & 0x07) as u8,
            alu: ((word >> 16) & 0x1F) as u8,
            seq_ctrl: ((word >> 14) & 0x03) as u8,
            seq_cond: ((word >> 10) & 0x0F) as u8,
            next_addr: (word & 0x3FF) as u16,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) const fn micro(
    bus_out: u8,
    bus_in: u8,
    cnt: u8,
    mem: u8,
    alu: u8,
    seq_ctrl: u8,
    seq_cond: u8,
    next_addr: u16,
) -> u64 {
    (bus_out as u64) << 34
        | (bus_in as u64) << 27
        | (cnt as u64) << 24
        | (mem as u64) << 21
        | (alu as u64) << 16
        | (seq_ctrl as u64) << 14
        | (seq_cond as u64) << 10
        | next_addr as u64
}

const FETCH: u16 = 0x000;

const NOP_ADDR: u16 = 0x100;
pub(crate) const HLT_ADDR: u16 = 0x101;
const CLA_ADDR: u16 = 0x102;
const CMA_ADDR: u16 = 0x104;
const INV_ADDR: u16 = 0x106;
const INC_ADDR: u16 = 0x107;
const DEC_ADDR: u16 = 0x108;
const ABS_ADDR: u16 = 0x109;
const EXT8_ADDR: u16 = 0x10A;
const EXT16_ADDR: u16 = 0x10B;
const PUSH_ADDR: u16 = 0x110;
const POP_ADDR: u16 = 0x112;
const RET_ADDR: u16 = 0x114;

const LD_IMM: u16 = 0x200;
const LD_DIR: u16 = 0x202;
const LD_AINC: u16 = 0x205;
const LD_ADEC: u16 = 0x20D;

const ST_DIR: u16 = 0x215;
const ST_AINC: u16 = 0x218;
const ST_ADEC: u16 = 0x222;

const ADD_IMM: u16 = 0x300;
const ADD_DIR: u16 = 0x302;
const ADC_IMM: u16 = 0x308;
const ADC_DIR: u16 = 0x30B;
const SUB_IMM: u16 = 0x310;
const SUB_DIR: u16 = 0x312;
const MUL_IMM: u16 = 0x318;
const MUL_DIR: u16 = 0x31A;
const DIV_IMM: u16 = 0x320;
const DIV_DIR: u16 = 0x322;
const MOD_IMM: u16 = 0x328;
const MOD_DIR: u16 = 0x32A;
const AND_IMM: u16 = 0x330;
const AND_DIR: u16 = 0x332;
const OR_IMM: u16 = 0x338;
const OR_DIR: u16 = 0x33A;
const XOR_IMM: u16 = 0x340;
const XOR_DIR: u16 = 0x342;
const CMP_IMM: u16 = 0x348;
const CMP_DIR: u16 = 0x34A;

const JMP_ADDR: u16 = 0x030;
const JEQ_ADDR: u16 = 0x034;
const JNE_ADDR: u16 = 0x038;
const JLT_ADDR: u16 = 0x03C;
const JGE_ADDR: u16 = 0x040;
const JCS_ADDR: u16 = 0x044;
const JCC_ADDR: u16 = 0x048;
const JVS_ADDR: u16 = 0x04C;
const JVC_ADDR: u16 = 0x050;
const CALL_ADDR: u16 = 0x054;

// Mode dispatch addresses
const LD_DISPATCH: u16 = 0x120;
const ST_DISPATCH: u16 = 0x125;
const ADD_DISPATCH: u16 = 0x130;
const ADC_DISPATCH: u16 = 0x133;
const SUB_DISPATCH: u16 = 0x136;
const MUL_DISPATCH: u16 = 0x139;
const DIV_DISPATCH: u16 = 0x13C;
const MOD_DISPATCH: u16 = 0x13F;
const CMP_DISPATCH: u16 = 0x142;
const AND_DISPATCH: u16 = 0x145;
const OR_DISPATCH: u16 = 0x148;
const XOR_DISPATCH: u16 = 0x14B;

pub struct MicroProcessor {
    pub microcode: [u64; 2048],
    pub mapping_rom: [u16; 256],
    pub mpc: u16,
}

impl MicroProcessor {
    pub fn new() -> Self {
        let mut mp = MicroProcessor {
            microcode: [0; 2048],
            mapping_rom: [0; 256],
            mpc: 0,
        };
        mp.init_mapping_rom();
        mp.init_microcode();
        mp
    }

    pub fn check_condition(ps: &ProcessorStatus, cr: u32, seq_cond: u8) -> bool {
        match seq_cond {
            COND_UNCOND => true,
            COND_Z => ps.z,
            COND_N => ps.n,
            COND_C => ps.c,
            COND_V => ps.v,
            COND_INP => ps.inp,
            0x08..=0x0D => ((cr >> 21) & 0x7) == (seq_cond - 0x08) as u32,
            _ => false,
        }
    }

    pub fn next_mpc(&mut self, cr: u32, ps: &ProcessorStatus) {
        let word = self.microcode[self.mpc as usize];
        let micro_instruction = MicroInstruction::from(word);

        match micro_instruction.seq_ctrl {
            SEQ_NEXT => self.mpc = self.mpc.wrapping_add(1),
            SEQ_JUMP => {
                if Self::check_condition(ps, cr, micro_instruction.seq_cond) {
                    self.mpc = micro_instruction.next_addr;
                } else {
                    self.mpc = self.mpc.wrapping_add(1);
                }
            }
            SEQ_MAP => {
                let opcode = ((cr >> 24) & 0xFF) as usize;
                self.mpc = self.mapping_rom[opcode];
            }
            _ => self.mpc = self.mpc.wrapping_add(1),
        }
    }

    fn init_mapping_rom(&mut self) {
        self.mapping_rom[0x00] = NOP_ADDR;
        self.mapping_rom[0x01] = HLT_ADDR;
        self.mapping_rom[0x02] = CLA_ADDR;
        self.mapping_rom[0x03] = CMA_ADDR;
        self.mapping_rom[0x04] = INV_ADDR;
        self.mapping_rom[0x05] = INC_ADDR;
        self.mapping_rom[0x06] = DEC_ADDR;
        self.mapping_rom[0x07] = ABS_ADDR;
        self.mapping_rom[0x08] = PUSH_ADDR;
        self.mapping_rom[0x09] = POP_ADDR;
        self.mapping_rom[0x0A] = RET_ADDR;
        self.mapping_rom[0x0B] = EXT8_ADDR;
        self.mapping_rom[0x0C] = EXT16_ADDR;

        self.mapping_rom[0x10] = LD_DISPATCH;
        self.mapping_rom[0x11] = ST_DISPATCH;

        self.mapping_rom[0x20] = ADD_DISPATCH;
        self.mapping_rom[0x21] = ADC_DISPATCH;
        self.mapping_rom[0x22] = SUB_DISPATCH;
        self.mapping_rom[0x23] = MUL_DISPATCH;
        self.mapping_rom[0x24] = DIV_DISPATCH;
        self.mapping_rom[0x25] = MOD_DISPATCH;
        self.mapping_rom[0x26] = CMP_DISPATCH;
        self.mapping_rom[0x27] = AND_DISPATCH;
        self.mapping_rom[0x28] = OR_DISPATCH;
        self.mapping_rom[0x29] = XOR_DISPATCH;

        self.mapping_rom[0x30] = JMP_ADDR;
        self.mapping_rom[0x31] = JEQ_ADDR;
        self.mapping_rom[0x32] = JNE_ADDR;
        self.mapping_rom[0x33] = JLT_ADDR;
        self.mapping_rom[0x34] = JGE_ADDR;
        self.mapping_rom[0x35] = JCS_ADDR;
        self.mapping_rom[0x36] = JCC_ADDR;
        self.mapping_rom[0x37] = JVS_ADDR;
        self.mapping_rom[0x38] = JVC_ADDR;
        self.mapping_rom[0x39] = CALL_ADDR;
    }

    fn init_microcode(&mut self) {
        self.microcode[0] = micro(BUS_OUT_IP, BUS_IN_AR, 0, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[1] = micro(0, 0, 0, MEM_READ | MEM_WAIT, 0, SEQ_NEXT, 0, 0);
        self.microcode[2] = micro(BUS_OUT_DR, BUS_IN_CR, CNT_IP_INC, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[3] = micro(0, 0, 0, 0, 0, SEQ_MAP, 0, 0);

        self.microcode[NOP_ADDR as usize] = micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);
        self.microcode[HLT_ADDR as usize] = micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_UNCOND, HLT_ADDR);

        self.microcode[CLA_ADDR as usize] = micro(BUS_OUT_AC, 0, 0, 0, ALU_SUB, SEQ_NEXT, 0, 0);
        self.microcode[CLA_ADDR as usize + 1] = micro(
            BUS_OUT_ALU,
            BUS_IN_AC | BUS_IN_PS,
            0,
            0,
            0,
            SEQ_JUMP,
            COND_UNCOND,
            FETCH,
        );

        self.microcode[CMA_ADDR as usize] = micro(0, BUS_IN_AC, 0, 0, ALU_INV, SEQ_NEXT, 0, 0);
        self.microcode[CMA_ADDR as usize + 1] = micro(
            0,
            BUS_IN_AC | BUS_IN_PS,
            0,
            0,
            ALU_DEC,
            SEQ_JUMP,
            COND_UNCOND,
            FETCH,
        );

        self.microcode[INV_ADDR as usize] = micro(
            0,
            BUS_IN_AC | BUS_IN_PS,
            0,
            0,
            ALU_INV,
            SEQ_JUMP,
            COND_UNCOND,
            FETCH,
        );
        self.microcode[INC_ADDR as usize] = micro(
            0,
            BUS_IN_AC | BUS_IN_PS,
            0,
            0,
            ALU_INC,
            SEQ_JUMP,
            COND_UNCOND,
            FETCH,
        );
        self.microcode[DEC_ADDR as usize] = micro(
            0,
            BUS_IN_AC | BUS_IN_PS,
            0,
            0,
            ALU_DEC,
            SEQ_JUMP,
            COND_UNCOND,
            FETCH,
        );
        self.microcode[ABS_ADDR as usize] = micro(
            0,
            BUS_IN_AC | BUS_IN_PS,
            0,
            0,
            ALU_ABS,
            SEQ_JUMP,
            COND_UNCOND,
            FETCH,
        );
        self.microcode[EXT8_ADDR as usize] = micro(
            0,
            BUS_IN_AC | BUS_IN_PS,
            0,
            0,
            ALU_EXT8,
            SEQ_JUMP,
            COND_UNCOND,
            FETCH,
        );
        self.microcode[EXT16_ADDR as usize] = micro(
            0,
            BUS_IN_AC | BUS_IN_PS,
            0,
            0,
            ALU_EXT16,
            SEQ_JUMP,
            COND_UNCOND,
            FETCH,
        );

        self.microcode[PUSH_ADDR as usize] =
            micro(BUS_OUT_AC, BUS_IN_DR, CNT_SP_DEC, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[PUSH_ADDR as usize + 1] = micro(
            BUS_OUT_SP,
            BUS_IN_AR,
            0,
            MEM_WRITE | MEM_WAIT,
            0,
            SEQ_JUMP,
            COND_UNCOND,
            FETCH,
        );
        self.microcode[POP_ADDR as usize] = micro(
            BUS_OUT_SP,
            BUS_IN_AR,
            CNT_SP_INC,
            MEM_READ | MEM_WAIT,
            0,
            SEQ_NEXT,
            0,
            0,
        );
        self.microcode[POP_ADDR as usize + 1] =
            micro(BUS_OUT_DR, BUS_IN_AC, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);
        self.microcode[RET_ADDR as usize] = micro(
            BUS_OUT_SP,
            BUS_IN_AR,
            CNT_SP_INC,
            MEM_READ | MEM_WAIT,
            0,
            SEQ_NEXT,
            0,
            0,
        );
        self.microcode[RET_ADDR as usize + 1] =
            micro(BUS_OUT_DR, BUS_IN_IP, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);

        self.microcode[LD_DISPATCH as usize] = micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_MODE1, LD_IMM);
        self.microcode[LD_DISPATCH as usize + 1] =
            micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_MODE2, LD_DIR);
        self.microcode[LD_DISPATCH as usize + 2] =
            micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_MODE4, LD_AINC);
        self.microcode[LD_DISPATCH as usize + 3] =
            micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_MODE5, LD_ADEC);
        self.microcode[LD_DISPATCH as usize + 4] =
            micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);

        self.microcode[ST_DISPATCH as usize] = micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_MODE2, ST_DIR);
        self.microcode[ST_DISPATCH as usize + 1] =
            micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_MODE4, ST_AINC);
        self.microcode[ST_DISPATCH as usize + 2] =
            micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_MODE5, ST_ADEC);
        self.microcode[ST_DISPATCH as usize + 3] =
            micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);

        macro_rules! alu_dispatch {
            ($addr:expr, $imm:expr, $dir:expr) => {
                self.microcode[$addr as usize] = micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_MODE1, $imm);
                self.microcode[$addr as usize + 1] =
                    micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_MODE2, $dir);
                self.microcode[$addr as usize + 2] =
                    micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);
            };
        }

        alu_dispatch!(ADD_DISPATCH, ADD_IMM, ADD_DIR);
        alu_dispatch!(ADC_DISPATCH, ADC_IMM, ADC_DIR);
        alu_dispatch!(SUB_DISPATCH, SUB_IMM, SUB_DIR);
        alu_dispatch!(MUL_DISPATCH, MUL_IMM, MUL_DIR);
        alu_dispatch!(DIV_DISPATCH, DIV_IMM, DIV_DIR);
        alu_dispatch!(MOD_DISPATCH, MOD_IMM, MOD_DIR);
        alu_dispatch!(CMP_DISPATCH, CMP_IMM, CMP_DIR);
        alu_dispatch!(AND_DISPATCH, AND_IMM, AND_DIR);
        alu_dispatch!(OR_DISPATCH, OR_IMM, OR_DIR);
        alu_dispatch!(XOR_DISPATCH, XOR_IMM, XOR_DIR);

        self.microcode[LD_IMM as usize] = micro(BUS_OUT_CR, BUS_IN_DR, 0, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[LD_IMM as usize + 1] =
            micro(BUS_OUT_DR, BUS_IN_AC, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);

        self.microcode[LD_DIR as usize] = micro(BUS_OUT_CR, BUS_IN_AR, 0, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[LD_DIR as usize + 1] =
            micro(0, 0, 0, MEM_READ | MEM_WAIT, 0, SEQ_NEXT, 0, 0);
        self.microcode[LD_DIR as usize + 2] =
            micro(BUS_OUT_DR, BUS_IN_AC, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);

        self.microcode[LD_AINC as usize] = micro(BUS_OUT_CR, BUS_IN_AR, 0, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[LD_AINC as usize + 1] =
            micro(0, 0, 0, MEM_READ | MEM_WAIT, 0, SEQ_NEXT, 0, 0);
        self.microcode[LD_AINC as usize + 2] =
            micro(BUS_OUT_DR, BUS_IN_AC, 0, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[LD_AINC as usize + 3] =
            micro(BUS_OUT_AC, BUS_IN_DR, 0, 0, ALU_INC, SEQ_NEXT, 0, 0);
        self.microcode[LD_AINC as usize + 4] =
            micro(0, 0, 0, MEM_WRITE | MEM_WAIT, 0, SEQ_NEXT, 0, 0);
        self.microcode[LD_AINC as usize + 5] =
            micro(BUS_OUT_AC, BUS_IN_AR, 0, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[LD_AINC as usize + 6] =
            micro(0, 0, 0, MEM_READ | MEM_WAIT, 0, SEQ_NEXT, 0, 0);
        self.microcode[LD_AINC as usize + 7] =
            micro(BUS_OUT_DR, BUS_IN_AC, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);

        self.microcode[LD_ADEC as usize] = micro(BUS_OUT_CR, BUS_IN_AR, 0, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[LD_ADEC as usize + 1] =
            micro(0, 0, 0, MEM_READ | MEM_WAIT, 0, SEQ_NEXT, 0, 0);
        self.microcode[LD_ADEC as usize + 2] =
            micro(BUS_OUT_DR, BUS_IN_AC, 0, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[LD_ADEC as usize + 3] =
            micro(BUS_OUT_AC, BUS_IN_DR, 0, 0, ALU_DEC, SEQ_NEXT, 0, 0);
        self.microcode[LD_ADEC as usize + 4] =
            micro(0, 0, 0, MEM_WRITE | MEM_WAIT, 0, SEQ_NEXT, 0, 0);
        self.microcode[LD_ADEC as usize + 5] =
            micro(BUS_OUT_DR, BUS_IN_AR, 0, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[LD_ADEC as usize + 6] =
            micro(0, 0, 0, MEM_READ | MEM_WAIT, 0, SEQ_NEXT, 0, 0);
        self.microcode[LD_ADEC as usize + 7] =
            micro(BUS_OUT_DR, BUS_IN_AC, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);

        self.microcode[ST_DIR as usize] = micro(BUS_OUT_CR, BUS_IN_AR, 0, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[ST_DIR as usize + 1] = micro(BUS_OUT_AC, BUS_IN_DR, 0, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[ST_DIR as usize + 2] = micro(
            0,
            0,
            0,
            MEM_WRITE | MEM_WAIT,
            0,
            SEQ_JUMP,
            COND_UNCOND,
            FETCH,
        );

        self.microcode[ST_AINC as usize] = micro(BUS_OUT_CR, BUS_IN_AR, 0, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[ST_AINC as usize + 1] =
            micro(0, 0, 0, MEM_READ | MEM_WAIT, 0, SEQ_NEXT, 0, 0);
        self.microcode[ST_AINC as usize + 2] =
            micro(BUS_OUT_DR, BUS_IN_AR, 0, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[ST_AINC as usize + 3] =
            micro(BUS_OUT_AC, BUS_IN_DR, 0, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[ST_AINC as usize + 4] =
            micro(0, 0, 0, MEM_WRITE | MEM_WAIT, 0, SEQ_NEXT, 0, 0);
        self.microcode[ST_AINC as usize + 5] =
            micro(BUS_OUT_CR, BUS_IN_AR, 0, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[ST_AINC as usize + 6] =
            micro(0, 0, 0, MEM_READ | MEM_WAIT, 0, SEQ_NEXT, 0, 0);
        self.microcode[ST_AINC as usize + 7] =
            micro(BUS_OUT_DR, BUS_IN_AC, 0, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[ST_AINC as usize + 8] =
            micro(BUS_OUT_AC, BUS_IN_DR, 0, 0, ALU_INC, SEQ_NEXT, 0, 0);
        self.microcode[ST_AINC as usize + 9] = micro(
            0,
            0,
            0,
            MEM_WRITE | MEM_WAIT,
            0,
            SEQ_JUMP,
            COND_UNCOND,
            FETCH,
        );

        self.microcode[ST_ADEC as usize] = micro(BUS_OUT_CR, BUS_IN_AR, 0, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[ST_ADEC as usize + 1] =
            micro(0, 0, 0, MEM_READ | MEM_WAIT, 0, SEQ_NEXT, 0, 0);
        self.microcode[ST_ADEC as usize + 2] =
            micro(BUS_OUT_DR, BUS_IN_AR, 0, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[ST_ADEC as usize + 3] =
            micro(BUS_OUT_AC, BUS_IN_DR, 0, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[ST_ADEC as usize + 4] =
            micro(0, 0, 0, MEM_WRITE | MEM_WAIT, 0, SEQ_NEXT, 0, 0);
        self.microcode[ST_ADEC as usize + 5] =
            micro(BUS_OUT_CR, BUS_IN_AR, 0, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[ST_ADEC as usize + 6] =
            micro(0, 0, 0, MEM_READ | MEM_WAIT, 0, SEQ_NEXT, 0, 0);
        self.microcode[ST_ADEC as usize + 7] =
            micro(BUS_OUT_DR, BUS_IN_AC, 0, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[ST_ADEC as usize + 8] =
            micro(BUS_OUT_AC, BUS_IN_DR, 0, 0, ALU_DEC, SEQ_NEXT, 0, 0);
        self.microcode[ST_ADEC as usize + 9] = micro(
            0,
            0,
            0,
            MEM_WRITE | MEM_WAIT,
            0,
            SEQ_JUMP,
            COND_UNCOND,
            FETCH,
        );

        macro_rules! alu_op {
            ($imm:expr, $dir:expr, $op:ident) => {
                self.microcode[$imm as usize] = micro(BUS_OUT_CR, 0, 0, 0, $op, SEQ_NEXT, 0, 0);
                self.microcode[$imm as usize + 1] = micro(
                    BUS_OUT_ALU,
                    BUS_IN_AC | BUS_IN_PS,
                    0,
                    0,
                    0,
                    SEQ_JUMP,
                    COND_UNCOND,
                    FETCH,
                );
                self.microcode[$dir as usize] =
                    micro(BUS_OUT_CR, BUS_IN_AR, 0, 0, 0, SEQ_NEXT, 0, 0);
                self.microcode[$dir as usize + 1] =
                    micro(0, 0, 0, MEM_READ | MEM_WAIT, 0, SEQ_NEXT, 0, 0);
                self.microcode[$dir as usize + 2] = micro(BUS_OUT_DR, 0, 0, 0, $op, SEQ_NEXT, 0, 0);
                self.microcode[$dir as usize + 3] = micro(
                    BUS_OUT_ALU,
                    BUS_IN_AC | BUS_IN_PS,
                    0,
                    0,
                    0,
                    SEQ_JUMP,
                    COND_UNCOND,
                    FETCH,
                );
            };
        }

        alu_op!(ADD_IMM, ADD_DIR, ALU_ADD);
        alu_op!(SUB_IMM, SUB_DIR, ALU_SUB);
        alu_op!(MUL_IMM, MUL_DIR, ALU_MUL);
        alu_op!(DIV_IMM, DIV_DIR, ALU_DIV);
        alu_op!(MOD_IMM, MOD_DIR, ALU_MOD);
        alu_op!(AND_IMM, AND_DIR, ALU_AND);
        alu_op!(OR_IMM, OR_DIR, ALU_OR);
        alu_op!(XOR_IMM, XOR_DIR, ALU_XOR);

        self.microcode[CMP_IMM as usize] = micro(BUS_OUT_CR, 0, 0, 0, ALU_CMP, SEQ_NEXT, 0, 0);
        self.microcode[CMP_IMM as usize + 1] = micro(
            BUS_OUT_ALU,
            BUS_IN_PS,
            0,
            0,
            0,
            SEQ_JUMP,
            COND_UNCOND,
            FETCH,
        );
        self.microcode[CMP_DIR as usize] = micro(BUS_OUT_CR, BUS_IN_AR, 0, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[CMP_DIR as usize + 1] =
            micro(0, 0, 0, MEM_READ | MEM_WAIT, 0, SEQ_NEXT, 0, 0);
        self.microcode[CMP_DIR as usize + 2] = micro(BUS_OUT_DR, 0, 0, 0, ALU_CMP, SEQ_NEXT, 0, 0);
        self.microcode[CMP_DIR as usize + 3] = micro(
            BUS_OUT_ALU,
            BUS_IN_PS,
            0,
            0,
            0,
            SEQ_JUMP,
            COND_UNCOND,
            FETCH,
        );

        self.microcode[ADC_IMM as usize] = micro(BUS_OUT_CR, 0, 0, 0, ALU_ADC, SEQ_NEXT, 0, 0);
        self.microcode[ADC_IMM as usize + 1] = micro(
            BUS_OUT_ALU,
            BUS_IN_AC | BUS_IN_PS,
            0,
            0,
            0,
            SEQ_JUMP,
            COND_UNCOND,
            FETCH,
        );
        self.microcode[ADC_DIR as usize] = micro(BUS_OUT_CR, BUS_IN_AR, 0, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[ADC_DIR as usize + 1] =
            micro(0, 0, 0, MEM_READ | MEM_WAIT, 0, SEQ_NEXT, 0, 0);
        self.microcode[ADC_DIR as usize + 2] = micro(BUS_OUT_DR, 0, 0, 0, ALU_ADC, SEQ_NEXT, 0, 0);
        self.microcode[ADC_DIR as usize + 3] = micro(
            BUS_OUT_ALU,
            BUS_IN_AC | BUS_IN_PS,
            0,
            0,
            0,
            SEQ_JUMP,
            COND_UNCOND,
            FETCH,
        );

        self.microcode[JMP_ADDR as usize] =
            micro(BUS_OUT_CR, BUS_IN_IP, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);

        self.microcode[JEQ_ADDR as usize] = micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_Z, JEQ_ADDR + 2);
        self.microcode[JEQ_ADDR as usize + 1] = micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);
        self.microcode[JEQ_ADDR as usize + 2] =
            micro(BUS_OUT_CR, BUS_IN_IP, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);

        self.microcode[JNE_ADDR as usize] = micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_Z, JNE_ADDR + 2);
        self.microcode[JNE_ADDR as usize + 1] =
            micro(BUS_OUT_CR, BUS_IN_IP, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);
        self.microcode[JNE_ADDR as usize + 2] = micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);

        self.microcode[JLT_ADDR as usize] = micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_N, JLT_ADDR + 2);
        self.microcode[JLT_ADDR as usize + 1] = micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);
        self.microcode[JLT_ADDR as usize + 2] =
            micro(BUS_OUT_CR, BUS_IN_IP, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);

        self.microcode[JGE_ADDR as usize] = micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_N, JGE_ADDR + 2);
        self.microcode[JGE_ADDR as usize + 1] =
            micro(BUS_OUT_CR, BUS_IN_IP, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);
        self.microcode[JGE_ADDR as usize + 2] = micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);

        self.microcode[JCS_ADDR as usize] = micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_C, JCS_ADDR + 2);
        self.microcode[JCS_ADDR as usize + 1] = micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);
        self.microcode[JCS_ADDR as usize + 2] =
            micro(BUS_OUT_CR, BUS_IN_IP, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);

        self.microcode[JCC_ADDR as usize] = micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_C, JCC_ADDR + 2);
        self.microcode[JCC_ADDR as usize + 1] =
            micro(BUS_OUT_CR, BUS_IN_IP, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);
        self.microcode[JCC_ADDR as usize + 2] = micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);

        self.microcode[JVS_ADDR as usize] = micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_V, JVS_ADDR + 2);
        self.microcode[JVS_ADDR as usize + 1] = micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);
        self.microcode[JVS_ADDR as usize + 2] =
            micro(BUS_OUT_CR, BUS_IN_IP, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);

        self.microcode[JVC_ADDR as usize] = micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_V, JVC_ADDR + 2);
        self.microcode[JVC_ADDR as usize + 1] =
            micro(BUS_OUT_CR, BUS_IN_IP, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);
        self.microcode[JVC_ADDR as usize + 2] = micro(0, 0, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);

        self.microcode[CALL_ADDR as usize] =
            micro(BUS_OUT_IP, BUS_IN_DR, CNT_SP_DEC, 0, 0, SEQ_NEXT, 0, 0);
        self.microcode[CALL_ADDR as usize + 1] = micro(
            BUS_OUT_SP,
            BUS_IN_AR,
            0,
            MEM_WRITE | MEM_WAIT,
            0,
            SEQ_NEXT,
            0,
            0,
        );
        self.microcode[CALL_ADDR as usize + 2] =
            micro(BUS_OUT_CR, BUS_IN_IP, 0, 0, 0, SEQ_JUMP, COND_UNCOND, FETCH);
    }
}
