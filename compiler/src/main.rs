mod codegen;
mod lexer;
mod parser;

use codegen::Codegen;
use lexer::Lexer;
use parser::{Parser, optimize_tree};
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "Usage: {} <input.algo> <output.bin> [--listing listing.txt] [--ast ast.txt]",
            args[0]
        );
        process::exit(1);
    }
    let input_path = &args[1];
    let output_path = &args[2];
    let mut listing_path = None;
    let mut ast_path = None;
    let mut i = 3;
    while i < args.len() {
        if args[i] == "--listing" {
            listing_path = args.get(i + 1).cloned();
            i += 2;
        } else if args[i] == "--ast" {
            ast_path = args.get(i + 1).cloned();
            i += 2;
        } else {
            i += 1;
        }
    }

    let source = match fs::read_to_string(input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading input file '{}': {}", input_path, e);
            process::exit(1);
        }
    };

    let mut lexer = Lexer::new(&source);
    let tokens = match lexer.parse() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Lexical error: {}", e);
            process::exit(1);
        }
    };

    let mut parser = Parser::new(tokens);
    let ast = match parser.parse_program() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Parse error: {:?}", e);
            process::exit(1);
        }
    };

    if let Some(ap) = ast_path {
        let ast_text = ast.to_string();
        if let Err(e) = fs::write(&ap, &ast_text) {
            eprintln!("Error writing AST '{}': {}", ap, e);
        } else {
            println!("Wrote AST to {}", ap);
        }
    }

    let optimized = optimize_tree(ast);

    let codegen = Codegen::new();
    let (binary, listing) = codegen.generate(&optimized);

    let bytes: Vec<u8> = binary.iter().flat_map(|w| w.to_le_bytes()).collect();

    if let Err(e) = fs::write(output_path, &bytes) {
        eprintln!("Error writing output '{}': {}", output_path, e);
        process::exit(1);
    }
    println!("Wrote {} bytes to {}", bytes.len(), output_path);

    if let Some(lp) = listing_path {
        let listing_text = listing.join("\n");
        if let Err(e) = fs::write(&lp, &listing_text) {
            eprintln!("Error writing listing '{}': {}", lp, e);
        } else {
            println!("Wrote listing to {}", lp);
        }
    }
}
