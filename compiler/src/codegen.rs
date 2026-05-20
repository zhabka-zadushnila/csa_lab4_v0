use crate::parser::{Expression, Ops, Value};
use std::collections::HashMap;

const CODE_BASE: u32 = 8;

const IN_PORT: u32 = 0x0000;
const OUT_PORT: u32 = 0x0004;

const OP_HLT: u8 = 0x01;
const OP_CLA: u8 = 0x02;
const OP_INV: u8 = 0x04;
const OP_INC: u8 = 0x05;
const OP_DEC: u8 = 0x06;
const OP_ABS: u8 = 0x07;
const OP_PUSH: u8 = 0x08;
const OP_POP: u8 = 0x09;
const OP_RET: u8 = 0x0A;
const OP_LD: u8 = 0x10;
const OP_ST: u8 = 0x11;
const OP_ADD: u8 = 0x20;
const OP_SUB: u8 = 0x22;
const OP_MUL: u8 = 0x23;
const OP_DIV: u8 = 0x24;
const OP_MOD: u8 = 0x25;
const OP_CMP: u8 = 0x26;
const OP_AND: u8 = 0x27;
const OP_OR: u8 = 0x28;
const OP_XOR: u8 = 0x29;
const OP_JMP: u8 = 0x30;
const OP_JEQ: u8 = 0x31;
const OP_JNE: u8 = 0x32;
const OP_JLT: u8 = 0x33;
const OP_JGE: u8 = 0x34;
const OP_CALL: u8 = 0x39;

const MODE_DIRECT: u8 = 2;
const MODE_AUTO_INC: u8 = 4;

fn encode(opcode: u8, mode: u8, operand: u32) -> u32 {
    (opcode as u32) << 24 | (mode as u32) << 21 | (operand & 0x1FFFFF)
}

fn mnemonic(instruction: u32) -> String {
    let opcode = (instruction >> 24) & 0xFF;
    let mode = (instruction >> 21) & 7;
    let operand = instruction & 0x1FFFFF;
    let name = match opcode {
        0x01 => "HLT",
        0x02 => "CLA",
        0x04 => "INV",
        0x05 => "INC",
        0x06 => "DEC",
        0x07 => "ABS",
        0x08 => "PUSH",
        0x09 => "POP",
        0x0A => "RET",
        0x10 => "LD",
        0x11 => "ST",
        0x20 => "ADD",
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
        0x39 => "CALL",
        _ => "???",
    };
    match mode {
        0 => name.into(),
        1 => format!("{} #{}", name, operand as i32),
        2 => format!("{} [0x{:05X}]", name, operand),
        3 => format!("{} [SP+0x{:05X}]", name, operand),
        4 => format!("{} (0x{:05X})+", name, operand),
        5 => format!("{} -(0x{:05X})", name, operand),
        _ => format!("{} ?{}? 0x{:05X}", name, mode, operand),
    }
}

#[derive(Clone)]
#[allow(dead_code)]
struct Var {
    addr: u32,
    typ: String,
    size: u32,
}

struct Fixup {
    address: u32,
    symbol: String,
    offset: u32,
}

pub struct Codegen {
    instructions: Vec<u32>,
    data_values: Vec<u32>,
    variables: HashMap<String, Var>,
    string_offsets: HashMap<String, u32>,
    labels: HashMap<String, u32>,
    fixups: Vec<Fixup>,
    bss_entries: Vec<(String, u32)>,
    label_counter: u32,
    temp_counter: u32,
    code_address: u32,
    data_address: u32,
    in_main_function: bool,
    pub listing: Vec<String>,
}

impl Codegen {
    pub fn new() -> Self {
        Codegen {
            instructions: Vec::new(),
            data_values: Vec::new(),
            variables: HashMap::new(),
            string_offsets: HashMap::new(),
            labels: HashMap::new(),
            fixups: Vec::new(),
            bss_entries: Vec::new(),
            label_counter: 0,
            temp_counter: 0,
            code_address: 0,
            data_address: 0,
            in_main_function: false,
            listing: Vec::new(),
        }
    }

    fn new_label(&mut self) -> String {
        let label_name = format!(".L{}", self.label_counter);
        self.label_counter += 1;
        label_name
    }

    fn new_temp(&mut self) -> String {
        let var_name = format!("__t{}", self.temp_counter);
        self.temp_counter += 1;
        self.bss_entries.push((var_name.clone(), 1));
        self.variables.insert(
            var_name.clone(),
            Var {
                addr: 0,
                typ: "i32".into(),
                size: 1,
            },
        );
        var_name
    }

    fn type_size(&self, _type_name: &str) -> u32 {
        1
    }

    fn emit(&mut self, opcode: u8, mode: u8, operand: u32) {
        let instruction = encode(opcode, mode, operand);
        self.listing.push(format!(
            "0x{:04X}  0x{:08X}  {}",
            self.code_address + CODE_BASE,
            instruction,
            mnemonic(instruction)
        ));
        self.instructions.push(instruction);
        self.code_address += 4;
    }

    fn emit_direct(&mut self, opcode: u8, address: u32) {
        self.emit(opcode, MODE_DIRECT, address);
    }
    fn emit_immediate(&mut self, opcode: u8, value: i32) {
        self.emit(opcode, 1, value as u32);
    }
    fn emit_register(&mut self, opcode: u8) {
        self.emit(opcode, 0, 0);
    }

    fn emit_variable_ref(&mut self, opcode: u8, symbol: &str, offset: u32) {
        let instruction = encode(opcode, MODE_DIRECT, offset);
        let idx = self.instructions.len() as u32;
        self.listing.push(format!(
            "0x{:04X}  0x{:08X}  {} {} (fixup +{})",
            self.code_address + CODE_BASE,
            instruction,
            mnemonic(instruction),
            symbol,
            offset
        ));
        self.instructions.push(instruction);
        self.code_address += 4;
        self.fixups.push(Fixup {
            address: idx,
            symbol: symbol.to_string(),
            offset,
        });
    }

    fn emit_immediate_ref(&mut self, opcode: u8, symbol: &str) {
        let instruction = encode(opcode, 1, 0);
        let idx = self.instructions.len() as u32;
        self.listing.push(format!(
            "0x{:04X}  0x{:08X}  {} #{} (fixup)",
            self.code_address + CODE_BASE,
            instruction,
            mnemonic(instruction),
            symbol
        ));
        self.instructions.push(instruction);
        self.code_address += 4;
        self.fixups.push(Fixup {
            address: idx,
            symbol: symbol.to_string(),
            offset: 0,
        });
    }

    fn emit_auto_inc_ref(&mut self, opcode: u8, symbol: &str) {
        let instruction = encode(opcode, MODE_AUTO_INC, 0);
        let idx = self.instructions.len() as u32;
        self.listing.push(format!(
            "0x{:04X}  0x{:08X}  {} ({})+ (fixup)",
            self.code_address + CODE_BASE,
            instruction,
            mnemonic(instruction),
            symbol
        ));
        self.instructions.push(instruction);
        self.code_address += 4;
        self.fixups.push(Fixup {
            address: idx,
            symbol: symbol.to_string(),
            offset: 0,
        });
    }

    fn label_here(&mut self, name: &str) {
        self.labels
            .insert(name.to_string(), self.code_address + CODE_BASE);
        self.listing.push(format!(
            "0x{:04X}  {}:",
            self.code_address + CODE_BASE,
            name
        ));
    }

    fn jump(&mut self, opcode: u8, target: &str) {
        if let Some(&address) = self.labels.get(target) {
            self.emit_direct(opcode, address);
            return;
        }
        let instruction = encode(opcode, MODE_DIRECT, 0);
        let idx = self.instructions.len() as u32;
        self.listing.push(format!(
            "0x{:04X}  0x{:08X}  {} {} (fixup)",
            self.code_address + CODE_BASE,
            instruction,
            mnemonic(instruction),
            target
        ));
        self.instructions.push(instruction);
        self.code_address += 4;
        self.fixups.push(Fixup {
            address: idx,
            symbol: target.to_string(),
            offset: 0,
        });
    }

    fn collect_symbols(&mut self, expression: &Expression) {
        match expression {
            Expression::Block(items) => {
                for statement in items {
                    self.collect_symbols(statement);
                }
            }
            Expression::VarDecl {
                typ,
                name,
                init,
                size,
            } => {
                let var_size = size
                    .as_ref()
                    .and_then(|s| {
                        if let Expression::Atom(Value::Integer(n)) = s.as_ref() {
                            Some(*n as u32)
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| self.type_size(typ));
                if !self.variables.contains_key(name) {
                    self.variables.insert(
                        name.clone(),
                        Var {
                            addr: 0,
                            typ: typ.clone(),
                            size: var_size,
                        },
                    );
                    self.bss_entries.push((name.clone(), var_size));
                }
                if let Some(x) = init {
                    self.collect_symbols(x);
                }
                if let Some(x) = size {
                    self.collect_symbols(x);
                }
            }
            Expression::FuncDecl { args, body, .. } => {
                for (arg_typ, arg_name) in args {
                    if !self.variables.contains_key(arg_name) {
                        self.variables.insert(
                            arg_name.clone(),
                            Var {
                                addr: 0,
                                typ: arg_typ.clone(),
                                size: 1,
                            },
                        );
                        self.bss_entries.push((arg_name.clone(), 1));
                    }
                }
                self.collect_symbols(body);
            }
            Expression::Cout(items) | Expression::Cin(items) => {
                for x in items {
                    self.collect_symbols(x);
                }
            }
            Expression::Return(x) => {
                if let Some(x) = x {
                    self.collect_symbols(x);
                }
            }
            Expression::If {
                cond,
                then_br,
                else_br,
            } => {
                self.collect_symbols(cond);
                self.collect_symbols(then_br);
                if let Some(e) = else_br {
                    self.collect_symbols(e);
                }
            }
            Expression::While { cond, body } => {
                self.collect_symbols(cond);
                self.collect_symbols(body);
            }
            Expression::For {
                init,
                cond,
                step,
                body,
            } => {
                self.collect_symbols(init);
                self.collect_symbols(cond);
                self.collect_symbols(step);
                self.collect_symbols(body);
            }
            Expression::Operation(_, items) => {
                for a in items {
                    self.collect_symbols(a);
                }
            }
            Expression::FuncCall { caller, args } => {
                self.collect_symbols(caller);
                for a in args {
                    self.collect_symbols(a);
                }
            }
            Expression::ArrayIndex { arr, index } => {
                self.collect_symbols(arr);
                self.collect_symbols(index);
            }
            Expression::ArrayInit(items) => {
                for x in items {
                    self.collect_symbols(x);
                }
            }
            Expression::Absolute(e) => {
                self.collect_symbols(e);
            }
            Expression::Atom(val) => {
                if let Value::String(s) = val
                    && !self.string_offsets.contains_key(s)
                {
                    let offset = self.data_address;
                    for c in s.chars() {
                        self.data_values.push(c as u32);
                        self.data_address += 4;
                    }
                    self.data_values.push(0);
                    self.data_address += 4;
                    self.string_offsets.insert(s.clone(), offset);
                }
            }
        }
    }

    fn compile_statement(&mut self, expression: &Expression) {
        match expression {
            Expression::Block(items) => {
                for statement in items {
                    self.compile_statement(statement);
                }
            }
            Expression::VarDecl { name, init, .. } => {
                if !self.variables.contains_key(name) {
                    self.variables.insert(
                        name.clone(),
                        Var {
                            addr: 0,
                            typ: "i32".into(),
                            size: 1,
                        },
                    );
                    self.bss_entries.push((name.clone(), 1));
                }
                if let Some(x) = init {
                    self.compile_expression(x);
                    self.emit_variable_ref(OP_ST, name, 0);
                }
            }
            Expression::FuncDecl {
                name, args, body, ..
            } => {
                for (arg_typ, arg_name) in args {
                    if !self.variables.contains_key(arg_name) {
                        self.variables.insert(
                            arg_name.clone(),
                            Var {
                                addr: 0,
                                typ: arg_typ.clone(),
                                size: 1,
                            },
                        );
                        self.bss_entries.push((arg_name.clone(), 1));
                    }
                }
                self.label_here(&format!("fn_{}", name));
                self.in_main_function = name == "main";
                if name != "main" && !args.is_empty() {
                    let ret_temp = self.new_temp();
                    self.emit_register(OP_POP);
                    self.emit_variable_ref(OP_ST, &ret_temp, 0);
                    for (_, arg_name) in args.iter() {
                        self.emit_register(OP_POP);
                        self.emit_variable_ref(OP_ST, arg_name, 0);
                    }
                    self.emit_variable_ref(OP_LD, &ret_temp, 0);
                    self.emit_register(OP_PUSH);
                }
                self.compile_statement(body);
                self.in_main_function = false;
                if name != "main" {
                    self.emit_register(OP_RET);
                }
            }
            Expression::If {
                cond,
                then_br,
                else_br,
            } => {
                let true_label = self.new_label();
                let else_label = self.new_label();
                let end_label = self.new_label();
                let false_label = if else_br.is_some() {
                    &else_label
                } else {
                    &end_label
                };
                self.compile_condition(cond, &true_label, false_label);
                self.label_here(&true_label);
                self.compile_statement(then_br);
                self.jump(OP_JMP, &end_label);
                if let Some(else_branch) = else_br {
                    self.label_here(&else_label);
                    self.compile_statement(else_branch);
                }
                self.label_here(&end_label);
            }
            Expression::While { cond, body } => {
                let loop_label = self.new_label();
                let body_label = self.new_label();
                let end_label = self.new_label();
                self.label_here(&loop_label);
                self.compile_condition(cond, &body_label, &end_label);
                self.label_here(&body_label);
                self.compile_statement(body);
                self.jump(OP_JMP, &loop_label);
                self.label_here(&end_label);
            }
            Expression::For {
                init,
                cond,
                step,
                body,
            } => {
                self.compile_statement(init);
                let loop_label = self.new_label();
                let body_label = self.new_label();
                let end_label = self.new_label();
                self.label_here(&loop_label);
                self.compile_condition(cond, &body_label, &end_label);
                self.label_here(&body_label);
                self.compile_statement(body);
                self.compile_statement(step);
                self.jump(OP_JMP, &loop_label);
                self.label_here(&end_label);
            }
            Expression::Cout(items) => {
                for x in items {
                    self.compile_expression(x);
                    self.emit_direct(OP_ST, OUT_PORT);
                }
            }
            Expression::Cin(items) => {
                for x in items {
                    if let Expression::Atom(Value::Variable(var_name)) = x {
                        self.emit_direct(OP_LD, IN_PORT);
                        self.emit_variable_ref(OP_ST, var_name, 0);
                    }
                }
            }
            Expression::Return(Some(x)) => {
                self.compile_expression(x);
                if !self.in_main_function {
                    self.emit_register(OP_RET);
                }
            }
            Expression::Return(None) => {
                self.emit_register(OP_CLA);
                if !self.in_main_function {
                    self.emit_register(OP_RET);
                }
            }
            Expression::Operation(Ops::Assign, items) => {
                self.compile_assign(&items[0], &items[1]);
            }
            Expression::Operation(Ops::Inc, items) => {
                self.compile_increment_or_decrement(&items[0], true);
            }
            Expression::Operation(Ops::Dec, items) => {
                self.compile_increment_or_decrement(&items[0], false);
            }
            _ => {
                self.compile_expression(expression);
            }
        }
    }

    fn compile_assign(&mut self, left_value: &Expression, right_value: &Expression) {
        match left_value {
            Expression::Atom(Value::Variable(var_name)) => {
                self.compile_expression(right_value);
                self.emit_variable_ref(OP_ST, var_name, 0);
            }
            Expression::ArrayIndex { arr, index } => {
                let array_name = if let Expression::Atom(Value::Variable(n)) = arr.as_ref() {
                    n.clone()
                } else {
                    return;
                };
                let temp_var = self.new_temp();
                let var_info = self.variables.get(&array_name);
                let is_ptr = var_info.is_some_and(|v| v.typ == "ptr" && v.size == 1);
                if is_ptr {
                    self.emit_variable_ref(OP_LD, &array_name, 0);
                } else {
                    self.emit_immediate_ref(OP_LD, &array_name);
                }
                self.emit_variable_ref(OP_ST, &temp_var, 0);
                self.compile_expression(index);
                self.emit_immediate(OP_MUL, 4);
                self.emit_variable_ref(OP_ADD, &temp_var, 0);
                self.emit_variable_ref(OP_ST, &temp_var, 0);
                self.compile_expression(right_value);
                self.emit_auto_inc_ref(OP_ST, &temp_var);
            }
            _ => {}
        }
    }

    fn compile_increment_or_decrement(&mut self, left_value: &Expression, increment: bool) {
        let var_name = if let Expression::Atom(Value::Variable(x)) = left_value {
            x.clone()
        } else {
            return;
        };
        self.emit_variable_ref(OP_LD, &var_name, 0);
        if increment {
            self.emit_register(OP_INC);
        } else {
            self.emit_register(OP_DEC);
        }
        self.emit_variable_ref(OP_ST, &var_name, 0);
    }

    fn compile_expression(&mut self, expression: &Expression) {
        match expression {
            Expression::Atom(Value::Integer(n)) => self.emit_immediate(OP_LD, *n),
            Expression::Atom(Value::Char(c)) => self.emit_immediate(OP_LD, *c as i32),
            Expression::Atom(Value::Boolean(b)) => {
                self.emit_immediate(OP_LD, if *b { 1 } else { 0 })
            }
            Expression::Atom(Value::Variable(name)) => {
                let is_array = self.variables.get(name).is_some_and(|v| v.size > 1);
                if is_array {
                    self.emit_immediate_ref(OP_LD, name);
                } else {
                    self.emit_variable_ref(OP_LD, name, 0);
                }
            }
            Expression::Atom(Value::String(s)) => {
                let offset = self.string_offsets.get(s).copied().unwrap_or(0);
                let sym = format!("@str_{}", offset);
                let instruction = encode(OP_LD, 1, 0);
                let idx = self.instructions.len() as u32;
                self.listing.push(format!(
                    "0x{:04X}  0x{:08X}  {} {} (fixup)",
                    self.code_address + CODE_BASE,
                    instruction,
                    mnemonic(instruction),
                    sym
                ));
                self.instructions.push(instruction);
                self.code_address += 4;
                self.fixups.push(Fixup {
                    address: idx,
                    symbol: sym,
                    offset: 0,
                });
            }
            Expression::Operation(Ops::Neg, items) => {
                self.compile_expression(&items[0]);
                self.emit_register(OP_INV);
            }
            Expression::Operation(Ops::Not, items) => {
                self.compile_expression(&items[0]);
                self.emit_immediate(OP_CMP, 0);
                let true_label = self.new_label();
                let false_label = self.new_label();
                let end_label = self.new_label();
                self.jump(OP_JEQ, &true_label);
                self.jump(OP_JMP, &false_label);
                self.label_here(&true_label);
                self.emit_immediate(OP_LD, 1);
                self.jump(OP_JMP, &end_label);
                self.label_here(&false_label);
                self.emit_immediate(OP_LD, 0);
                self.label_here(&end_label);
            }
            Expression::Operation(op, items)
                if matches!(
                    op,
                    Ops::Add
                        | Ops::Sub
                        | Ops::Mul
                        | Ops::Div
                        | Ops::Rem
                        | Ops::And
                        | Ops::BitAnd
                        | Ops::Or
                        | Ops::BitOr
                        | Ops::Xor
                ) =>
            {
                self.compile_expression(&items[0]);
                if items.len() > 1 {
                    self.compile_second_operand(&items[1], Self::binary_opcode(op.clone()));
                }
            }
            Expression::Operation(op, items)
                if matches!(
                    op,
                    Ops::Eq | Ops::NotEq | Ops::Less | Ops::LessEq | Ops::Greater | Ops::GreaterEq
                ) =>
            {
                let true_label = self.new_label();
                let false_label = self.new_label();
                let end_label = self.new_label();
                self.compile_comparison(op.clone(), items, &true_label, &false_label);
                self.label_here(&true_label);
                self.emit_immediate(OP_LD, 1);
                self.jump(OP_JMP, &end_label);
                self.label_here(&false_label);
                self.emit_immediate(OP_LD, 0);
                self.label_here(&end_label);
            }
            Expression::Absolute(x) => {
                self.compile_expression(x);
                self.emit_register(OP_ABS);
            }
            Expression::ArrayIndex { arr, index } => {
                let array_name = if let Expression::Atom(Value::Variable(n)) = arr.as_ref() {
                    n.clone()
                } else {
                    return;
                };
                let temp_var = self.new_temp();
                let var_info = self.variables.get(&array_name);
                let is_ptr = var_info.is_some_and(|v| v.typ == "ptr" && v.size == 1);
                if is_ptr {
                    self.emit_variable_ref(OP_LD, &array_name, 0);
                } else {
                    self.emit_immediate_ref(OP_LD, &array_name);
                }
                self.emit_variable_ref(OP_ST, &temp_var, 0);
                self.compile_expression(index);
                self.emit_immediate(OP_MUL, 4);
                self.emit_variable_ref(OP_ADD, &temp_var, 0);
                self.emit_variable_ref(OP_ST, &temp_var, 0);
                self.emit_auto_inc_ref(OP_LD, &temp_var);
            }
            Expression::FuncCall { caller, args } => {
                let function_name = if let Expression::Atom(Value::Variable(s)) = caller.as_ref() {
                    s.clone()
                } else {
                    return;
                };
                for arg in args.iter().rev() {
                    self.compile_expression(arg);
                    self.emit_register(OP_PUSH);
                }
                self.jump(OP_CALL, &format!("fn_{}", function_name));
            }
            Expression::ArrayInit(items) => {
                let base_address = self.data_address;
                for item in items {
                    match item {
                        Expression::Atom(Value::Integer(n)) => {
                            self.data_values.push(*n as u32);
                            self.data_address += 4;
                        }
                        Expression::Atom(Value::Char(c)) => {
                            self.data_values.push(*c as u32);
                            self.data_address += 4;
                        }
                        Expression::ArrayInit(inner_items) => {
                            for inner_item in inner_items {
                                if let Expression::Atom(Value::Integer(n)) = inner_item {
                                    self.data_values.push(*n as u32);
                                    self.data_address += 4;
                                }
                            }
                        }
                        _ => {
                            self.data_values.push(0);
                            self.data_address += 4;
                        }
                    }
                }
                self.emit_immediate(OP_LD, base_address as i32);
            }
            _ => {
                self.compile_statement(expression);
            }
        }
    }

    fn compile_condition(&mut self, expression: &Expression, true_label: &str, false_label: &str) {
        match expression {
            Expression::Atom(Value::Boolean(b)) => {
                self.jump(OP_JMP, if *b { true_label } else { false_label });
            }
            Expression::Operation(op, items)
                if matches!(
                    op,
                    Ops::Eq | Ops::NotEq | Ops::Less | Ops::LessEq | Ops::Greater | Ops::GreaterEq
                ) =>
            {
                self.compile_comparison(op.clone(), items, true_label, false_label);
            }
            Expression::Operation(Ops::And, items) => {
                let next_label = self.new_label();
                self.compile_condition(&items[0], &next_label, false_label);
                self.label_here(&next_label);
                self.compile_condition(&items[1], true_label, false_label);
            }
            Expression::Operation(Ops::Or, items) => {
                let next_label = self.new_label();
                self.compile_condition(&items[0], true_label, &next_label);
                self.label_here(&next_label);
                self.compile_condition(&items[1], true_label, false_label);
            }
            Expression::Operation(Ops::Not, items) => {
                self.compile_condition(&items[0], false_label, true_label);
            }
            _ => {
                self.compile_expression(expression);
                self.emit_immediate(OP_CMP, 0);
                self.jump(OP_JNE, true_label);
                self.jump(OP_JMP, false_label);
            }
        }
    }

    fn compile_comparison(
        &mut self,
        op: Ops,
        items: &[Expression],
        true_label: &str,
        false_label: &str,
    ) {
        let (jump_opcode, swap) = match op {
            Ops::Eq => (OP_JEQ, false),
            Ops::NotEq => (OP_JNE, false),
            Ops::Less => (OP_JLT, false),
            Ops::LessEq => (OP_JGE, false),
            Ops::Greater => (OP_JLT, true),
            Ops::GreaterEq => (OP_JGE, true),
            _ => return,
        };
        if swap {
            self.compile_expression(&items[1]);
            self.compile_second_operand(&items[0], OP_CMP);
        } else {
            self.compile_expression(&items[0]);
            self.compile_second_operand(&items[1], OP_CMP);
        }
        self.jump(jump_opcode, true_label);
        self.jump(OP_JMP, false_label);
    }

    fn compile_second_operand(&mut self, expression: &Expression, opcode: u8) {
        match expression {
            Expression::Atom(Value::Integer(n)) => self.emit_immediate(opcode, *n),
            Expression::Atom(Value::Char(c)) => self.emit_immediate(opcode, *c as i32),
            Expression::Atom(Value::Variable(name)) => {
                let is_array = self.variables.get(name).is_some_and(|v| v.size > 1);
                if is_array {
                    self.emit_immediate_ref(opcode, name);
                } else {
                    self.emit_variable_ref(opcode, name, 0);
                }
            }
            _ => {
                let temp_var = self.new_temp();
                self.emit_variable_ref(OP_ST, &temp_var, 0);
                self.compile_expression(expression);
                self.emit_variable_ref(opcode, &temp_var, 0);
            }
        }
    }

    fn binary_opcode(op: Ops) -> u8 {
        match op {
            Ops::Add => OP_ADD,
            Ops::Sub => OP_SUB,
            Ops::Mul => OP_MUL,
            Ops::Div => OP_DIV,
            Ops::Rem => OP_MOD,
            Ops::And | Ops::BitAnd => OP_AND,
            Ops::Or | Ops::BitOr => OP_OR,
            Ops::Xor => OP_XOR,
            _ => OP_ADD,
        }
    }

    pub fn generate(mut self, program: &Expression) -> (Vec<u32>, Vec<String>) {
        self.collect_symbols(program);
        self.data_address = 0;
        self.data_values.clear();
        self.string_offsets.clear();
        self.collect_symbols(program);

        self.jump(OP_JMP, "fn_main");
        self.compile_statement(program);
        self.emit_register(OP_HLT);

        let code_end_words = self.instructions.len() as u32;
        let code_end_bytes = code_end_words * 4;
        let data_len_words = self.data_values.len() as u32;
        let data_len_bytes = data_len_words * 4;

        let mut bss_address = code_end_bytes + data_len_bytes + CODE_BASE;
        for (name, size) in &self.bss_entries {
            if let Some(v) = self.variables.get_mut(name) {
                v.addr = bss_address;
            }
            bss_address += size * 4;
        }

        for fixup in &self.fixups {
            let base = if let Some(&label_address) = self.labels.get(&fixup.symbol) {
                label_address
            } else if let Some(variable_info) = self.variables.get(&fixup.symbol) {
                variable_info.addr
            } else if fixup.symbol.starts_with("@str_") {
                let offset: u32 = fixup.symbol[5..].parse().unwrap_or(0);
                code_end_bytes + CODE_BASE + offset
            } else {
                continue;
            };
            let target = base + fixup.offset;

            let instruction = self.instructions[fixup.address as usize];
            let opcode = (instruction >> 24) as u8;
            let mode = (instruction >> 21) as u8;
            let new_instruction = encode(opcode, mode & 7, target);
            self.instructions[fixup.address as usize] = new_instruction;

            let byte_addr = fixup.address * 4;
            let address_hex = format!("0x{:04X}", byte_addr + CODE_BASE);
            for line in &mut self.listing {
                if line.starts_with(&address_hex) && line.contains("fixup") {
                    *line = format!(
                        "0x{:04X}  0x{:08X}  {}",
                        byte_addr + CODE_BASE,
                        new_instruction,
                        mnemonic(new_instruction)
                    );
                    break;
                }
            }
        }

        let bss_total_words = (bss_address - code_end_bytes - data_len_bytes - CODE_BASE) / 4;
        let mut memory = Vec::new();
        memory.extend_from_slice(&self.instructions);
        memory.extend_from_slice(&self.data_values);
        memory.resize(
            (code_end_words + data_len_words + bss_total_words) as usize,
            0,
        );
        (memory, self.listing)
    }
}
