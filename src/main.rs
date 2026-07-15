use std::{fs, path::{Path, PathBuf}, process};

use clap::{Args, Parser as ClapParser, Subcommand};
use lexer::Lexer;
use parser::Parser as FuncParser;
use inkwell::targets::{InitializationConfig, Target, TargetMachine};

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
    /// Affiche les cibles supportées et les alias reconnus
    ListTargets,
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

    /// Compile l'IR LLVM textuelle vers un objet (.o ou .obj selon la cible)
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

    /// Spécifie la cible LLVM (ex: x86_64-pc-windows-msvc, aarch64-unknown-linux-gnu)
    #[arg(long)]
    target: Option<String>,
}

#[derive(Debug)]
struct TargetInfo {
    triple: String,
    object_ext: &'static str,
    exe_ext: &'static str,
}

impl TargetInfo {
    fn from_target_arg(target: &Option<String>) -> Result<Self, String> {
        let host = default_host_target();
        let target = resolve_target_alias(target.as_deref(), &host)?;
        let normalized = target.to_lowercase();
        let is_windows = normalized.contains("windows");
        let object_ext = if is_windows { "obj" } else { "o" };
        let exe_ext = if is_windows { "exe" } else { "" };
        Ok(Self {
            triple: target,
            object_ext,
            exe_ext,
        })
    }

    fn exe_suffix(&self) -> &'static str {
        self.exe_ext
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compile(args) => run_compile(args),
        Commands::ListTargets => list_targets(),
    }
}

fn list_targets() {
    let host = default_host_target();
    println!("Cible hôte (défaut): {host}");
    println!("Alias reconnus:");
    println!("  - x86_64  => x86_64-unknown-linux-gnu (Windows: x86_64-pc-windows-msvc)");
    println!("  - amd64   => x86_64-unknown-linux-gnu (Windows: x86_64-pc-windows-msvc)");
    println!("  - aarch64 => aarch64-unknown-linux-gnu (Windows: aarch64-pc-windows-msvc)");
    println!("  - arm64   => aarch64-unknown-linux-gnu (Windows: aarch64-pc-windows-msvc)");
    println!("  - x86     => i386-unknown-linux-gnu (Windows: i386-pc-windows-msvc)");
    println!("  - i386    => i386-unknown-linux-gnu (Windows: i386-pc-windows-msvc)");
    println!("  - native  => cible par défaut de l'hôte");
    println!("Formats de triplet LLVM acceptés: arch-vendor-os (par ex: x86_64-pc-windows-msvc, aarch64-unknown-linux-gnu)");
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

    let target_info = match TargetInfo::from_target_arg(&args.target) {
        Ok(target) => target,
        Err(err) => {
            eprintln!("Cible invalide: {err}");
            process::exit(1);
        }
    };

    if args.emit_ir || args.emit_obj || args.emit_exe {
        println!("Target: {}", target_info.triple);
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
            Some(emit_object(&ir, &args, &target_info))
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
                Some(obj) => link_executable(&obj, &args, &target_info),
                None => process::exit(1),
            }
        }
    }
}

fn emit_object(ir: &str, args: &CompileArgs, target: &TargetInfo) -> PathBuf {
    let object_path = args
        .out_obj
        .clone()
        .unwrap_or_else(|| {
            let stem = default_stem(&args.input);
            let extension = target.object_ext;
            format!("{}.{}", stem, extension)
        });

    let ir_path = if let Some(path) = args.out.clone() {
        if args.emit_ir {
            PathBuf::from(path)
        } else {
            let fallback = std::env::temp_dir().join(format!(
                "func-{}-{}.ll",
                default_stem(&args.input),
                process::id()
            ));
            if let Err(err) = fs::write(&fallback, ir) {
                eprintln!("Échec d'écriture IR temporaire {fallback:?}: {err}");
                process::exit(1);
            }
            fallback
        }
    } else {
        let fallback = std::env::temp_dir().join(format!(
            "func-{}-{}.ll",
            default_stem(&args.input),
            process::id()
        ));
        if let Err(err) = fs::write(&fallback, ir) {
            eprintln!("Échec d'écriture IR temporaire {fallback:?}: {err}");
            process::exit(1);
        }
        fallback
    };

    let mut cmd = process::Command::new("llc");
    cmd.arg("-filetype=obj");
    cmd.arg("-o");
    cmd.arg(&object_path);
    cmd.arg(format!("-mtriple={}", target.triple));
    cmd.arg(&ir_path);
    let status = cmd.status();

    match status {
        Ok(exit) if exit.success() => {
            println!("Objet écrit dans {object_path}");
        }
        Ok(exit) => {
            eprintln!("Échec de llc (code de sortie: {exit})");
            eprintln!(
                "Assurez-vous d'avoir un llvm de niveau compatible avec la cible: {}",
                target.triple
            );
            process::exit(1);
        }
        Err(err) => {
            eprintln!("Impossible d'exécuter llc: {err}");
            eprintln!("Installons LLVM/llc (via les paquets de votre système) puis relancez.");
            process::exit(1);
        }
    }

    if args.out.is_none() && !args.emit_ir {
        let _ = fs::remove_file(&ir_path);
    }

    PathBuf::from(object_path)
}

fn link_executable(object_path: &PathBuf, args: &CompileArgs, target: &TargetInfo) {
    let exe_path = args
        .out_exe
        .clone()
        .unwrap_or_else(|| {
            let stem = default_stem(&args.input);
            if target.exe_suffix().is_empty() {
                stem
            } else {
                format!("{stem}.{}", target.exe_suffix())
            }
        });

    let mut linkers = Vec::new();
    linkers.push("clang");
    linkers.push("cc");
    if cfg!(windows) {
        linkers.push("link");
    }

    for linker in linkers {
        let mut cmd = process::Command::new(linker);
            cmd.arg(object_path.as_os_str());
            if linker == "clang" {
                cmd.arg("-target");
                cmd.arg(&target.triple);
            }
        cmd.arg("-o");
        cmd.arg(&exe_path);
        let status = cmd.status();

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

    eprintln!(
        "Aucun linker disponible (clang/cc), impossible de produire un exécutable."
    );
    eprintln!(
        "Pour une cible '{}', vérifiez que la chaîne de compilation LLVM/Clang installée supporte ce triplet.",
        target.triple
    );
    process::exit(1);
}

fn default_stem(input: &str) -> String {
    Path::new(input)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| "a".to_string())
}

fn default_host_target() -> String {
    Target::initialize_all(&InitializationConfig::default());
    let triple = TargetMachine::get_default_triple().to_string();
    if let Some(inner) = triple
        .strip_prefix("TargetTriple(\"")
        .and_then(|value| value.strip_suffix("\")"))
    {
        inner.to_string()
    } else {
        triple
    }
}

fn looks_like_triple(value: &str) -> bool {
    value.split('-').count() >= 2
}

fn resolve_target_alias(raw: Option<&str>, host: &str) -> Result<String, String> {
    match raw {
        None | Some("") => Ok(host.to_string()),
        Some("native") => Ok(host.to_string()),
        Some(value) => {
            let value = value.trim();
            if value.contains('-') {
                if looks_like_triple(value) {
                    Ok(value.to_string())
                } else {
                    Err(format!(
                        "cible '{value}' invalide: le format attendu ressemble à un triplet (arch-vendor-système)"
                    ))
                }
            } else {
                let alias = match value {
                    "x86_64" | "amd64" => {
                        if host.contains("windows") {
                            "x86_64-pc-windows-msvc"
                        } else {
                            "x86_64-unknown-linux-gnu"
                        }
                    }
                    "aarch64" | "arm64" => {
                        if host.contains("windows") {
                            "aarch64-pc-windows-msvc"
                        } else {
                            "aarch64-unknown-linux-gnu"
                        }
                    }
                    "x86" | "i386" => {
                        if host.contains("windows") {
                            "i386-pc-windows-msvc"
                        } else {
                            "i386-unknown-linux-gnu"
                        }
                    }
                    other => {
                        return Err(format!(
                            "cible '{other}' non reconnue. Utiliser un triplet (ex: x86_64-pc-windows-msvc, aarch64-unknown-linux-gnu) ou 'native'."
                        ));
                    }
                };
                Ok(alias.to_string())
            }
        }
    }
}
