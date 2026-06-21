use crate::semantic::{SemanticAnalyzer, error::SemanticErrorKind, test_utils::parse_program};

#[test]
fn semantic_unit_test_builtin() {
    let source = r#"
function good_f() => print("" @ sqrt(sin(cos(exp(log(E,PI * rand()))))));
    
print(42);
        "#;
    let program = parse_program(source);
    let mut analyzer = SemanticAnalyzer::new();
    let _ = analyzer.analyze_program(program);
    assert_eq!(analyzer.diagnostics.len(), 1);
    assert_eq!(
        analyzer.diagnostics[0].kind,
        SemanticErrorKind::FunctionArgumentTypeMismatch {
            name: "print".to_string(),
            index: 1,
            expected: "String".to_string(),
            found: "Number".to_string()
        }
    );
}

#[test]
fn semantic_generic_function_monomorphizes_per_concrete_type() {
    let source = r#"
function f(x) => x;

type Point(x: Number) {
    px = x;
}

{
    f(1);
    f("hello");
    f(new Point(1));
}
        "#;
    let program = parse_program(source);
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze_program(program);
    assert!(
        analyzer.diagnostics.is_empty(),
        "expected no diagnostics, got: {:?}",
        analyzer.diagnostics
    );
    let hir = result.expect("program should analyze successfully");
    let mangled_names: Vec<&str> = hir
        .node
        .monomorphized_functions
        .iter()
        .map(|d| match &d.node {
            crate::semantic::hir::TypedDeclKind::Function { name, .. } => name.as_str(),
            _ => panic!("expected Function in monomorphized_functions"),
        })
        .collect();
    assert_eq!(mangled_names.len(), 3, "expected 3 instantiations of f");
    assert!(mangled_names.contains(&"f$Number"));
    assert!(mangled_names.contains(&"f$String"));
    assert!(mangled_names.contains(&"f$Point"));
}

#[test]
fn semantic_generic_function_reuses_cached_instance_for_same_type() {
    let source = r#"
function f(x) => x;

{
    f(1);
    f(2);
    f(3);
}
        "#;
    let program = parse_program(source);
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze_program(program);
    assert!(analyzer.diagnostics.is_empty());
    let hir = result.expect("program should analyze successfully");
    assert_eq!(
        hir.node.monomorphized_functions.len(),
        1,
        "three calls with the same concrete type should produce a single instantiation"
    );
}

#[test]
fn semantic_unused_generic_function_produces_no_instantiation() {
    let source = r#"
function f(x) => x;

42;
        "#;
    let program = parse_program(source);
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze_program(program);
    assert!(analyzer.diagnostics.is_empty());
    let hir = result.expect("program should analyze successfully");
    assert!(hir.node.monomorphized_functions.is_empty());
}

#[test]
fn semantic_mixed_generic_and_concrete_params() {
    let source = r#"
function f(x, y: Number) => x;

{
    f("hello", 1);
    f(true, 2);
}
        "#;
    let program = parse_program(source);
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze_program(program);
    assert!(
        analyzer.diagnostics.is_empty(),
        "expected no diagnostics, got: {:?}",
        analyzer.diagnostics
    );
    let hir = result.expect("program should analyze successfully");
    assert_eq!(hir.node.monomorphized_functions.len(), 2);
}

#[test]
fn semantic_mixed_generic_function_still_checks_annotated_param() {
    let source = r#"
function f(x, y: Number) => x;

{
    f("hello", "not a number");
}
        "#;
    let program = parse_program(source);
    let mut analyzer = SemanticAnalyzer::new();
    let _ = analyzer.analyze_program(program);
    assert_eq!(analyzer.diagnostics.len(), 1);
    assert_eq!(
        analyzer.diagnostics[0].kind,
        SemanticErrorKind::FunctionArgumentTypeMismatch {
            name: "f".to_string(),
            index: 2,
            expected: "Number".to_string(),
            found: "String".to_string(),
        }
    );
}

#[test]
fn semantic_generic_function_infers_return_type_from_body() {
    let source = r#"
function f(x) => 42;

{
    f(1);
    f("hello");
}
        "#;
    let program = parse_program(source);
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze_program(program);
    assert!(analyzer.diagnostics.is_empty());
    let hir = result.expect("program should analyze successfully");
    for instance in &hir.node.monomorphized_functions {
        if let crate::semantic::hir::TypedDeclKind::Function { return_type, .. } = &instance.node {
            let number_id = analyzer.ctx.types.resolve("Number").unwrap();
            assert_eq!(*return_type, number_id);
        }
    }
}

#[test]
fn semantic_generic_function_call_arity_mismatch() {
    let source = r#"
function f(x) => x;

{
    f();
    f(1, 2);
}
        "#;
    let program = parse_program(source);
    let mut analyzer = SemanticAnalyzer::new();
    let _ = analyzer.analyze_program(program);
    assert_eq!(analyzer.diagnostics.len(), 2);
    assert_eq!(
        analyzer.diagnostics[0].kind,
        SemanticErrorKind::InvalidFunctionArity {
            name: "f".to_string(),
            expected: 1,
            found: 0,
        }
    );
    assert_eq!(
        analyzer.diagnostics[1].kind,
        SemanticErrorKind::InvalidFunctionArity {
            name: "f".to_string(),
            expected: 1,
            found: 2,
        }
    );
}

#[test]
fn semantic_unannotated_direct_recursion_fails_inference() {
    let source = r#"
function fact(n) => if (n == 0) { 1; } else { n * fact(n - 1); };

{
    fact(5);
}
        "#;
    let program = parse_program(source);
    let mut analyzer = SemanticAnalyzer::new();
    let _ = analyzer.analyze_program(program);
    assert!(
        analyzer.diagnostics.iter().any(|d| matches!(
            &d.kind,
            SemanticErrorKind::GenericInferenceFailed { function } if function == "fact"
        )),
        "expected GenericInferenceFailed for 'fact', got: {:?}",
        analyzer.diagnostics
    );
}

#[test]
fn semantic_annotated_direct_recursion_compiles_fine() {
    let source = r#"
function fact(n: Number): Number => if (n == 0) { 1; } else { n * fact(n - 1); };

{
    fact(5);
}
        "#;
    let program = parse_program(source);
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze_program(program);
    assert!(
        analyzer.diagnostics.is_empty(),
        "expected no diagnostics, got: {:?}",
        analyzer.diagnostics
    );
    let hir = result.expect("program should analyze successfully");
    assert!(hir.node.monomorphized_functions.is_empty());
}

#[test]
fn semantic_generic_function_used_as_variable_is_rejected() {
    let source = r#"
function f(x) => x;

let y = f in 42;
        "#;
    let program = parse_program(source);
    let mut analyzer = SemanticAnalyzer::new();
    let _ = analyzer.analyze_program(program);
    assert_eq!(analyzer.diagnostics.len(), 1);
    assert_eq!(
        analyzer.diagnostics[0].kind,
        SemanticErrorKind::NotAVariable {
            name: "f".to_string()
        }
    );
}

#[test]
fn semantic_generic_type_basic_instantiation() {
    let source = r#"
type Point(x, y) {
    x = x;
    y = y;
}

{
    new Point(1, 2);
    new Point("a", "b");
}
        "#;
    let program = parse_program(source);
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze_program(program);
    assert!(
        analyzer.diagnostics.is_empty(),
        "{:?}",
        analyzer.diagnostics
    );
    let hir = result.unwrap();
    assert_eq!(hir.node.monomorphized_types.len(), 2);
}

#[test]
fn semantic_generic_type_mixed_params() {
    let source = r#"
type Punto(x: Number, y) {
    x = x;
    y = y;
}

{
    new Punto(1, "a");
    new Punto(1, 2);
}
        "#;
    let program = parse_program(source);
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze_program(program);
    assert!(
        analyzer.diagnostics.is_empty(),
        "{:?}",
        analyzer.diagnostics
    );
    assert_eq!(result.unwrap().node.monomorphized_types.len(), 2);
}

#[test]
fn semantic_generic_type_reuses_same_instance() {
    let source = r#"
type Point(x, y) { x = x; y = y; }

{
    new Point(1, 2);
    new Point(3, 4);
}
        "#;
    let program = parse_program(source);
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze_program(program);
    assert!(analyzer.diagnostics.is_empty());
    assert_eq!(result.unwrap().node.monomorphized_types.len(), 1);
}

#[test]
fn semantic_generic_type_inheriting_from_generic_type() {
    let source = r#"
type Point(x, y) { x = x; y = y; }

type PolarPoint(phi, rho) inherits Point(rho * sin(phi), rho * cos(phi)) {}

{
    new PolarPoint(1, 2);
}
        "#;
    let program = parse_program(source);
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze_program(program);
    assert!(
        analyzer.diagnostics.is_empty(),
        "{:?}",
        analyzer.diagnostics
    );
    assert_eq!(result.unwrap().node.monomorphized_types.len(), 2);
}

#[test]
fn semantic_unused_generic_type_produces_no_instantiation() {
    let source = r#"
type Point(x, y) { x = x; y = y; }

42;
        "#;
    let program = parse_program(source);
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze_program(program);
    assert!(analyzer.diagnostics.is_empty());
    assert!(result.unwrap().node.monomorphized_types.is_empty());
}
