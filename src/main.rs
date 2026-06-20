pub mod ast;
mod backend;
mod diagnostics;
mod lexer;
mod parser;
mod semantic;
use backend::Backend;
use chumsky::Parser;
use diagnostics::print_diagnostic;
use inkwell::context::Context;
use lexer::Lexer;
use parser::program::program_parser;
use std::fs;

use crate::{
    diagnostics::Diagnostic, parser::error::rich_to_diagnostic, semantic::SemanticAnalyzer,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_name = "test";

    let source = read_file(&format!("tests/{}.hulk", file_name));

    let tokens = match Lexer::new(&source).tokenize() {
        Ok(v) => v,
        Err(errors) => {
            for error in errors {
                print_diagnostic(&error.into(), &file_name, &source);
            }
            return Err("lexical analysis failed".into());
        }
    };

    let program = match program_parser().parse(&tokens.as_slice()).into_result() {
        Ok(ast) => ast,
        Err(errors) => {
            for error in errors {
                let diagnostic = rich_to_diagnostic(error);
                print_diagnostic(&diagnostic, &file_name, &source);
            }
            return Err("syntax analysis failed".into());
        }
    };

    let mut analyzer = SemanticAnalyzer::new();
    let hir = match analyzer.analyze_program(program) {
        Ok(h) => h,
        Err(errors) => {
            for error in errors {
                let d: Diagnostic = error.into();
                print_diagnostic(&d, file_name, &source);
            }
            return Err("semantic analysis failed".into());
        }
    };

    let llvm_context = Context::create();
    let mut backend = Backend::new(&llvm_context, "hulk");
    match backend.compile_program(&hir, &analyzer) {
        Ok(_) => {}
        Err(err) => {
            eprintln!("backend error: {}", err);
            return Err("backend compilation failed".into());
        }
    }
    backend::emit::emit_ir_to_file(&backend.module, "output.ll")?;

    println!("Compilation succesful");
    Ok(())
}

fn read_file(file_path: &str) -> String {
    fs::read_to_string(file_path)
        .unwrap_or_else(|e| panic!("failed to read '{}': {}", file_path, e))
}
