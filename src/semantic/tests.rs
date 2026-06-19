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
