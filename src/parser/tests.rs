use crate::parser::{program::*, test_utils::tokenize};
use chumsky::Parser;
use insta::assert_yaml_snapshot;

#[test]
fn parser_snapshot_unit_test() {
    let source = r#"
function aux() {

    // HULK Syntax Tour: A comprehensive guide to language constructs

    // 1. Built-in constants and math functions
    print(sin(2 * PI) ^ 2 + cos(3 * PI / log(4, 64)));

    // 2. Literals and basic expressions
    let a = 42, b = "Hello", c = true in {
        print("Escaped quote: \" and newline: \n");
    };
}

// 3. Custom Functions (Inline and Full-form)
function tan(x) => sin(x) / cos(x);

// 8. Polymorphism, base and @@ (spaced concatenation)
type Person(firstname, lastname) {
    firstname = firstname;
    lastname = lastname;
    name() => self.firstname @@ self.lastname;
}

type Knight() inherits Person() {
    name() => "John";
}

// Global expression entry point
print("HULK Tour Complete!");
    "#;

    let tokens = tokenize(source);

    let parser = program_parser();

    let ast = parser.parse(&tokens).into_result().expect("Parse error.");

    assert_yaml_snapshot!(ast);
}
