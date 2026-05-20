use crate::lexer::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum ParserError {
    UnexpectedToken(TokenKind, TokenKind),
    UnexpectedEOF,
    Generic(String),
}

#[derive(Debug, Clone)]
pub enum Value {
    Integer(i32),
    String(String),
    Char(char),
    Boolean(bool),
    Variable(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Ops {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    NotEq,
    GreaterEq,
    LessEq,
    Greater,
    Less,
    And,
    Or,
    BitAnd,
    BitOr,
    Xor,
    Assign,
    Inc,
    Dec,
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub enum Expression {
    Atom(Value),
    Operation(Ops, Vec<Expression>),
    Block(Vec<Expression>),
    VarDecl {
        typ: String,
        name: String,
        init: Option<Box<Expression>>,
        size: Option<Box<Expression>>,
    },
    FuncDecl {
        name: String,
        args: Vec<(String, String)>,
        ret_type: Option<String>,
        body: Box<Expression>,
    },
    FuncCall {
        caller: Box<Expression>,
        args: Vec<Expression>,
    },
    If {
        cond: Box<Expression>,
        then_br: Box<Expression>,
        else_br: Option<Box<Expression>>,
    },
    While {
        cond: Box<Expression>,
        body: Box<Expression>,
    },
    For {
        init: Box<Expression>,
        cond: Box<Expression>,
        step: Box<Expression>,
        body: Box<Expression>,
    },
    Return(Option<Box<Expression>>),
    ArrayInit(Vec<Expression>),
    ArrayIndex {
        arr: Box<Expression>,
        index: Box<Expression>,
    },
    Cout(Vec<Expression>),
    Cin(Vec<Expression>),
    Absolute(Box<Expression>),
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Parser {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> &TokenKind {
        self.tokens
            .get(self.pos)
            .map(|t| &t.kind)
            .unwrap_or(&TokenKind::EOF)
    }

    fn next_token(&mut self) -> TokenKind {
        let kind = self.peek().clone();
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        kind
    }

    fn expect(&mut self, expected: TokenKind) -> Result<(), ParserError> {
        let current = self.next_token();
        if current == expected {
            Ok(())
        } else {
            Err(ParserError::UnexpectedToken(expected, current))
        }
    }

    fn expect_ident(&mut self) -> Result<String, ParserError> {
        match self.next_token() {
            TokenKind::Variable(s) => Ok(s),
            t => Err(ParserError::Generic(format!(
                "Expected identifier, found {:?}",
                t
            ))),
        }
    }

    pub fn parse_program(&mut self) -> Result<Expression, ParserError> {
        let mut statements = Vec::new();
        while *self.peek() != TokenKind::EOF {
            statements.push(self.parse_statement()?);
        }
        Ok(Expression::Block(statements))
    }

    fn parse_statement(&mut self) -> Result<Expression, ParserError> {
        match self.peek() {
            TokenKind::Function => self.parse_fn(),
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::For => self.parse_for(),
            TokenKind::Return => {
                self.next_token();
                let expr = if *self.peek() != TokenKind::Semicolon {
                    Some(Box::new(self.parse_expr(0)?))
                } else {
                    None
                };
                self.expect(TokenKind::Semicolon)?;
                Ok(Expression::Return(expr))
            }
            TokenKind::Cout => self.parse_cout(),
            TokenKind::Cin => self.parse_cin(),
            TokenKind::CurlyBracketOpen => self.parse_block(),
            _ => {
                let is_decl = matches!(
                    (self.peek(), self.tokens.get(self.pos + 1).map(|t| &t.kind)),
                    (TokenKind::I32Type, Some(TokenKind::Variable(_)))
                        | (TokenKind::Ptr, Some(TokenKind::Variable(_)))
                        | (TokenKind::Variable(_), Some(TokenKind::Variable(_)))
                );

                if is_decl {
                    self.parse_var_decl()
                } else {
                    let expr = self.parse_expr(0)?;
                    self.expect(TokenKind::Semicolon)?;
                    Ok(expr)
                }
            }
        }
    }

    fn parse_var_decl(&mut self) -> Result<Expression, ParserError> {
        let typ = self.parse_type()?;
        let name = self.expect_ident()?;
        let mut init = None;
        let mut size = None;

        let next_token = self.peek();
        if *next_token == TokenKind::SquareBracketOpen {
            self.next_token();
            size = Some(Box::new(self.parse_expr(0)?));
            self.expect(TokenKind::SquareBracketClose)?;
        } else if *next_token == TokenKind::Assign {
            self.next_token();
            init = Some(Box::new(self.parse_expr(0)?));
        }
        self.expect(TokenKind::Semicolon)?;
        Ok(Expression::VarDecl {
            typ,
            name,
            init,
            size,
        })
    }

    fn parse_type(&mut self) -> Result<String, ParserError> {
        match self.next_token() {
            TokenKind::I32Type => Ok("i32".to_string()),
            TokenKind::Ptr => Ok("ptr".to_string()),
            TokenKind::Variable(s) => Ok(s),
            t => Err(ParserError::Generic(format!("Expected type, got {:?}", t))),
        }
    }

    fn parse_block(&mut self) -> Result<Expression, ParserError> {
        self.expect(TokenKind::CurlyBracketOpen)?;
        let mut stmts = Vec::new();
        while *self.peek() != TokenKind::CurlyBracketClose && *self.peek() != TokenKind::EOF {
            stmts.push(self.parse_statement()?);
        }
        self.expect(TokenKind::CurlyBracketClose)?;
        Ok(Expression::Block(stmts))
    }

    fn parse_fn(&mut self) -> Result<Expression, ParserError> {
        self.expect(TokenKind::Function)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::RoundBracketOpen)?;
        let mut args = Vec::new();
        if *self.peek() != TokenKind::RoundBracketClose {
            loop {
                let typ = self.parse_type()?;
                let arg_name = self.expect_ident()?;
                args.push((typ, arg_name));
                if *self.peek() == TokenKind::Comma {
                    self.next_token();
                } else {
                    break;
                }
            }
        }
        self.expect(TokenKind::RoundBracketClose)?;

        let mut ret_type = None;
        if *self.peek() == TokenKind::Arrow {
            self.next_token();
            ret_type = Some(self.parse_type()?);
        }
        let body = Box::new(self.parse_block()?);
        Ok(Expression::FuncDecl {
            name,
            args,
            ret_type,
            body,
        })
    }

    fn parse_if(&mut self) -> Result<Expression, ParserError> {
        self.expect(TokenKind::If)?;
        let cond = Box::new(self.parse_expr(0)?);
        let then_br = Box::new(self.parse_block()?);
        let mut else_br = None;
        if *self.peek() == TokenKind::Else {
            self.next_token();
            else_br = Some(Box::new(if *self.peek() == TokenKind::If {
                self.parse_if()?
            } else {
                self.parse_block()?
            }));
        }
        Ok(Expression::If {
            cond,
            then_br,
            else_br,
        })
    }

    fn parse_while(&mut self) -> Result<Expression, ParserError> {
        self.expect(TokenKind::While)?;
        let cond = Box::new(self.parse_expr(0)?);
        let body = Box::new(self.parse_block()?);
        Ok(Expression::While { cond, body })
    }

    fn parse_for(&mut self) -> Result<Expression, ParserError> {
        self.expect(TokenKind::For)?;
        self.expect(TokenKind::RoundBracketOpen)?;
        let init = Box::new(self.parse_statement()?); // parse_statement handles the ';'
        let cond = Box::new(self.parse_expr(0)?);
        self.expect(TokenKind::Semicolon)?;
        let step = Box::new(self.parse_expr(0)?);
        self.expect(TokenKind::RoundBracketClose)?;
        let body = Box::new(self.parse_block()?);
        Ok(Expression::For {
            init,
            cond,
            step,
            body,
        })
    }

    fn parse_cout(&mut self) -> Result<Expression, ParserError> {
        self.expect(TokenKind::Cout)?;
        let mut exprs = Vec::new();
        while *self.peek() == TokenKind::OpShiftLeft {
            self.next_token();
            exprs.push(self.parse_expr(0)?);
        }
        self.expect(TokenKind::Semicolon)?;
        Ok(Expression::Cout(exprs))
    }

    fn parse_cin(&mut self) -> Result<Expression, ParserError> {
        self.expect(TokenKind::Cin)?;
        let mut exprs = Vec::new();
        while *self.peek() == TokenKind::OpShiftRight {
            self.next_token();
            exprs.push(self.parse_expr(0)?);
        }
        self.expect(TokenKind::Semicolon)?;
        Ok(Expression::Cin(exprs))
    }

    // Pratt Parser for Expressions
    fn parse_expr(&mut self, min_bp: u8) -> Result<Expression, ParserError> {
        let mut lhs = self.parse_prefix()?;

        loop {
            let kind = self.peek().clone();

            // Postfix operators
            match kind {
                TokenKind::SquareBracketOpen => {
                    self.next_token();
                    let index = self.parse_expr(0)?;
                    self.expect(TokenKind::SquareBracketClose)?;
                    lhs = Expression::ArrayIndex {
                        arr: Box::new(lhs),
                        index: Box::new(index),
                    };
                    continue;
                }
                TokenKind::RoundBracketOpen => {
                    self.next_token();
                    let mut args = Vec::new();
                    if *self.peek() != TokenKind::RoundBracketClose {
                        loop {
                            args.push(self.parse_expr(0)?);
                            if *self.peek() == TokenKind::Comma {
                                self.next_token();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(TokenKind::RoundBracketClose)?;
                    lhs = Expression::FuncCall {
                        caller: Box::new(lhs),
                        args,
                    };
                    continue;
                }
                TokenKind::OpInc => {
                    self.next_token();
                    lhs = Expression::Operation(Ops::Inc, vec![lhs]);
                    continue;
                }
                TokenKind::OpDec => {
                    self.next_token();
                    lhs = Expression::Operation(Ops::Dec, vec![lhs]);
                    continue;
                }
                _ => {}
            }

            // Infix operators
            if let Some((l_bp, r_bp)) = self.infix_binding_power(&kind) {
                if l_bp < min_bp {
                    break;
                }
                self.next_token();
                let op = self.token_to_op(&kind);
                let rhs = self.parse_expr(r_bp)?;
                lhs = Expression::Operation(op, vec![lhs, rhs]);
                continue;
            }
            break;
        }
        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expression, ParserError> {
        match self.next_token() {
            TokenKind::NumberInteger(n) => Ok(Expression::Atom(Value::Integer(n))),
            TokenKind::StringLiteral(s) => Ok(Expression::Atom(Value::String(s))),
            TokenKind::CharLiteral(c) => Ok(Expression::Atom(Value::Char(c))),
            TokenKind::True => Ok(Expression::Atom(Value::Boolean(true))),
            TokenKind::False => Ok(Expression::Atom(Value::Boolean(false))),
            TokenKind::Variable(s) => Ok(Expression::Atom(Value::Variable(s))),
            TokenKind::RoundBracketOpen => {
                let expr = self.parse_expr(0)?;
                self.expect(TokenKind::RoundBracketClose)?;
                Ok(expr)
            }
            TokenKind::OpSub => Ok(Expression::Operation(Ops::Neg, vec![self.parse_expr(23)?])),
            TokenKind::OpNot => Ok(Expression::Operation(Ops::Not, vec![self.parse_expr(23)?])),
            TokenKind::OpBitOr => {
                let expr = self.parse_expr(8)?;
                self.expect(TokenKind::OpBitOr)?;
                Ok(Expression::Absolute(Box::new(expr)))
            }
            TokenKind::SquareBracketOpen => {
                let mut items = Vec::new();
                if *self.peek() != TokenKind::SquareBracketClose {
                    loop {
                        items.push(self.parse_expr(0)?);
                        if *self.peek() == TokenKind::Comma {
                            self.next_token();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::SquareBracketClose)?;
                Ok(Expression::ArrayInit(items))
            }
            t => Err(ParserError::Generic(format!(
                "Unexpected prefix token: {:?}",
                t
            ))),
        }
    }

    fn infix_binding_power(&self, kind: &TokenKind) -> Option<(u8, u8)> {
        match kind {
            TokenKind::Assign => Some((2, 1)),
            TokenKind::OpOr => Some((3, 4)),
            TokenKind::OpAnd => Some((5, 6)),
            TokenKind::OpBitOr => Some((7, 8)),
            TokenKind::OpXor => Some((9, 10)),
            TokenKind::OpBitAnd => Some((11, 12)),
            TokenKind::OpEq | TokenKind::OpNotEq => Some((13, 14)),
            TokenKind::OpLessThan
            | TokenKind::OpLessOrEq
            | TokenKind::OpGreaterThan
            | TokenKind::OpGreaterOrEq => Some((15, 16)),
            // Removed: shift operators (<<, >>) are only used for cout/cin syntax
            TokenKind::OpAdd | TokenKind::OpSub => Some((19, 20)),
            TokenKind::OpMul | TokenKind::OpDiv | TokenKind::OpRem => Some((21, 22)),
            _ => None,
        }
    }

    fn token_to_op(&self, kind: &TokenKind) -> Ops {
        match kind {
            TokenKind::Assign => Ops::Assign,
            TokenKind::OpAdd => Ops::Add,
            TokenKind::OpSub => Ops::Sub,
            TokenKind::OpMul => Ops::Mul,
            TokenKind::OpDiv => Ops::Div,
            TokenKind::OpRem => Ops::Rem,
            TokenKind::OpEq => Ops::Eq,
            TokenKind::OpNotEq => Ops::NotEq,
            TokenKind::OpLessThan => Ops::Less,
            TokenKind::OpLessOrEq => Ops::LessEq,
            TokenKind::OpGreaterThan => Ops::Greater,
            TokenKind::OpGreaterOrEq => Ops::GreaterEq,
            TokenKind::OpShiftLeft => Ops::Add, // unused, only for cout/cin
            TokenKind::OpShiftRight => Ops::Add, // unused, only for cout/cin
            TokenKind::OpAnd => Ops::And,
            TokenKind::OpOr => Ops::Or,
            TokenKind::OpBitAnd => Ops::BitAnd,
            TokenKind::OpBitOr => Ops::BitOr,
            TokenKind::OpXor => Ops::Xor,
            _ => unreachable!(),
        }
    }
}

pub fn optimize_tree(expr: Expression) -> Expression {
    match expr {
        Expression::Operation(op, args) => {
            let optimized_args: Vec<Expression> = args.into_iter().map(optimize_tree).collect();

            if let [
                Expression::Atom(Value::Integer(l)),
                Expression::Atom(Value::Integer(r)),
            ] = optimized_args.as_slice()
            {
                match op {
                    Ops::Add => return Expression::Atom(Value::Integer(l + r)),
                    Ops::Sub => return Expression::Atom(Value::Integer(l - r)),
                    Ops::Mul => return Expression::Atom(Value::Integer(l * r)),
                    Ops::Div => {
                        if *r != 0 {
                            return Expression::Atom(Value::Integer(l / r));
                        }
                    }
                    Ops::Rem => {
                        if *r != 0 {
                            return Expression::Atom(Value::Integer(l % r));
                        }
                    }
                    _ => {}
                }
            }

            if let [Expression::Atom(Value::Integer(v))] = optimized_args.as_slice()
                && let Ops::Neg = op
            {
                return Expression::Atom(Value::Integer(-v));
            }

            Expression::Operation(op, optimized_args)
        }

        Expression::Block(stmts) => {
            let opt_stmts = stmts.into_iter().map(optimize_tree).collect();
            Expression::Block(opt_stmts)
        }

        Expression::Cout(exprs) => {
            let opt_exprs = exprs.into_iter().map(optimize_tree).collect();
            Expression::Cout(opt_exprs)
        }

        Expression::While { cond, body } => Expression::While {
            cond: Box::new(optimize_tree(*cond)),
            body: Box::new(optimize_tree(*body)),
        },
        Expression::For {
            init,
            cond,
            step,
            body,
        } => Expression::For {
            init: Box::new(optimize_tree(*init)),
            cond: Box::new(optimize_tree(*cond)),
            step: Box::new(optimize_tree(*step)),
            body: Box::new(optimize_tree(*body)),
        },

        Expression::VarDecl {
            typ,
            name,
            init,
            size,
        } => Expression::VarDecl {
            typ,
            name,
            init: init.map(|e| Box::new(optimize_tree(*e))),
            size: size.map(|e| Box::new(optimize_tree(*e))),
        },

        Expression::If {
            cond,
            then_br,
            else_br,
        } => Expression::If {
            cond: Box::new(optimize_tree(*cond)),
            then_br: Box::new(optimize_tree(*then_br)),
            else_br: else_br.map(|e| Box::new(optimize_tree(*e))),
        },

        Expression::Return(e) => Expression::Return(e.map(|x| Box::new(optimize_tree(*x)))),
        Expression::Cin(exprs) => Expression::Cin(exprs.into_iter().map(optimize_tree).collect()),
        Expression::Absolute(e) => Expression::Absolute(Box::new(optimize_tree(*e))),
        Expression::ArrayIndex { arr, index } => Expression::ArrayIndex {
            arr: Box::new(optimize_tree(*arr)),
            index: Box::new(optimize_tree(*index)),
        },
        Expression::ArrayInit(items) => {
            Expression::ArrayInit(items.into_iter().map(optimize_tree).collect())
        }
        Expression::FuncCall { caller, args } => Expression::FuncCall {
            caller: Box::new(optimize_tree(*caller)),
            args: args.into_iter().map(optimize_tree).collect(),
        },
        Expression::FuncDecl {
            name,
            args,
            ret_type,
            body,
        } => Expression::FuncDecl {
            name,
            args,
            ret_type,
            body: Box::new(optimize_tree(*body)),
        },
        Expression::Atom(_) => expr,
    }
}

use std::fmt;

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();
        s.push_str(".\n");
        self.build_string(&mut s, "", true);
        write!(f, "{}", s)
    }
}

impl Expression {
    fn build_string(&self, s: &mut String, prefix: &str, is_last: bool) {
        let branch = if is_last { "└── " } else { "├── " };
        let cont = if is_last { "    " } else { "│   " };
        let ch_prefix = format!("{}{}", prefix, cont);

        match self {
            Expression::Atom(v) => {
                s.push_str(&format!("{}{}Atom: {:?}\n", prefix, branch, v));
            }
            Expression::Absolute(e) => {
                s.push_str(&format!("{}{}Absolute (|...|)\n", prefix, branch));
                e.build_string(s, &ch_prefix, true);
            }
            Expression::Operation(op, ch) => {
                s.push_str(&format!("{}{}Op: {:?}\n", prefix, branch, op));
                for (i, c) in ch.iter().enumerate() {
                    c.build_string(s, &ch_prefix, i == ch.len() - 1);
                }
            }
            Expression::Block(stmts) => {
                s.push_str(&format!("{}{}Block\n", prefix, branch));
                for (i, stmt) in stmts.iter().enumerate() {
                    stmt.build_string(s, &ch_prefix, i == stmts.len() - 1);
                }
            }
            Expression::VarDecl {
                typ,
                name,
                init,
                size,
            } => {
                s.push_str(&format!("{}{}VarDecl: {} {}\n", prefix, branch, typ, name));
                if let Some(sz) = size {
                    s.push_str(&format!("{}    Size:\n", prefix));
                    sz.build_string(s, &ch_prefix, init.is_none());
                }
                if let Some(i) = init {
                    s.push_str(&format!("{}    Init:\n", prefix));
                    i.build_string(s, &ch_prefix, true);
                }
            }
            Expression::FuncDecl {
                name,
                args,
                ret_type,
                body,
            } => {
                s.push_str(&format!(
                    "{}{}Fn: {} (Args: {:?}) -> {:?}\n",
                    prefix, branch, name, args, ret_type
                ));
                body.build_string(s, &ch_prefix, true);
            }
            Expression::FuncCall { caller, args } => {
                s.push_str(&format!("{}{}Call\n", prefix, branch));
                caller.build_string(s, &ch_prefix, args.is_empty());
                for (i, a) in args.iter().enumerate() {
                    a.build_string(s, &ch_prefix, i == args.len() - 1);
                }
            }
            Expression::If {
                cond,
                then_br,
                else_br,
            } => {
                s.push_str(&format!("{}{}If\n", prefix, branch));
                cond.build_string(s, &ch_prefix, false);
                then_br.build_string(s, &ch_prefix, else_br.is_none());
                if let Some(e) = else_br {
                    e.build_string(s, &ch_prefix, true);
                }
            }
            Expression::While { cond, body } => {
                s.push_str(&format!("{}{}While\n", prefix, branch));
                cond.build_string(s, &ch_prefix, false);
                body.build_string(s, &ch_prefix, true);
            }
            Expression::For {
                init,
                cond,
                step,
                body,
            } => {
                s.push_str(&format!("{}{}For\n", prefix, branch));
                init.build_string(s, &ch_prefix, false);
                cond.build_string(s, &ch_prefix, false);
                step.build_string(s, &ch_prefix, false);
                body.build_string(s, &ch_prefix, true);
            }
            Expression::ArrayInit(items) => {
                s.push_str(&format!("{}{}ArrayInit\n", prefix, branch));
                for (i, c) in items.iter().enumerate() {
                    c.build_string(s, &ch_prefix, i == items.len() - 1);
                }
            }
            Expression::ArrayIndex { arr, index } => {
                s.push_str(&format!("{}{}Index Access\n", prefix, branch));
                arr.build_string(s, &ch_prefix, false);
                index.build_string(s, &ch_prefix, true);
            }
            Expression::Cout(items) => {
                s.push_str(&format!("{}{}Cout\n", prefix, branch));
                for (i, c) in items.iter().enumerate() {
                    c.build_string(s, &ch_prefix, i == items.len() - 1);
                }
            }
            Expression::Cin(items) => {
                s.push_str(&format!("{}{}Cin\n", prefix, branch));
                for (i, c) in items.iter().enumerate() {
                    c.build_string(s, &ch_prefix, i == items.len() - 1);
                }
            }
            Expression::Return(e) => {
                s.push_str(&format!("{}{}Return\n", prefix, branch));
                if let Some(ex) = e {
                    ex.build_string(s, &ch_prefix, true);
                }
            }
        }
    }
}
