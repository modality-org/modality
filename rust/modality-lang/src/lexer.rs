use lalrpop_util::lexer::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum ModalityToken {
    Model,
    Part,
    Action,
    Test,
    Commit,
    Arrow,
    Colon,
    Plus,
    Minus,
    LeftParen,
    RightParen,
    Ident(String),
    Whitespace,
    Comment,
    Eof,
}

pub struct Lexer {
    input: Vec<char>,
    position: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        if self.position < self.input.len() {
            Some(self.input[self.position])
        } else {
            None
        }
    }

    fn advance(&mut self) {
        self.position += 1;
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_identifier(&mut self) -> String {
        let mut ident = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        ident
    }

    fn read_comment(&mut self) {
        // Skip // and everything until newline
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
    }

    pub fn next_token(&mut self) -> ModalityToken {
        self.skip_whitespace();

        if self.position >= self.input.len() {
            return ModalityToken::Eof;
        }

        let ch = self.input[self.position];
        self.advance();

        match ch {
            '/' => {
                if let Some('/') = self.peek() {
                    self.advance();
                    self.read_comment();
                    self.next_token() // Recursively get next token after comment
                } else {
                    // This shouldn't happen in valid Modality syntax
                    ModalityToken::Eof
                }
            }
            ':' => ModalityToken::Colon,
            '(' => ModalityToken::LeftParen,
            ')' => ModalityToken::RightParen,
            '+' => ModalityToken::Plus,
            '-' => {
                if let Some('>') = self.peek() {
                    self.advance();
                    ModalityToken::Arrow
                } else {
                    ModalityToken::Minus
                }
            }
            'a' => {
                let ident = self.read_identifier();
                if ident == "action" {
                    ModalityToken::Action
                } else {
                    ModalityToken::Ident(ident)
                }
            }
            'c' => {
                let ident = self.read_identifier();
                if ident == "commit" {
                    ModalityToken::Commit
                } else {
                    ModalityToken::Ident(ident)
                }
            }
            'm' => {
                let ident = self.read_identifier();
                if ident == "model" {
                    ModalityToken::Model
                } else {
                    ModalityToken::Ident(ident)
                }
            }
            'p' => {
                let ident = self.read_identifier();
                if ident == "part" {
                    ModalityToken::Part
                } else {
                    ModalityToken::Ident(ident)
                }
            }
            't' => {
                let ident = self.read_identifier();
                if ident == "test" {
                    ModalityToken::Test
                } else {
                    ModalityToken::Ident(ident)
                }
            }
            _ if ch.is_alphabetic() || ch == '_' => {
                let mut ident = String::from(ch);
                ident.push_str(&self.read_identifier());
                ModalityToken::Ident(ident)
            }
            _ => ModalityToken::Eof, // Skip unknown characters
        }
    }
}

impl Iterator for Lexer {
    type Item = ModalityToken;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.next_token();
        if token == ModalityToken::Eof {
            None
        } else {
            Some(token)
        }
    }
} 