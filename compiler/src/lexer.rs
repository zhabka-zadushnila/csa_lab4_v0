#![allow(clippy::upper_case_acronyms)]
use std::{iter::Peekable, str::Chars};

#[derive(Debug, PartialEq, Clone)]
pub enum TokenKind {
    RoundBracketOpen,
    RoundBracketClose,
    SquareBracketOpen,
    SquareBracketClose,
    CurlyBracketOpen,
    CurlyBracketClose,

    NumberInteger(i32),
    StringLiteral(String),
    CharLiteral(char),
    Ptr,

    OpAdd,
    OpSub,
    OpMul,
    OpDiv,
    OpRem,
    OpNot,
    OpEq,
    OpNotEq,
    OpGreaterOrEq,
    OpLessOrEq,
    OpGreaterThan,
    OpLessThan,
    OpShiftLeft,
    OpShiftRight,
    OpInc,
    OpDec,

    OpAnd,
    OpOr,
    OpBitAnd,
    OpBitOr,
    OpXor,

    Variable(String),
    Semicolon,
    Comma,
    Assign,
    Colon,

    If,
    Else,
    While,
    For,
    True,
    False,
    Cout,
    Cin,
    I32Type,
    Return,
    Function,
    Arrow,

    EOF,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum LexerError {
    InvalidNumber(String),
    UnexpectedChar(char),
    UnexpectedEOF,
}

pub struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Lexer {
            chars: input.chars().peekable(),
            line: 1,
            col: 1,
        }
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.next();
        if let Some(ch) = c {
            if ch == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        c
    }

    fn peek(&mut self) -> Option<&char> {
        self.chars.peek()
    }

    pub fn parse(&mut self) -> Result<Vec<Token>, String> {
        let mut result = Vec::new();

        while let Some(&c) = self.peek() {
            let start_line = self.line;
            let start_col = self.col;

            macro_rules! push_token {
                ($kind:expr) => {{
                    self.advance();
                    result.push(Token {
                        kind: $kind,
                        line: start_line,
                        col: start_col,
                    });
                }};
            }

            match c {
                c if c.is_whitespace() => {
                    self.advance();
                }
                '(' => push_token!(TokenKind::RoundBracketOpen),
                ')' => push_token!(TokenKind::RoundBracketClose),
                '{' => push_token!(TokenKind::CurlyBracketOpen),
                '}' => push_token!(TokenKind::CurlyBracketClose),
                '[' => push_token!(TokenKind::SquareBracketOpen),
                ']' => push_token!(TokenKind::SquareBracketClose),
                ';' => push_token!(TokenKind::Semicolon),
                ',' => push_token!(TokenKind::Comma),

                ':' => push_token!(TokenKind::Colon),
                '*' => push_token!(TokenKind::OpMul),
                '%' => push_token!(TokenKind::OpRem),
                '^' => push_token!(TokenKind::OpXor),

                '+' => {
                    self.advance();
                    if self.peek() == Some(&'+') {
                        self.advance();
                        result.push(Token {
                            kind: TokenKind::OpInc,
                            line: start_line,
                            col: start_col,
                        });
                    } else {
                        result.push(Token {
                            kind: TokenKind::OpAdd,
                            line: start_line,
                            col: start_col,
                        });
                    }
                }
                '-' => {
                    self.advance();
                    if self.peek() == Some(&'>') {
                        self.advance();
                        result.push(Token {
                            kind: TokenKind::Arrow,
                            line: start_line,
                            col: start_col,
                        });
                    } else if self.peek() == Some(&'-') {
                        self.advance();
                        result.push(Token {
                            kind: TokenKind::OpDec,
                            line: start_line,
                            col: start_col,
                        });
                    } else {
                        result.push(Token {
                            kind: TokenKind::OpSub,
                            line: start_line,
                            col: start_col,
                        });
                    }
                }
                '/' => {
                    self.advance();
                    if self.peek() == Some(&'/') {
                        while let Some(&ch) = self.peek() {
                            if ch == '\n' {
                                break;
                            }
                            self.advance();
                        }
                    } else {
                        result.push(Token {
                            kind: TokenKind::OpDiv,
                            line: start_line,
                            col: start_col,
                        });
                    }
                }
                '=' => {
                    self.advance();
                    if self.peek() == Some(&'=') {
                        self.advance();
                        result.push(Token {
                            kind: TokenKind::OpEq,
                            line: start_line,
                            col: start_col,
                        });
                    } else {
                        result.push(Token {
                            kind: TokenKind::Assign,
                            line: start_line,
                            col: start_col,
                        });
                    }
                }
                '>' => {
                    self.advance();
                    if self.peek() == Some(&'=') {
                        self.advance();
                        result.push(Token {
                            kind: TokenKind::OpGreaterOrEq,
                            line: start_line,
                            col: start_col,
                        });
                    } else if self.peek() == Some(&'>') {
                        self.advance();
                        result.push(Token {
                            kind: TokenKind::OpShiftRight,
                            line: start_line,
                            col: start_col,
                        });
                    } else {
                        result.push(Token {
                            kind: TokenKind::OpGreaterThan,
                            line: start_line,
                            col: start_col,
                        });
                    }
                }
                '<' => {
                    self.advance();
                    if self.peek() == Some(&'=') {
                        self.advance();
                        result.push(Token {
                            kind: TokenKind::OpLessOrEq,
                            line: start_line,
                            col: start_col,
                        });
                    } else if self.peek() == Some(&'<') {
                        self.advance();
                        result.push(Token {
                            kind: TokenKind::OpShiftLeft,
                            line: start_line,
                            col: start_col,
                        });
                    } else {
                        result.push(Token {
                            kind: TokenKind::OpLessThan,
                            line: start_line,
                            col: start_col,
                        });
                    }
                }
                '!' => {
                    self.advance();
                    if self.peek() == Some(&'=') {
                        self.advance();
                        result.push(Token {
                            kind: TokenKind::OpNotEq,
                            line: start_line,
                            col: start_col,
                        });
                    } else {
                        result.push(Token {
                            kind: TokenKind::OpNot,
                            line: start_line,
                            col: start_col,
                        });
                    }
                }
                '&' => {
                    self.advance();
                    if self.peek() == Some(&'&') {
                        self.advance();
                        result.push(Token {
                            kind: TokenKind::OpAnd,
                            line: start_line,
                            col: start_col,
                        });
                    } else {
                        result.push(Token {
                            kind: TokenKind::OpBitAnd,
                            line: start_line,
                            col: start_col,
                        });
                    }
                }
                '|' => {
                    self.advance();
                    if self.peek() == Some(&'|') {
                        self.advance();
                        result.push(Token {
                            kind: TokenKind::OpOr,
                            line: start_line,
                            col: start_col,
                        });
                    } else {
                        result.push(Token {
                            kind: TokenKind::OpBitOr,
                            line: start_line,
                            col: start_col,
                        });
                    }
                }
                '\'' => {
                    self.advance();
                    let mut char_val = self.advance().unwrap();
                    if char_val == '\\' {
                        char_val = match self.advance().unwrap() {
                            'n' => '\n',
                            't' => '\t',
                            '\\' => '\\',
                            '\'' => '\'',
                            other => other,
                        };
                    }
                    self.advance(); // consume closing quote
                    result.push(Token {
                        kind: TokenKind::CharLiteral(char_val),
                        line: start_line,
                        col: start_col,
                    });
                }
                '"' => {
                    self.advance();
                    let mut s = String::new();
                    while let Some(&ch) = self.peek() {
                        if ch == '"' {
                            self.advance();
                            break;
                        }
                        if ch == '\\' {
                            self.advance();
                            s.push(match self.advance().unwrap() {
                                'n' => '\n',
                                't' => '\t',
                                '\\' => '\\',
                                '"' => '"',
                                other => other,
                            });
                        } else {
                            s.push(self.advance().unwrap());
                        }
                    }
                    result.push(Token {
                        kind: TokenKind::StringLiteral(s),
                        line: start_line,
                        col: start_col,
                    });
                }
                c if c.is_alphabetic() || c == '_' => {
                    let mut s = String::new();
                    while let Some(&ch) = self.peek() {
                        if ch.is_alphanumeric() || ch == '_' {
                            s.push(self.advance().unwrap());
                        } else {
                            break;
                        }
                    }
                    let kind = match s.as_str() {
                        "if" => TokenKind::If,
                        "else" => TokenKind::Else,
                        "while" => TokenKind::While,
                        "for" => TokenKind::For,
                        "true" => TokenKind::True,
                        "false" => TokenKind::False,
                        "return" => TokenKind::Return,
                        "fn" => TokenKind::Function,
                        "cout" => TokenKind::Cout,
                        "cin" => TokenKind::Cin,
                        "i32" => TokenKind::I32Type,
                        "ptr" => TokenKind::Ptr,
                        _ => TokenKind::Variable(s),
                    };
                    result.push(Token {
                        kind,
                        line: start_line,
                        col: start_col,
                    });
                }
                c if c.is_numeric() => {
                    let mut s = String::new();
                    s.push(self.advance().unwrap());
                    if c == '0'
                        && let Some(&next) = self.peek()
                    {
                        if next == 'x' || next == 'X' {
                            self.advance();
                            let mut hex_s = String::new();
                            while let Some(&ch) = self.peek() {
                                if ch.is_ascii_hexdigit() {
                                    hex_s.push(self.advance().unwrap());
                                } else {
                                    break;
                                }
                            }
                            let val = i32::from_str_radix(&hex_s, 16).unwrap();
                            result.push(Token {
                                kind: TokenKind::NumberInteger(val),
                                line: start_line,
                                col: start_col,
                            });
                            continue;
                        } else if next == 'b' || next == 'B' {
                            self.advance();
                            let mut bin_s = String::new();
                            while let Some(&ch) = self.peek() {
                                if ch == '0' || ch == '1' {
                                    bin_s.push(self.advance().unwrap());
                                } else {
                                    break;
                                }
                            }
                            let val = i32::from_str_radix(&bin_s, 2).unwrap();
                            result.push(Token {
                                kind: TokenKind::NumberInteger(val),
                                line: start_line,
                                col: start_col,
                            });
                            continue;
                        }
                    }
                    while let Some(&ch) = self.peek() {
                        if ch.is_numeric() {
                            s.push(self.advance().unwrap());
                        } else {
                            break;
                        }
                    }
                    result.push(Token {
                        kind: TokenKind::NumberInteger(s.parse().unwrap()),
                        line: start_line,
                        col: start_col,
                    });
                }
                _ => return Err(format!("Unexpected char: {}", self.advance().unwrap())),
            }
        }
        result.push(Token {
            kind: TokenKind::EOF,
            line: self.line,
            col: self.col,
        });
        Ok(result)
    }
}
