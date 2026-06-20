pub mod ast;
mod backend;
mod diagnostics;
mod lexer;
mod parser;
mod semantic;

use backend::Backend;
use chumsky::Parser;
use inkwell::context::Context;
use lexer::Lexer;
use parser::program::program_parser;
use std::fs;
use std::process::{Command, ExitCode};

use crate::{
    diagnostics::Diagnostic, parser::error::rich_to_diagnostic, semantic::SemanticAnalyzer,
};

const EXIT_LEXICAL: u8 = 1;
const EXIT_SYNTACTIC: u8 = 2;
const EXIT_SEMANTIC: u8 = 3;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let Some(file_path) = args.get(1) else {
        eprintln!("(0,0) SYNTACTIC: missing input file argument, usage: hulk <file.hulk>");
        return ExitCode::from(EXIT_SYNTACTIC);
    };

    let source = match fs::read_to_string(file_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "(0,0) SYNTACTIC: could not read file '{}': {}",
                file_path, e
            );
            return ExitCode::from(EXIT_SYNTACTIC);
        }
    };

    let tokens = match Lexer::new(&source).tokenize() {
        Ok(v) => v,
        Err(errors) => {
            for error in errors {
                print_contract_diagnostic(&error.into(), "LEXICAL", &source);
            }
            return ExitCode::from(EXIT_LEXICAL);
        }
    };

    let program = match program_parser().parse(&tokens.as_slice()).into_result() {
        Ok(ast) => ast,
        Err(errors) => {
            for error in errors {
                let diagnostic = rich_to_diagnostic(error);
                print_contract_diagnostic(&diagnostic, "SYNTACTIC", &source);
            }
            return ExitCode::from(EXIT_SYNTACTIC);
        }
    };

    let mut analyzer = SemanticAnalyzer::new();
    let hir = match analyzer.analyze_program(program) {
        Ok(h) => h,
        Err(errors) => {
            for error in errors {
                let d: Diagnostic = error.into();
                print_contract_diagnostic(&d, "SEMANTIC", &source);
            }
            return ExitCode::from(EXIT_SEMANTIC);
        }
    };

    let llvm_context = Context::create();
    let mut backend = Backend::new(&llvm_context, "hulk");
    if let Err(err) = backend.compile_program(&hir, &analyzer) {
        eprintln!("(0,0) SEMANTIC: internal backend error: {}", err);
        return ExitCode::from(EXIT_SEMANTIC);
    }

    let ir_path = "output.ll";
    if let Err(e) = backend::emit::emit_ir_to_file(&backend.module, ir_path) {
        eprintln!("(0,0) SEMANTIC: failed to emit LLVM IR: {}", e);
        return ExitCode::from(EXIT_SEMANTIC);
    }

    if let Err(e) = compile_ir_to_executable(ir_path) {
        eprintln!("(0,0) SEMANTIC: native compilation failed: {}", e);
        return ExitCode::from(EXIT_SEMANTIC);
    }

    ExitCode::SUCCESS
}

fn compile_ir_to_executable(ir_path: &str) -> Result<(), String> {
    let llc = std::env::var("HULK_LLC").unwrap_or_else(|_| "llc".to_string());
    let cc = std::env::var("HULK_CC").unwrap_or_else(|_| "cc".to_string());
    let obj_path = "output.o";
    let runtime_src = "runtime/runtime.c";
    let runtime_obj = "runtime.o";
    let exe_path = "output";
    run_command(
        &llc,
        &[
            "-filetype=obj",
            "-relocation-model=pic",
            ir_path,
            "-o",
            obj_path,
        ],
    )?;
    run_command(
        &cc,
        &[
            "-Wall",
            "-O2",
            "-ffast-math",
            "-c",
            runtime_src,
            "-o",
            runtime_obj,
        ],
    )?;
    run_command(
        &cc,
        &["-no-pie", "-o", exe_path, obj_path, runtime_obj, "-lm"],
    )?;
    Ok(())
}

fn run_command(program: &str, args: &[&str]) -> Result<(), String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("failed to spawn '{}': {}", program, e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("'{}' failed: {}", program, stderr));
    }
    Ok(())
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(source.len());
    let mut line = 1usize;
    let mut col = 1usize;
    for ch in source[..offset].chars() {
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

fn print_contract_diagnostic(diagnostic: &Diagnostic, error_type: &str, source: &str) {
    let (line, col) = if diagnostic.span.start == 0 && diagnostic.span.end == 0 {
        (0, 0)
    } else {
        offset_to_line_col(source, diagnostic.span.start)
    };
    eprintln!("({},{}) {}: {}", line, col, error_type, diagnostic.message);
}
