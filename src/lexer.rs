/// Represents the type of a token in the language.
#[derive(Debug, PartialEq, Clone)]
pub enum TokenType {
    // End of file
    Eof,

    // Keywords
    Fn,
    Extern,
    If,
    Else,
    Then,
    For,
    In,
    While,
    Return,
    Var,

    // Types
    F8Type,
    F16Type,
    F32Type,
    F64Type,

    // Identifiers and literals
    Identifier,
    Number,

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Semicolon,
    Colon,

    // Single-char operators
    Plus,
    Minus,
    Star,
    Slash,
    Less,
    Greater,
    Assign,
    Bang,
    Pipe,
    Ampersand,
    Caret,
    Percent,
    Dollar,
    At,
    Tilde,

    // Multi-char operators
    Equal,        // ==
    NotEqual,     // !=
    LessEqual,    // <=
    GreaterEqual, // >=
    And,          // &&
    Or,           // ||
    Arrow,        // ->
}

/// Error type returned when lexing fails.
#[derive(Debug, Clone)]
pub struct LexError {
    pub message: String,
    pub row: usize,
    pub column: usize,
}

/// A single token with its type, lexeme, and source location.
#[derive(Debug, Clone)]
pub struct Token {
    pub tag: TokenType,
    pub lexeme: String,
    pub row: usize,
    pub column: usize,
}

/// The lexer context that maintains state during lexical analysis.
pub struct LexerContext {
    tokens: Vec<Token>,
    row: usize,
    column: usize,
    cursor: usize,
    input: String,
}

impl LexerContext {

    /// Peeks at a character at the given lookahead offset from the current cursor position.
    /// Returns `None` if the position is beyond the end of the input.
    fn peek(&self, lookahead: usize) -> Option<char> {
        let remaining = &self.input[self.cursor..];
        remaining.chars().nth(lookahead)
    }

    /// Advances the cursor by one character, updating row and column tracking.
    /// If at a newline, increments the row and resets the column.
    /// Does nothing if already at the end of input.
    fn advance(&mut self) {
        if let Some(c) = self.peek(0) {
            if c == '\n' {
                self.column = 0;
                self.row += 1;
            } else {
                self.column += 1;
            }
            self.cursor += 1;
        }
    }

    /// Advances the cursor by `n` characters.
    fn advance_by(&mut self, n: usize) {
        for _ in 0..n {
            self.advance();
        }
    }

    /// Adds a token to the token list at the current position without advancing the cursor.
    /// The token is tagged with the current row and column.
    fn add_token(&mut self, tag: TokenType, lexeme: String) {
        let token = Token {
            tag,
            lexeme,
            row: self.row,
            column: self.column,
        };
        self.tokens.push(token);
    }

    /// Adds a token and advances the cursor by the length of the lexeme.
    /// This is a convenience method for single-use tokens where the lexeme length
    /// matches the number of characters to consume.
    fn push_token(&mut self, tag: TokenType, lexeme: String) {
        self.add_token(tag, lexeme.clone());
        self.advance_by(lexeme.len());
    }

    /// Attempts to match and consume a multi-character operator token.
    /// Checks the current character and the next character using peek(1).
    /// Returns `true` if a multi-char token was matched and added, `false` otherwise.
    fn try_push_multi_char_token(&mut self, c: char) -> bool {
        let next = self.peek(1);

        match (c, next) {
            ('=', Some('=')) => {
                self.push_token(TokenType::Equal, "==".to_string());
                true
            }
            ('!', Some('=')) => {
                self.push_token(TokenType::NotEqual, "!=".to_string());
                true
            }
            ('<', Some('=')) => {
                self.push_token(TokenType::LessEqual, "<=".to_string());
                true
            }
            ('>', Some('=')) => {
                self.push_token(TokenType::GreaterEqual, ">=".to_string());
                true
            }
            ('&', Some('&')) => {
                self.push_token(TokenType::And, "&&".to_string());
                true
            }
            ('|', Some('|')) => {
                self.push_token(TokenType::Or, "||".to_string());
                true
            }
            ('-', Some('>')) => {
                self.push_token(TokenType::Arrow, "->".to_string());
                true
            }
            _ => false,
        }
    }

    /// Attempts to match and consume a single-character token.
    /// Returns `true` if the character was recognized as a token, `false` otherwise.
    fn try_push_single_char_token(&mut self, c: char) -> bool {
        let token_type = match c {
            '(' => TokenType::LParen,
            ')' => TokenType::RParen,
            '{' => TokenType::LBrace,
            '}' => TokenType::RBrace,
            ';' => TokenType::Semicolon,
            ':' => TokenType::Colon,
            '+' => TokenType::Plus,
            ',' => TokenType::Comma,
            '-' => TokenType::Minus,
            '/' => TokenType::Slash,
            '*' => TokenType::Star,
            '>' => TokenType::Greater,
            '<' => TokenType::Less,
            '=' => TokenType::Assign,
            '!' => TokenType::Bang,
            '|' => TokenType::Pipe,
            '&' => TokenType::Ampersand,
            '^' => TokenType::Caret,
            '%' => TokenType::Percent,
            '$' => TokenType::Dollar,
            '@' => TokenType::At,
            '~' => TokenType::Tilde,
            _ => return false,
        };
        self.push_token(token_type, c.to_string());
        true
    }

    /// Lexes the input string and returns a vector of tokens.
    ///
    /// This method consumes the lexer context and returns the complete list of tokens,
    /// including an EOF token at the end. It recognizes:
    /// - Keywords: fn, extern, if, else, then, for, in, while, return, var
    /// - Types: f64
    /// - Identifiers: alphanumeric with underscores (e.g., `my_var`, `_private`)
    /// - Number literals: integers and floats (e.g., `123`, `3.14`)
    /// - Single-char operators: +, -, *, /, <, >, =, !, |, &, ^, %, $, @, ~
    /// - Multi-char operators: ==, !=, <=, >=, &&, ||, ->
    /// - Delimiters: (, ), {, }, ,, ;, :
    /// - Comments: lines starting with #
    ///
    /// # Errors
    /// Returns a `LexError` if an unexpected character is encountered.
    ///
    /// # Example
    /// ```ignore
    /// let tokens = LexerContext::lex("fn foo(x: f64) -> f64 { return x + 1; }")?;
    /// ```
    pub fn lex(input: &str) -> Result<Vec<Token>, LexError> {
        let mut lexer = LexerContext {
            tokens: Vec::new(),
            row: 0,
            column: 0,
            cursor: 0,
            input: input.to_string(),
        };

        while let Some(c) = lexer.peek(0) {
            // Whitespace
            if c.is_whitespace() {
                lexer.advance();
                continue;
            }

            // Line Comments
            if c == '#' {
                while matches!(lexer.peek(0), Some(c) if c != '\n') {
                    lexer.advance();
                }
                continue;
            }

            // Multi-character operators (try first)
            if lexer.try_push_multi_char_token(c) {
                continue;
            }

            // Single character tokens
            if lexer.try_push_single_char_token(c) {
                continue;
            }

            // Numbers
            if c.is_ascii_digit() {
                let start = lexer.cursor;
                lexer.advance();
                let mut has_dot = false;

                while let Some(next_c) = lexer.peek(0) {
                    if next_c.is_ascii_digit() {
                        lexer.advance();
                    } else if next_c == '.' && !has_dot {
                        has_dot = true;
                        lexer.advance();
                    } else {
                        break;
                    }
                }

                let lexeme = lexer.input[start..lexer.cursor].to_string();
                lexer.add_token(TokenType::Number, lexeme);
                continue;
            }

            // Identifiers and keywords
            if c.is_alphabetic() || c == '_' {
                let start = lexer.cursor;
                lexer.advance();

                while let Some(next_c) = lexer.peek(0) {
                    if next_c.is_alphanumeric() || next_c == '_' {
                        lexer.advance();
                    } else {
                        break;
                    }
                }

                let lexeme = lexer.input[start..lexer.cursor].to_string();
                let token_type = match lexeme.as_str() {
                    "fn" => TokenType::Fn,
                    "extern" => TokenType::Extern,
                    "var" => TokenType::Var,
                    "if" => TokenType::If,
                    "else" => TokenType::Else,
                    "then" => TokenType::Then,
                    "for" => TokenType::For,
                    "in" => TokenType::In,
                    "while" => TokenType::While,
                    "return" => TokenType::Return,
                    "f8" => TokenType::F8Type,
                    "f16" => TokenType::F16Type,
                    "f32" => TokenType::F32Type,
                    "f64" => TokenType::F64Type,
                    _ => TokenType::Identifier,
                };
                lexer.add_token(token_type, lexeme);
                continue;
            }

            // Unknown character - error
            return Err(LexError {
                message: format!("Unexpected character '{}'", c),
                row: lexer.row + 1,
                column: lexer.column + 1,
            });
        }

        lexer.add_token(TokenType::Eof, String::new());
        Ok(lexer.tokens)
    }
}
