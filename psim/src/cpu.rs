use crate::consts::*;
use crate::mem::MemController;
use crate::microcode::{HLT_ADDR, MicroInstruction, MicroProcessor};

#[derive(Clone)]
struct AluResult {
    value: i32,
    z: bool,
    n: bool,
    c: bool,
    v: bool,
}

struct Registers {
    AC: i32,
    DR: i32,
    CR: u32,
    AR: u32,
    SP: u32,
    IP: u32,
    PS: ProcessorStatus,
}

pub struct CPU {
    mpu: MicroProcessor,
    registers: Registers,
    mem_controller: MemController,
    ticks: u64,
    halted: bool,
    alu_latch: Option<AluResult>,
    tracing: bool,
}

#[allow(dead_code)]
impl CPU {
    pub fn new() -> Self {
        CPU {
            mpu: MicroProcessor::new(),
            registers: Registers {
                AC: 0,
                DR: 0,
                CR: 0,
                AR: 0,
                SP: (RAM_WORDS * 4) as u32,
                IP: 8,
                PS: ProcessorStatus {
                    inp: false,
                    z: false,
                    n: false,
                    c: false,
                    v: false,
                },
            },
            mem_controller: MemController::new(),
            ticks: 0,
            halted: false,
            alu_latch: None,
            tracing: true,
        }
    }

    pub fn load_program(&mut self, byte_base: u32, data: &[i32]) {
        self.mem_controller.load_ram(byte_base, data);
    }

    pub fn set_ip(&mut self, ip: u32) {
        self.registers.IP = ip;
    }

    pub fn is_halted(&self) -> bool {
        self.halted
    }

    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    pub fn get_output(&self) -> &[i32] {
        &self.mem_controller.out_stream
    }

    pub fn add_input(&mut self, val: i32) {
        self.mem_controller.in_stream.push(val);
    }

    pub fn get_ac(&self) -> i32 {
        self.registers.AC
    }

    pub fn get_mpc(&self) -> u16 {
        self.mpu.mpc
    }

    pub fn get_cr(&self) -> u32 {
        self.registers.CR
    }

    fn sign_extend_21(val: u32) -> i32 {
        if val & 0x100000 != 0 {
            (val | 0xFFE00000) as i32
        } else {
            val as i32
        }
    }

    fn opcode_name(opcode: u8) -> &'static str {
        match opcode {
            0x00 => "NOP",
            0x01 => "HLT",
            0x02 => "CLA",
            0x03 => "CMA",
            0x04 => "INV",
            0x05 => "INC",
            0x06 => "DEC",
            0x07 => "ABS",
            0x08 => "PUSH",
            0x09 => "POP",
            0x0A => "RET",
            0x0B => "EXT8",
            0x0C => "EXT16",
            0x10 => "LD",
            0x11 => "ST",
            0x20 => "ADD",
            0x21 => "ADC",
            0x22 => "SUB",
            0x23 => "MUL",
            0x24 => "DIV",
            0x25 => "MOD",
            0x26 => "CMP",
            0x27 => "AND",
            0x28 => "OR",
            0x29 => "XOR",
            0x30 => "JMP",
            0x31 => "JEQ",
            0x32 => "JNE",
            0x33 => "JLT",
            0x34 => "JGE",
            0x35 => "JCS",
            0x36 => "JCC",
            0x37 => "JVS",
            0x38 => "JVC",
            0x39 => "CALL",
            _ => "???",
        }
    }

    pub fn disassemble(word: u32) -> String {
        let opcode = ((word >> 24) & 0xFF) as u8;
        let mode = ((word >> 21) & 0x7) as u8;
        let operand = word & 0x1FFFFF;

        let name = Self::opcode_name(opcode);

        if opcode <= 0x0C {
            return name.to_string();
        }

        match mode {
            0 => name.to_string(),
            1 => format!("{} #{}", name, Self::sign_extend_21(operand)),
            2 => format!("{} [{:#X}]", name, operand),
            3 => format!("{} [SP+{:#X}]", name, operand),
            4 => format!("{} ({:#X})+", name, operand),
            5 => format!("{} -({:#X})", name, operand),
            _ => format!("{} ?mode{}", name, mode),
        }
    }

    pub fn run(&mut self) {
        loop {
            self.step();
            if self.halted {
                break;
            }
        }
    }

    pub fn set_tracing(&mut self, enabled: bool) {
        self.tracing = enabled;
    }

    fn micro_action(micro: &MicroInstruction) -> String {
        let mut parts: Vec<&str> = Vec::new();
        match micro.bus_out {
            BUS_OUT_AC => parts.push("AC->bus"),
            BUS_OUT_DR => parts.push("DR->bus"),
            BUS_OUT_IP => parts.push("IP->bus"),
            BUS_OUT_SP => parts.push("SP->bus"),
            BUS_OUT_CR => parts.push("CR->bus"),
            BUS_OUT_ALU => parts.push("ALU->bus"),
            _ => {}
        }
        if micro.bus_in & BUS_IN_AC != 0 {
            parts.push("AC<-bus");
        }
        if micro.bus_in & BUS_IN_DR != 0 {
            parts.push("DR<-bus");
        }
        if micro.bus_in & BUS_IN_AR != 0 {
            parts.push("AR<-bus");
        }
        if micro.bus_in & BUS_IN_IP != 0 {
            parts.push("IP<-bus");
        }
        if micro.bus_in & BUS_IN_SP != 0 {
            parts.push("SP<-bus");
        }
        if micro.bus_in & BUS_IN_CR != 0 {
            parts.push("CR<-bus");
        }
        if micro.bus_in & BUS_IN_PS != 0 {
            parts.push("PS<-ALU");
        }
        if micro.cnt & CNT_IP_INC != 0 {
            parts.push("IP++");
        }
        if micro.cnt & CNT_SP_INC != 0 {
            parts.push("SP++");
        }
        if micro.cnt & CNT_SP_DEC != 0 {
            parts.push("SP--");
        }
        if micro.mem & MEM_READ != 0 {
            parts.push("Rd");
        }
        if micro.mem & MEM_WRITE != 0 {
            parts.push("Wr");
        }
        if micro.mem & MEM_WAIT != 0 {
            parts.push("Wait");
        }
        if micro.alu != ALU_PASS {
            let op = match micro.alu {
                ALU_ADD => "ADD",
                ALU_SUB => "SUB",
                ALU_MUL => "MUL",
                ALU_DIV => "DIV",
                ALU_MOD => "MOD",
                ALU_CMP => "CMP",
                ALU_AND => "AND",
                ALU_OR => "OR",
                ALU_XOR => "XOR",
                ALU_INV => "INV",
                ALU_ABS => "ABS",
                ALU_INC => "INC",
                ALU_DEC => "DEC",
                ALU_EXT8 => "EXT8",
                ALU_EXT16 => "EXT16",
                ALU_ADC => "ADC",
                _ => "???",
            };
            parts.push(op);
        }
        match micro.seq_ctrl {
            SEQ_NEXT => parts.push("NEXT"),
            SEQ_JUMP => parts.push("JMP"),
            SEQ_MAP => parts.push("MAP"),
            _ => {}
        }
        parts.join(" ")
    }

    pub fn step(&mut self) {
        if self.halted {
            return;
        }

        let micro = MicroInstruction::from(self.mpu.microcode[self.mpu.mpc as usize]);

        if self.tracing {
            let insn = Self::disassemble(self.registers.CR);
            println!(
                "TICK {:>6} | MPC={:#05x} {:30} | AC={:#010x} DR={:#010x} AR={:#010x} IP={:#010x} SP={:#010x} | I{}Z{}N{}C{}V{} | CR={:#010x} {}",
                self.ticks,
                self.mpu.mpc,
                Self::micro_action(&micro),
                self.registers.AC,
                self.registers.DR,
                self.registers.AR,
                self.registers.IP,
                self.registers.SP,
                self.registers.PS.inp as u8,
                self.registers.PS.z as u8,
                self.registers.PS.n as u8,
                self.registers.PS.c as u8,
                self.registers.PS.v as u8,
                self.registers.CR,
                insn,
            );
        }

        if (micro.mem & MEM_WAIT) != 0 && !self.mem_controller.is_ready() {
            self.mem_controller.tick();
            self.ticks += 1;
            return;
        }

        let bus_value = self.drive_bus(micro.bus_out);

        let (write_value, flag_z, flag_n, flag_c, flag_v) = if micro.alu != ALU_PASS {
            let result = Self::alu_execute(micro.alu, self.registers.AC, bus_value, self.registers.PS.c);
            self.alu_latch = Some(AluResult {
                value: result.value,
                z: result.z,
                n: result.n,
                c: result.c,
                v: result.v,
            });
            (result.value, result.z, result.n, result.c, result.v)
        } else if (micro.bus_out & BUS_OUT_ALU) != 0 {
            if let Some(ref latch) = self.alu_latch {
                (latch.value, latch.z, latch.n, latch.c, latch.v)
            } else {
                (0, true, false, false, false)
            }
        } else {
            (bus_value, bus_value == 0, bus_value < 0, false, false)
        };

        if (micro.bus_in & BUS_IN_AC) != 0 {
            self.registers.AC = write_value;
        }
        if (micro.bus_in & BUS_IN_DR) != 0 {
            self.registers.DR = write_value;
        }
        if (micro.bus_in & BUS_IN_AR) != 0 {
            self.registers.AR = write_value as u32;
        }
        if (micro.bus_in & BUS_IN_IP) != 0 {
            self.registers.IP = write_value as u32;
        }
        if (micro.bus_in & BUS_IN_SP) != 0 {
            self.registers.SP = write_value as u32;
        }
        if (micro.bus_in & BUS_IN_CR) != 0 {
            self.registers.CR = write_value as u32;
        }
        if (micro.bus_in & BUS_IN_PS) != 0 {
            self.registers.PS.z = flag_z;
            self.registers.PS.n = flag_n;
            self.registers.PS.c = flag_c;
            self.registers.PS.v = flag_v;
        }

        if (micro.mem & MEM_READ) != 0 {
            let data = self.mem_controller.read(self.registers.AR);
            if (micro.bus_in & BUS_IN_DR) == 0 {
                self.registers.DR = data;
            }
        }
        if (micro.mem & MEM_WRITE) != 0 {
            self.mem_controller
                .write(self.registers.AR, self.registers.DR);
        }

        if (micro.cnt & CNT_IP_INC) != 0 {
            self.registers.IP = self.registers.IP.wrapping_add(4);
        }
        if (micro.cnt & CNT_SP_INC) != 0 {
            self.registers.SP = self.registers.SP.wrapping_add(4);
        }
        if (micro.cnt & CNT_SP_DEC) != 0 {
            self.registers.SP = self.registers.SP.wrapping_sub(4);
        }

        if (micro.mem & MEM_WAIT) != 0 && !self.mem_controller.is_ready() {
            self.ticks += 1;
            return;
        }

        self.registers.PS.inp = !self.mem_controller.in_stream.is_empty();

        self.mpu.next_mpc(self.registers.CR, &self.registers.PS);

        if self.mpu.mpc == HLT_ADDR {
            let next_word = MicroInstruction::from(self.mpu.microcode[HLT_ADDR as usize]);
            if next_word.seq_ctrl == SEQ_JUMP
                && next_word.seq_cond == COND_UNCOND
                && next_word.next_addr == HLT_ADDR
            {
                self.halted = true;
            }
        }

        self.ticks += 1;
    }

    fn drive_bus(&self, bus_out: u8) -> i32 {
        if bus_out & BUS_OUT_AC != 0 {
            self.registers.AC
        } else if bus_out & BUS_OUT_DR != 0 {
            self.registers.DR
        } else if bus_out & BUS_OUT_IP != 0 {
            self.registers.IP as i32
        } else if bus_out & BUS_OUT_SP != 0 {
            self.registers.SP as i32
        } else if bus_out & BUS_OUT_CR != 0 {
            Self::sign_extend_21(self.registers.CR & 0x1FFFFF)
        } else if bus_out & BUS_OUT_ALU != 0 {
            self.alu_latch.as_ref().map_or(0, |r| r.value)
        } else {
            0
        }
    }

    fn alu_execute(op: u8, ac: i32, bus: i32, carry: bool) -> AluResult {
        match op {
            ALU_ADC => {
                let operand = bus + if carry { 1 } else { 0 };
                let (value, c) = ac.overflowing_add(operand);
                let v = ((ac ^ value) & (operand ^ value)) < 0;
                AluResult {
                    value,
                    z: value == 0,
                    n: value < 0,
                    c,
                    v,
                }
            }
            ALU_PASS => AluResult {
                value: bus,
                z: bus == 0,
                n: bus < 0,
                c: false,
                v: false,
            },
            ALU_ADD | ALU_INC => {
                let operand = if op == ALU_INC { 1 } else { bus };
                let (value, c) = ac.overflowing_add(operand);
                let v = ((ac ^ value) & (operand ^ value)) < 0;
                AluResult {
                    value,
                    z: value == 0,
                    n: value < 0,
                    c,
                    v,
                }
            }
            ALU_SUB | ALU_CMP | ALU_DEC => {
                let operand = if op == ALU_DEC { 1 } else { bus };
                let (value, c) = ac.overflowing_sub(operand);
                let v = ((ac ^ operand) & (ac ^ value)) < 0;
                AluResult {
                    value,
                    z: value == 0,
                    n: value < 0,
                    c,
                    v,
                }
            }
            ALU_MUL => {
                let (value, _) = ac.overflowing_mul(bus);
                AluResult {
                    value,
                    z: value == 0,
                    n: value < 0,
                    c: false,
                    v: false,
                }
            }
            ALU_DIV => {
                let value = if bus == 0 { 0 } else { ac / bus };
                AluResult {
                    value,
                    z: value == 0,
                    n: value < 0,
                    c: false,
                    v: false,
                }
            }
            ALU_MOD => {
                let value = if bus == 0 { 0 } else { ac % bus };
                AluResult {
                    value,
                    z: value == 0,
                    n: value < 0,
                    c: false,
                    v: false,
                }
            }
            ALU_AND => {
                let value = ac & bus;
                AluResult {
                    value,
                    z: value == 0,
                    n: value < 0,
                    c: false,
                    v: false,
                }
            }
            ALU_OR => {
                let value = ac | bus;
                AluResult {
                    value,
                    z: value == 0,
                    n: value < 0,
                    c: false,
                    v: false,
                }
            }
            ALU_XOR => {
                let value = ac ^ bus;
                AluResult {
                    value,
                    z: value == 0,
                    n: value < 0,
                    c: false,
                    v: false,
                }
            }
            ALU_INV => {
                let (value, v) = ac.overflowing_neg();
                AluResult {
                    value,
                    z: value == 0,
                    n: value < 0,
                    c: ac == 0,
                    v,
                }
            }
            ALU_ABS => {
                let (value, v) = ac.overflowing_abs();
                AluResult {
                    value,
                    z: value == 0,
                    n: value < 0,
                    c: false,
                    v,
                }
            }
            ALU_EXT8 => {
                let value = (ac as i8) as i32;
                AluResult {
                    value,
                    z: value == 0,
                    n: value < 0,
                    c: false,
                    v: false,
                }
            }
            ALU_EXT16 => {
                let value = (ac as i16) as i32;
                AluResult {
                    value,
                    z: value == 0,
                    n: value < 0,
                    c: false,
                    v: false,
                }
            }
            _ => AluResult {
                value: 0,
                z: true,
                n: false,
                c: false,
                v: false,
            },
        }
    }
}
