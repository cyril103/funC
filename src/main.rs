use std::{fs, path::{Path, PathBuf}, process};
use std::collections::HashSet;
use crate::ast::Program;

use clap::{Args, Parser as ClapParser, Subcommand};
use lexer::Lexer;
use parser::Parser as FuncParser;
use inkwell::targets::{InitializationConfig, Target, TargetMachine};

mod ast;
mod constfold;
mod codegen;
mod memorycheck;
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
    Fmt(FmtArgs),
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

    /// Analyse seulement: parse + typecheck sans génération d'IR/code
    #[arg(long)]
    check: bool,

    /// Avertit sur les allocations heap potentiellement non libérées (heuristique)
    #[arg(long)]
    warn_memory: bool,

    /// Génère les métadonnées de debug dans la sortie llc (DWARF)
    #[arg(long)]
    debug_info: bool,

    /// Affiche l'IR LLVM textuelle dans la sortie standard
    #[arg(long)]
    emit_ir: bool,

    /// Génère l'assembleur natif via llc (-filetype=asm)
    #[arg(long)]
    emit_asm: bool,

    /// Écrit l'IR LLVM textuelle dans un fichier .ll
    #[arg(short, long)]
    out: Option<String>,

    /// Écrit l'assembleur dans un fichier .s (active --emit-asm)
    #[arg(long)]
    out_asm: Option<String>,

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

#[derive(Debug, Args)]
struct FmtArgs {
    /// Fichier source FunC à formater
    input: String,

    /// Fichier de sortie (défaut : réécrire le fichier d'entrée)
    #[arg(short, long)]
    out: Option<String>,

    /// Vérifie seulement (exit code 1 si des changements seraient produits)
    #[arg(short, long)]
    check: bool,
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
        Commands::Fmt(args) => run_fmt(args),
        Commands::ListTargets => list_targets(),
    }
}

fn run_fmt(args: FmtArgs) {
    let source = match fs::read_to_string(&args.input) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("Impossible de lire le fichier {}: {err}", args.input);
            process::exit(1);
        }
    };

    let tokens = match Lexer::new(&source).tokenize() {
        Ok(tokens) => tokens,
        Err(err) => {
            eprintln!("Erreur lexer: {err}");
            process::exit(1);
        }
    };

    let parsed = match FuncParser::new(tokens).parse_program() {
        Ok(program) => program,
        Err(err) => {
            print_diagnostic(
                &source,
                "Erreur parser",
                err.line,
                err.column,
                &err.message,
                Some("Réparez la syntaxe pour pouvoir lancer le formateur."),
            );
            process::exit(1);
        }
    };

    let formatted = format!("{}\n", parsed);
    let normalized_source = source.replace('\r', "");
    if args.check {
        if normalized_source == formatted {
            return;
        }
        eprintln!("Le fichier {} n'est pas formaté", args.input);
        process::exit(1);
    }

    let output = args.out.unwrap_or(args.input);
    if let Err(err) = fs::write(&output, formatted) {
        eprintln!("Impossible d'écrire le fichier formaté {}: {err}", output);
        process::exit(1);
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

    let mut parsed = match load_program_from_entry(&args.input) {
        Ok(program) => program,
        Err(err) => {
            eprintln!("{err}");
            process::exit(1);
        }
    };
    constfold::fold_program(&mut parsed);

    if args.emit_ast {
        println!("=== AST ===\n{parsed}");
    }

    let types = match typecheck::check(&parsed, &source) {
        Ok(infos) => infos,
        Err(err) => {
            print_diagnostic(
                &source,
                "Erreur typage",
                err.line,
                err.column,
                &err.message,
                err.suggestion.as_deref(),
            );
            process::exit(1);
        }
    };

    if args.warn_memory {
        let warnings = memorycheck::analyze(&parsed);
        if warnings.is_empty() {
            println!("Avertissements mémoire: aucun");
        } else {
            println!("Avertissements mémoire (heuristique, non bloquants):");
            for warning in warnings {
                print_diagnostic(
                    &source,
                    "Avertissement mémoire",
                    warning.line,
                    warning.column,
                    &warning.message,
                    None,
                );
            }
        }
    }

    if args.emit_typed {
        println!("=== TYPES DÉDUITS ===");
        for (id, ty) in &types {
            println!("expr #{id}: {ty}");
        }
    }

    if args.check {
        if args.emit_ir || args.emit_asm || args.emit_obj || args.emit_exe {
            eprintln!("`--check` ignore les options de génération backend: aucun IR/objet/exécutable/asm produit.");
        }
        println!("Compilation check OK: {}", args.input);
        return;
    }

    let target_info = match TargetInfo::from_target_arg(&args.target) {
        Ok(target) => target,
        Err(err) => {
            eprintln!("Cible invalide: {err}");
            process::exit(1);
        }
    };

    if args.emit_ir || args.emit_asm || args.emit_obj || args.emit_exe {
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

        let asm_path = if args.emit_asm {
            Some(emit_asm(&ir, &args, &target_info))
        } else {
            None
        };

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

fn emit_asm(ir: &str, args: &CompileArgs, target: &TargetInfo) -> PathBuf {
    let asm_path = args
        .out_asm
        .clone()
        .unwrap_or_else(|| format!("{}.s", default_stem(&args.input)));

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
    cmd.arg("-filetype=asm");
    if args.debug_info {
        cmd.arg("-g");
    }
    cmd.arg("-o");
    cmd.arg(&asm_path);
    cmd.arg(format!("-mtriple={}", target.triple));
    cmd.arg(&ir_path);
    let status = cmd.status();

    match status {
        Ok(exit) if exit.success() => {
            println!("Assembleur écrit dans {asm_path}");
            PathBuf::from(asm_path)
        }
        Ok(exit) => {
            eprintln!("Échec de llc en mode assembleur (code de sortie: {exit})");
            eprintln!(
                "Assurez-vous d'avoir un llvm de niveau compatible avec la cible: {}",
                target.triple
            );
            process::exit(1);
        }
        Err(err) => {
            eprintln!("Impossible d'exécuter llc: {err}");
            eprintln!("Installez LLVM/llc puis relancez.");
            process::exit(1);
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
    if args.debug_info {
        cmd.arg("-g");
    }
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

fn load_program_from_entry(entry: &str) -> Result<Program, String> {
    let mut visited = HashSet::new();
    let mut functions = Vec::new();
    let entry_path = Path::new(entry);
    load_module_program(entry_path, &mut visited, &mut functions)?;
    Ok(Program {
        functions,
        structs: Vec::new(),
        enums: Vec::new(),
        imports: Vec::new(),
    })
}

fn load_module_program(
    path: &Path,
    visited: &mut HashSet<PathBuf>,
    functions: &mut Vec<crate::ast::Function>,
) -> Result<(), String> {
    let canonical = path.canonicalize().map_err(|err| {
        format!(
            "Impossible de localiser le module '{}': {err}",
            path.to_string_lossy()
        )
    })?;

    if visited.contains(&canonical) {
        return Ok(());
    }
    visited.insert(canonical.clone());

    let source = fs::read_to_string(&canonical).map_err(|err| {
        format!("Impossible de lire le module '{}': {err}", canonical.display())
    })?;

    let lexed = Lexer::new(&source)
        .tokenize()
        .map_err(|err| format!(
            "{}:{}:{}: Erreur lexer: {}",
            canonical.display(),
            err.line,
            err.column,
            err.message
        ))?;

    let parsed = FuncParser::new(lexed)
        .parse_program()
        .map_err(|err| format!(
            "{}:{}:{}: Erreur parser: {}",
            canonical.display(),
            err.line,
            err.column,
            err.message
        ))?;

    let base_dir = canonical.parent().unwrap_or(Path::new("."));
    for import in &parsed.imports {
        let import_path = resolve_import_path(base_dir, import);
        load_module_program(&import_path, visited, functions)?;
    }

    functions.extend(parsed.functions);
    Ok(())
}

fn resolve_import_path(base_dir: &Path, raw_import: &str) -> PathBuf {
    let normalized = if raw_import.ends_with(".fc") {
        raw_import.to_string()
    } else {
        format!("{raw_import}.fc")
    };

    let import_path = Path::new(&normalized);
    if import_path.is_absolute() {
        import_path.to_path_buf()
    } else {
        base_dir.join(import_path)
    }
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
    let exe_path = normalize_exe_path(exe_path, target);

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

fn normalize_exe_path(out_exe: String, target: &TargetInfo) -> PathBuf {
    let mut exe_path = PathBuf::from(out_exe);

    if !target.exe_suffix().is_empty() {
        let has_extension = exe_path.extension().is_some();
        if !has_extension {
            exe_path.set_extension(target.exe_suffix());
        }
    }

    exe_path
}

fn print_diagnostic(
    source: &str,
    kind: &str,
    line: usize,
    column: usize,
    message: &str,
    suggestion: Option<&str>,
) {
    eprintln!("{kind} en {line}:{column}: {message}");

    if line == 0 || column == 0 {
        return;
    }

    let lines: Vec<&str> = source.lines().collect();
    if let Some(context) = lines.get(line.saturating_sub(1)) {
        let gutter = line.to_string().len().max(2);
        let pointer_offset = column.saturating_sub(1);
        eprintln!("{:>width$} | {}", line, context, width = gutter);
        eprintln!(
            "{:>width$} | {:>pointer$}^",
            "",
            "",
            width = gutter,
            pointer = pointer_offset + 1
        );
    }
    if let Some(suggestion) = suggestion {
        eprintln!("  suggestion: {suggestion}");
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
