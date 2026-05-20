#![allow(clippy::upper_case_acronyms, non_snake_case)]

mod consts;
mod cpu;
mod mem;
mod microcode;

use std::env;
use std::fs;

fn parse_input(input: &str) -> Vec<i32> {
    let input = input.trim();
    if input.is_empty() {
        return Vec::new();
    }

    let inner = input
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(input);

    let mut result = Vec::new();
    let mut chars = inner.chars().peekable();

    while chars.peek().is_some() {
        while chars
            .peek()
            .is_some_and(|c| c.is_ascii_whitespace() || *c == ',')
        {
            chars.next();
        }

        if chars.peek().is_none() {
            break;
        }

        if chars.peek() == Some(&'\'') {
            chars.next();
            let c = chars.next().unwrap_or('\0');
            if chars.next() == Some('\'') {
                result.push(c as i32);
            }
        } else {
            let mut num_str = String::new();
            if chars.peek() == Some(&'-') {
                num_str.push(chars.next().unwrap());
            }
            while chars.peek().is_some_and(|c| c.is_ascii_digit()) {
                num_str.push(chars.next().unwrap());
            }
            if let Ok(n) = num_str.parse::<i32>() {
                result.push(n);
            }
        }
    }

    result
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <binary> [input_file] [output_file]", args[0]);
        std::process::exit(1);
    }

    let binary_path = &args[1];
    let input_path = args.get(2);
    let output_path = args.get(3);

    let binary_data = fs::read(binary_path).expect("Failed to read binary file");

    let words: Vec<i32> = binary_data
        .chunks_exact(4)
        .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();

    let mut cpu = cpu::CPU::new();
    cpu.load_program(8, &words);

    if let Some(input_path) = input_path {
        let input_data = fs::read_to_string(input_path).expect("Failed to read input file");
        for val in parse_input(&input_data) {
            cpu.add_input(val);
        }
    }

    cpu.run();

    let output = cpu.get_output();

    let output_str: String = output
        .iter()
        .flat_map(|&val| {
            let ch = std::char::from_u32(val as u32).unwrap_or('\0');
            let repr = if ch.is_ascii_graphic() || ch == ' ' {
                ch.to_string()
            } else if ch.is_whitespace() {
                "<space/\\n/\\t>".into()
            } else if ch != '\0' {
                format!("<U+{:04X}>", val as u32)
            } else {
                "<invalid>".into()
            };
            vec![format!("{}", val), format!("0x{:08X}", val as u32), repr]
        })
        .collect::<Vec<_>>()
        .join("\n");

    if let Some(output_path) = output_path {
        fs::write(output_path, &output_str).expect("Failed to write output file");
    } else {
        println!("{}", output_str);
    }
}
