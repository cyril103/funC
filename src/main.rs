use std::{fs, process};

use clap::{Args, Parser as ClapParser, Subcommand};
use lexer::Lexer;
use parser::Parser as FuncParser;

mod ast;
mod codegen;
mod lexer;
mod parser;
mod token;
mod typecheck;

#[derive(Debug, ClapParser)]
#[command(name = "funC", version, about = "Compilateur FunC")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Compile(CompileArgs),
}

#[derive(Debug, Args)]
struct CompileArgs {
    /// Fichier source FunC à compiler
    input: String,

    /// Affiche l'AST généré
    #[arg(long)]
    emit_ast: bool,

    /// Affiche les types inférés
    #[arg(long)]
    emit_typed: bool,

    /// Affiche l'IR LLVM textuelle dans la sortie standard
    #[arg(long)]
    emit_ir: bool,

    /// Écrit l'IR LLVM textuelle dans un fichier .ll
    #[arg(short, long)]
    out: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compile(args) => run_compile(args),
    }
}

fn run_compile(args: CompileArgs) {
    let source = match fs::read_to_string(&args.input) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("Impossible de lire le fichier {}: {err}", args.input);
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

    let parsed = match FuncParser::new(lexed).parse_program() {
        Ok(program) => program,
        Err(err) => {
            eprintln!("Erreur parser: {}:{}: {}", err.line, err.column, err.message);
            process::exit(1);
        }
    };

    if args.emit_ast {
        println!("=== AST ===\n{parsed}");
    }

    let types = match typecheck::check(&parsed) {
        Ok(infos) => infos,
        Err(err) => {
            eprintln!("Erreur typage: {err}");
            process::exit(1);
        }
    };

    if args.emit_typed {
        println!("=== TYPES DÉDUITS ===");
        for (id, ty) in &types {
            println!("expr #{id}: {ty}");
        }
    }

    if args.emit_ir {
        let ir = codegen::generate(&parsed, &types);
        if let Some(path) = args.out {
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
