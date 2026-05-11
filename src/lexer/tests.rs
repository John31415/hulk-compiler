#[cfg(test)]
mod tests {
    use crate::lexer::*;

    #[test]
    fn test_eof() {
        let mut lexer = Lexer::new("");
        let tokens = lexer.tokenize().expect("Lexer failed.");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::EOF);
    }

    #[test]
    fn test_let() {
        use TokenKind::*;

        let mut lexer = Lexer::new("let x = 42;");
        let tokens = lexer.tokenize().expect("Lexer failed.");
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();
        let expected = vec![
            Let,
            Identifier(String::from("x")),
            Equal,
            LiteralNumber(42.0),
            Semi,
            EOF,
        ];
        assert_eq!(kinds, expected);
    }

    #[test]
    fn test_string() {
        use TokenKind::*;

        let input = r#"let str: String = "\\ abracadabra \n \" hello \". \q";"#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("Lexer failed.");
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();
        let expected = vec![
            Let,
            Identifier(String::from("str")),
            Colon,
            TypeString,
            Equal,
            LiteralString(String::from("\\ abracadabra \n \" hello \". \\q")),
            Semi,
            EOF,
        ];
        assert_eq!(kinds, expected);
    }

    #[test]
    fn test_error_unexpected_character() {
        let input = "let x = 10 # 5;";
        let mut lexer = Lexer::new(input);
        let result = lexer.tokenize();
        match result {
            Err(e) => assert_eq!(e.kind, LexErrorKind::UnexpectedCharacter),
            _ => panic!("Expected error: UnexpectedCharacter."),
        }
    }

    #[test]
    fn test_error_unclosed_string() {
        let input = r#"let s = "missing quote;"#;
        let mut lexer = Lexer::new(input);
        let result = lexer.tokenize();
        match result {
            Err(e) => assert_eq!(e.kind, LexErrorKind::UnclosedString),
            _ => panic!("Expected error: UnclosedString."),
        }
    }
}
