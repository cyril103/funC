use std::{env, fs, process};

mod ast;
mod codegen;
mod lexer;
mod parser;
mod token;
mod typecheck;

use lexer::Lexer;
use parser::Parser;

#[derive(Default)]
struct Cli {
    input: Option<String>,
    emit_ast: bool,
    emit_typed: bool,
    emit_ir: bool,
    ir_path: Option<String>,
}

fn main() {
    let mut cli = Cli::default();
    let mut args = env::args().skip(1).peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--emit-ast" => cli.emit_ast = true,
            "--emit-typed" => cli.emit_typed = true,
            "--emit-ir" => cli.emit_ir = true,
            "--out" => {
                cli.ir_path = args.next();
            }
            _ if arg.starts_with('-') => {
                eprintln!("option inconnue: {arg}");
                print_usage();
                process::exit(1);
            }
            _ => {
                if cli.input.is_none() {
                    cli.input = Some(arg);
                }
            }
        }
    }

    let input = cli.input.unwrap_or_else(|| {
        print_usage();
        process::exit(1);
    });

    let source = match fs::read_to_string(&input) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("Impossible de lire le fichier {input}: {err}");
            process::exit(1);
        }
    };

    let lexed = match Lexer::new(&source).tokenize() {
        Ok(tokens) => tokens,
        Err(err) => {
            eprintln!("Erreur lexer: {err}");
            process::exit(1);
        }
    };

    let parsed = match Parser::new(lexed).parse_program() {
        Ok(program) => program,
        Err(err) => {
            eprintln!("Erreur parser: {}:{}: {}", err.line, err.column, err.message);
            process::exit(1);
        }
    };

    if cli.emit_ast {
        println!("=== AST ===\n{}", parsed);
    }

    let types = match typecheck::check(&parsed) {
        Ok(infos) => infos,
        Err(err) => {
            eprintln!("Erreur typage: {err}");
            process::exit(1);
        }
    };

    if cli.emit_typed {
        println!("=== TYPES DÉDUITS ===");
        for (id, ty) in &types {
            println!("expr #{id}: {ty}");
        }
    }

    if cli.emit_ir {
        let ir = codegen::generate(&parsed, &types);
        if let Some(path) = cli.ir_path {
            if let Err(err) = fs::write(&path, ir) {
                eprintln!("Échec d'écriture IR {path}: {err}");
                process::exit(1);
            }
            println!("IR écrite dans {path}");
        } else {
            println!("=== IR ===\n{ir}");
        }
    }
}

fn print_usage() {
    eprintln!("Usage: funC [--emit-ast] [--emit-typed] [--emit-ir] [--out file.ll] <source.fc>");
}
