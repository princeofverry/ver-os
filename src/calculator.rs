use alloc::vec::Vec;
use alloc::string::String;
use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref VARIABLES: Mutex<alloc::collections::BTreeMap<String, i64>> = Mutex::new(alloc::collections::BTreeMap::new());
}

pub fn set_var(name: String, val: i64) {
    VARIABLES.lock().insert(name, val);
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(i64),
    Plus,
    Minus,
    Multiply,
    Divide,
    LParen,
    RParen,
}

pub fn eval(input: &str) -> Result<i64, &'static str> {
    let tokens = tokenize(input)?;
    let mut parser = Parser::new(tokens);
    parser.parse()
}

fn tokenize(input: &str) -> Result<Vec<Token>, &'static str> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\t' => { chars.next(); },
            '+' => { tokens.push(Token::Plus); chars.next(); },
            '-' => { tokens.push(Token::Minus); chars.next(); },
            '*' => { tokens.push(Token::Multiply); chars.next(); },
            '/' => { tokens.push(Token::Divide); chars.next(); },
            '(' => { tokens.push(Token::LParen); chars.next(); },
            ')' => { tokens.push(Token::RParen); chars.next(); },
            '0'..='9' => {
                let mut num = 0;
                while let Some(&('0'..='9')) = chars.peek() {
                    num = num * 10 + (chars.next().unwrap() as i64 - '0' as i64);
                }
                tokens.push(Token::Number(num));
            },
            'a'..='z' | 'A'..='Z' => {
                let mut name = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_alphanumeric() {
                        name.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                let val = VARIABLES.lock().get(&name.to_lowercase()).copied().unwrap_or(0);
                tokens.push(Token::Number(val));
            },
            _ => return Err("Invalid character in expression"),
        }
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let tok = self.peek().cloned();
        self.pos += 1;
        tok
    }

    fn parse(&mut self) -> Result<i64, &'static str> {
        let res = self.parse_expr()?;
        if self.pos < self.tokens.len() {
            Err("Unexpected trailing characters")
        } else {
            Ok(res)
        }
    }

    fn parse_expr(&mut self) -> Result<i64, &'static str> {
        let mut left = self.parse_term()?;

        while let Some(tok) = self.peek() {
            match tok {
                Token::Plus => {
                    self.next();
                    left += self.parse_term()?;
                }
                Token::Minus => {
                    self.next();
                    left -= self.parse_term()?;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<i64, &'static str> {
        let mut left = self.parse_factor()?;

        while let Some(tok) = self.peek() {
            match tok {
                Token::Multiply => {
                    self.next();
                    left *= self.parse_factor()?;
                }
                Token::Divide => {
                    self.next();
                    let right = self.parse_factor()?;
                    if right == 0 {
                        return Err("Division by zero");
                    }
                    left /= right;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<i64, &'static str> {
        match self.next() {
            Some(Token::Number(n)) => Ok(n),
            Some(Token::LParen) => {
                let expr = self.parse_expr()?;
                if let Some(Token::RParen) = self.next() {
                    Ok(expr)
                } else {
                    Err("Expected closing parenthesis")
                }
            }
            Some(Token::Minus) => {
                Ok(-self.parse_factor()?)
            }
            _ => Err("Expected number or parenthesis"),
        }
    }
}