use super::{
    context::SemanticContext,
    symbols::{Symbol, SymbolKind, SymbolType},
};
use crate::lexer::span::Span;

pub fn install_builtins(ctx: &mut SemanticContext) {
    let number = ctx.types.resolve("Number").unwrap();
    let string = ctx.types.resolve("String").unwrap();
    ctx.declare(Symbol {
        name: "sqrt".to_string(),
        kind: SymbolKind::Function,
        ty: SymbolType::Function {
            params: vec![number],
            ret: number,
        },
        span: Span::new(0, 0),
    });
    ctx.declare(Symbol {
        name: "sin".to_string(),
        kind: SymbolKind::Function,
        ty: SymbolType::Function {
            params: vec![number],
            ret: number,
        },
        span: Span::new(0, 0),
    });
    ctx.declare(Symbol {
        name: "cos".to_string(),
        kind: SymbolKind::Function,
        ty: SymbolType::Function {
            params: vec![number],
            ret: number,
        },
        span: Span::new(0, 0),
    });
    ctx.declare(Symbol {
        name: "exp".to_string(),
        kind: SymbolKind::Function,
        ty: SymbolType::Function {
            params: vec![number],
            ret: number,
        },
        span: Span::new(0, 0),
    });
    ctx.declare(Symbol {
        name: "log".to_string(),
        kind: SymbolKind::Function,
        ty: SymbolType::Function {
            params: vec![number, number],
            ret: number,
        },
        span: Span::new(0, 0),
    });
    ctx.declare(Symbol {
        name: "rand".to_string(),
        kind: SymbolKind::Function,
        ty: SymbolType::Function {
            params: vec![],
            ret: number,
        },
        span: Span::new(0, 0),
    });
    ctx.declare(Symbol {
        name: "print".to_string(),
        kind: SymbolKind::Function,
        ty: SymbolType::Function {
            params: vec![string],
            ret: string,
        },
        span: Span::new(0, 0),
    });
    ctx.declare(Symbol {
        name: "PI".to_string(),
        kind: SymbolKind::Variable,
        ty: SymbolType::Variable(number),
        span: Span::new(0, 0),
    });
    ctx.declare(Symbol {
        name: "E".to_string(),
        kind: SymbolKind::Variable,
        ty: SymbolType::Variable(number),
        span: Span::new(0, 0),
    });
}
