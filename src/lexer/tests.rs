#[cfg(test)]
mod tests {
    use crate::lexer::*;

    #[test]
    fn test_lexer_eof() {
        let mut lexer = Lexer::new("");
        let tokens = lexer.tokenize().expect("Lexer failed.");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::EOF);
    }

    #[test]
    fn test_lexer_let() {
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
    fn test_lexer_string() {
        use TokenKind::*;

        let input = r#"let str: String = "\\ abracadabra \n \" hello \".";"#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("Lexer failed.");
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();
        let expected = vec![
            Let,
            Identifier(String::from("str")),
            Colon,
            Identifier(String::from("String")),
            Equal,
            LiteralString(String::from("\\ abracadabra \n \" hello \".")),
            Semi,
            EOF,
        ];
        assert_eq!(kinds, expected);
    }

    #[test]
    fn test_lexer_true_false() {
        use TokenKind::*;

        let input = "let a = true, b = false in 42;";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("Lexer failed.");
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();
        let expected = vec![
            Let,
            Identifier(String::from("a")),
            Equal,
            LiteralTrue,
            Comma,
            Identifier(String::from("b")),
            Equal,
            LiteralFalse,
            In,
            LiteralNumber(42.0),
            Semi,
            EOF,
        ];
        assert_eq!(kinds, expected);
    }

    #[test]
    fn test_lexer_type() {
        use TokenKind::*;

        let input = "
        type Point(x, y) {
            x = x;
            y = y;
            getX() => self.x;
            getY() => self.y;
        }
        ";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("Lexer failed.");
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();
        let expected = vec![
            Type,
            Identifier(String::from("Point")),
            LParen,
            Identifier(String::from("x")),
            Comma,
            Identifier(String::from("y")),
            RParen,
            LBrace,
            Identifier(String::from("x")),
            Equal,
            Identifier(String::from("x")),
            Semi,
            Identifier(String::from("y")),
            Equal,
            Identifier(String::from("y")),
            Semi,
            Identifier(String::from("getX")),
            LParen,
            RParen,
            Arrow,
            Identifier(String::from("self")),
            Dot,
            Identifier(String::from("x")),
            Semi,
            Identifier(String::from("getY")),
            LParen,
            RParen,
            Arrow,
            Identifier(String::from("self")),
            Dot,
            Identifier(String::from("y")),
            Semi,
            RBrace,
            EOF,
        ];
        assert_eq!(kinds, expected);
    }

    #[test]
    fn test_lexer_function_inline_while() {
        use TokenKind::*;

        let input = "
        function gcd(a, b) => while (a > 0)
            let m = a / b in {
                b := a;
                a := m;
            };
        ";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("Lexer failed.");
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();
        let expected = vec![
            Function,
            Identifier(String::from("gcd")),
            LParen,
            Identifier(String::from("a")),
            Comma,
            Identifier(String::from("b")),
            RParen,
            Arrow,
            While,
            LParen,
            Identifier(String::from("a")),
            Greater,
            LiteralNumber(0.0),
            RParen,
            Let,
            Identifier(String::from("m")),
            Equal,
            Identifier(String::from("a")),
            Slash,
            Identifier(String::from("b")),
            In,
            LBrace,
            Identifier(String::from("b")),
            ColonEqual,
            Identifier(String::from("a")),
            Semi,
            Identifier(String::from("a")),
            ColonEqual,
            Identifier(String::from("m")),
            Semi,
            RBrace,
            Semi,
            EOF,
        ];
        assert_eq!(kinds, expected);
    }

    #[test]
    fn test_lexer_for_is_as() {
        use TokenKind::*;

        let input = "
        for (x in new Circle(5))
            if (x is Shape) 1
            elif 2
            else 3;
        ";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("Lexer failed.");
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();
        let expected = vec![
            For,
            LParen,
            Identifier(String::from("x")),
            In,
            New,
            Identifier(String::from("Circle")),
            LParen,
            LiteralNumber(5.0),
            RParen,
            RParen,
            If,
            LParen,
            Identifier(String::from("x")),
            Is,
            Identifier(String::from("Shape")),
            RParen,
            LiteralNumber(1.0),
            Elif,
            LiteralNumber(2.0),
            Else,
            LiteralNumber(3.0),
            Semi,
            EOF,
        ];
        assert_eq!(kinds, expected);
    }

    #[test]
    fn test_lexer_error_unexpected_character() {
        let input = "let x = 10 # 5;";
        let mut lexer = Lexer::new(input);
        let result = lexer.tokenize();
        match result {
            Err(e) => assert_eq!(e.kind, LexErrorKind::UnexpectedCharacter),
            _ => panic!("Expected error: UnexpectedCharacter."),
        }
    }

    #[test]
    fn test_lexer_error_unclosed_string() {
        let input = r#"let s = "missing quote;"#;
        let mut lexer = Lexer::new(input);
        let result = lexer.tokenize();
        match result {
            Err(e) => assert_eq!(e.kind, LexErrorKind::UnclosedString),
            _ => panic!("Expected error: UnclosedString."),
        }
    }

    #[test]
    fn test_lexer_error_leading_zero() {
        let input = r#"let n = 01;"#;
        let mut lexer = Lexer::new(input);
        let result = lexer.tokenize();
        match result {
            Err(e) => assert_eq!(e.kind, LexErrorKind::LeadingZero),
            _ => panic!("Expected error: LeadingZero."),
        }
    }

    #[test]
    fn test_lexer_error_invalide_escape() {
        let input = r#"let n = "a \x b";"#;
        let mut lexer = Lexer::new(input);
        let result = lexer.tokenize();
        match result {
            Err(e) => assert_eq!(e.kind, LexErrorKind::InvalidEscapeSequence),
            _ => panic!("Expected error: InvalidEscapeSequence."),
        }
    }

    #[test]
    fn test_lexer_error_malformed_number() {
        let input = r#"let n = 1.;"#;
        let mut lexer = Lexer::new(input);
        let result = lexer.tokenize();
        match result {
            Err(e) => assert_eq!(e.kind, LexErrorKind::MalformedNumber),
            _ => panic!("Expected error: MalformedNumber."),
        }
    }

    #[test]
    fn test_lexer_error_numeric_overflow() {
        let input = r#"let n = 100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000;"#;
        let mut lexer = Lexer::new(input);
        let result = lexer.tokenize();
        match result {
            Err(e) => assert_eq!(e.kind, LexErrorKind::NumericOverflow),
            _ => panic!("Expected error: NumericOverflow."),
        }
    }
}
