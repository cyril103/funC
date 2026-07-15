use std::{fs, path::{Path, PathBuf}, process};

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

    /// Compile l'IR LLVM textuelle vers un objet (.o)
    #[arg(long)]
    emit_obj: bool,

    /// Écrit le fichier objet dans ce chemin (active --emit-obj)
    #[arg(long)]
    out_obj: Option<String>,

    /// Lie l'objet généré pour produire un exécutable natif
    #[arg(long)]
    emit_exe: bool,

    /// Écrit l'exécutable dans ce chemin (active --emit-exe)
    #[arg(long)]
    out_exe: Option<String>,
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

    if args.emit_ir || args.emit_obj || args.emit_exe {
        let ir = codegen::generate(&parsed, &types);

        if args.emit_ir {
            if let Some(path) = &args.out {
                if let Err(err) = fs::write(path, &ir) {
                    eprintln!("Échec d'écriture IR {path}: {err}");
                    process::exit(1);
                }
                println!("IR écrite dans {path}");
            } else {
                println!("=== IR ===\n{ir}");
            }
        }

        let object_path = if args.emit_obj || args.emit_exe {
            Some(emit_object(&ir, &args))
        } else {
            None
        };

        if args.emit_obj {
            if object_path.is_none() {
                process::exit(1);
            }
        }

        if args.emit_exe {
            match object_path {
                Some(obj) => link_executable(&obj, &args),
                None => process::exit(1),
            }
        }
    }
}

fn emit_object(ir: &str, args: &CompileArgs) -> PathBuf {
    let object_path = args
        .out_obj
        .clone()
        .unwrap_or_else(|| format!("{}.o", default_stem(&args.input)));

    let ir_path = if let Some(path) = args.out.clone() {
        PathBuf::from(path)
    } else {
        let fallback =
            std::env::temp_dir().join(format!("func-{}-{}.ll", default_stem(&args.input), process::id()));
        if let Err(err) = fs::write(&fallback, ir) {
            eprintln!("Échec d'écriture IR temporaire {fallback:?}: {err}");
            process::exit(1);
        }
        fallback
    };

    if ir_path.exists() {
        let status = process::Command::new("llc")
            .arg("-filetype=obj")
            .arg("-o")
            .arg(&object_path)
            .arg(&ir_path)
            .status();

        match status {
            Ok(exit) if exit.success() => {
                println!("Objet écrit dans {object_path}");
            }
            Ok(exit) => {
                eprintln!("Échec de llc (code de sortie: {exit})");
                process::exit(1);
            }
            Err(err) => {
                eprintln!("Impossible d'exécuter llc: {err}");
                process::exit(1);
            }
        }
    } else {
        eprintln!("Le fichier LLVM IR n'existe pas: {ir_path:?}");
        process::exit(1);
    }

    PathBuf::from(object_path)
}

fn link_executable(object_path: &PathBuf, args: &CompileArgs) {
    let exe_path = args
        .out_exe
        .clone()
        .unwrap_or_else(|| {
            let stem = default_stem(&args.input);
            if cfg!(windows) {
                format!("{stem}.exe")
            } else {
                stem
            }
        });

    let linkers = ["clang", "cc"];
    for linker in &linkers {
        let status = process::Command::new(linker)
            .arg(object_path.as_os_str())
            .arg("-o")
            .arg(&exe_path)
            .status();

        match status {
            Ok(exit) if exit.success() => {
                println!("Exécutable écrit dans {exe_path}");
                return;
            }
            Ok(exit) => {
                eprintln!("Échec de {linker} (code de sortie: {exit}), tentative suivante...");
            }
            Err(err) => {
                eprintln!("{linker} indisponible: {err}");
            }
        }
    }

    eprintln!("Aucun linker disponible (clang/cc), impossible de produire un exécutable.");
    process::exit(1);
}

fn default_stem(input: &str) -> String {
    Path::new(input)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| "a".to_string())
}
