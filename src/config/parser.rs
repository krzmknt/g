use crate::error::{Error, Result};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<Value>),
    Table(HashMap<String, Value>),
}

pub fn parse(input: &str) -> Result<HashMap<String, Value>> {
    let mut parser = Parser::new(input);
    parser.parse_document()
}

struct Parser<'a> {
    input: &'a str,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    current_table: Vec<String>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.char_indices().peekable(),
            current_table: Vec::new(),
        }
    }

    fn parse_document(&mut self) -> Result<HashMap<String, Value>> {
        let mut root = HashMap::new();

        loop {
            self.skip_whitespace_and_comments();

            if self.peek().is_none() {
                break;
            }

            if self.peek() == Some('[') {
                self.current_table = self.parse_table_header()?;
            } else if self
                .peek()
                .map(|c| c.is_alphanumeric() || c == '_' || c == '"')
                .unwrap_or(false)
            {
                let (key, value) = self.parse_key_value()?;
                self.insert_value(&mut root, &self.current_table.clone(), &key, value);
            } else {
                self.advance();
            }
        }

        Ok(root)
    }

    fn parse_table_header(&mut self) -> Result<Vec<String>> {
        self.expect('[')?;
        let mut path = Vec::new();

        loop {
            self.skip_whitespace();
            let key = self.parse_key()?;
            path.push(key);

            self.skip_whitespace();
            match self.peek() {
                Some('.') => {
                    self.advance();
                }
                Some(']') => {
                    self.advance();
                    break;
                }
                _ => return Err(Error::Config("Invalid table header".to_string())),
            }
        }

        self.skip_to_newline();
        Ok(path)
    }

    fn parse_key_value(&mut self) -> Result<(String, Value)> {
        let key = self.parse_key()?;
        self.skip_whitespace();
        self.expect('=')?;
        self.skip_whitespace();
        let value = self.parse_value()?;
        self.skip_to_newline();
        Ok((key, value))
    }

    fn parse_key(&mut self) -> Result<String> {
        if self.peek() == Some('"') {
            self.parse_quoted_key()
        } else {
            self.parse_bare_key()
        }
    }

    fn parse_bare_key(&mut self) -> Result<String> {
        let mut key = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                key.push(c);
                self.advance();
            } else {
                break;
            }
        }
        if key.is_empty() {
            return Err(Error::Config("Empty key".to_string()));
        }
        Ok(key)
    }

    fn parse_quoted_key(&mut self) -> Result<String> {
        self.expect('"')?;
        let mut key = String::new();
        while let Some(c) = self.peek() {
            if c == '"' {
                self.advance();
                return Ok(key);
            }
            if c == '\\' {
                self.advance();
                if let Some(escaped) = self.peek() {
                    key.push(self.parse_escape_char(escaped)?);
                    self.advance();
                }
            } else {
                key.push(c);
                self.advance();
            }
        }
        Err(Error::Config("Unterminated quoted key".to_string()))
    }

    fn parse_value(&mut self) -> Result<Value> {
        match self.peek() {
            Some('"') => self.parse_string(),
            Some('[') => self.parse_array(),
            Some('{') => self.parse_inline_table(),
            Some('t') | Some('f') => self.parse_boolean(),
            Some(c) if c.is_ascii_digit() || c == '-' || c == '+' => self.parse_number(),
            _ => Err(Error::Config("Invalid value".to_string())),
        }
    }

    fn parse_string(&mut self) -> Result<Value> {
        self.expect('"')?;
        let mut s = String::new();

        while let Some(c) = self.peek() {
            if c == '"' {
                self.advance();
                return Ok(Value::String(s));
            }
            if c == '\\' {
                self.advance();
                if let Some(escaped) = self.peek() {
                    s.push(self.parse_escape_char(escaped)?);
                    self.advance();
                }
            } else {
                s.push(c);
                self.advance();
            }
        }

        Err(Error::Config("Unterminated string".to_string()))
    }

    fn parse_escape_char(&self, c: char) -> Result<char> {
        match c {
            'n' => Ok('\n'),
            't' => Ok('\t'),
            'r' => Ok('\r'),
            '\\' => Ok('\\'),
            '"' => Ok('"'),
            _ => Err(Error::Config(format!("Invalid escape sequence: \\{}", c))),
        }
    }

    fn parse_number(&mut self) -> Result<Value> {
        let mut num_str = String::new();
        let mut is_float = false;

        if self.peek() == Some('-') || self.peek() == Some('+') {
            num_str.push(self.peek().unwrap());
            self.advance();
        }

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                num_str.push(c);
                self.advance();
            } else if c == '.' && !is_float {
                is_float = true;
                num_str.push(c);
                self.advance();
            } else if c == 'e' || c == 'E' {
                is_float = true;
                num_str.push(c);
                self.advance();
                if self.peek() == Some('-') || self.peek() == Some('+') {
                    num_str.push(self.peek().unwrap());
                    self.advance();
                }
            } else if c == '_' {
                self.advance(); // Skip underscores in numbers
            } else {
                break;
            }
        }

        if is_float {
            num_str
                .parse::<f64>()
                .map(Value::Float)
                .map_err(|_| Error::Config(format!("Invalid float: {}", num_str)))
        } else {
            num_str
                .parse::<i64>()
                .map(Value::Integer)
                .map_err(|_| Error::Config(format!("Invalid integer: {}", num_str)))
        }
    }

    fn parse_boolean(&mut self) -> Result<Value> {
        if self.try_consume("true") {
            Ok(Value::Boolean(true))
        } else if self.try_consume("false") {
            Ok(Value::Boolean(false))
        } else {
            Err(Error::Config("Invalid boolean".to_string()))
        }
    }

    fn parse_array(&mut self) -> Result<Value> {
        self.expect('[')?;
        let mut arr = Vec::new();

        loop {
            self.skip_whitespace_and_comments();

            if self.peek() == Some(']') {
                self.advance();
                break;
            }

            let value = self.parse_value()?;
            arr.push(value);

            self.skip_whitespace_and_comments();
            match self.peek() {
                Some(',') => {
                    self.advance();
                }
                Some(']') => {}
                _ => return Err(Error::Config("Invalid array".to_string())),
            }
        }

        Ok(Value::Array(arr))
    }

    fn parse_inline_table(&mut self) -> Result<Value> {
        self.expect('{')?;
        let mut table = HashMap::new();

        loop {
            self.skip_whitespace();

            if self.peek() == Some('}') {
                self.advance();
                break;
            }

            let (key, value) = self.parse_key_value()?;
            table.insert(key, value);

            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.advance();
                }
                Some('}') => {}
                _ => return Err(Error::Config("Invalid inline table".to_string())),
            }
        }

        Ok(Value::Table(table))
    }

    fn insert_value(
        &self,
        root: &mut HashMap<String, Value>,
        path: &[String],
        key: &str,
        value: Value,
    ) {
        let mut current = root;

        for segment in path {
            current = match current
                .entry(segment.clone())
                .or_insert_with(|| Value::Table(HashMap::new()))
            {
                Value::Table(t) => t,
                _ => return, // Invalid path
            };
        }

        current.insert(key.to_string(), value);
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, c)| *c)
    }

    fn advance(&mut self) -> Option<char> {
        self.chars.next().map(|(_, c)| c)
    }

    fn expect(&mut self, expected: char) -> Result<()> {
        match self.advance() {
            Some(c) if c == expected => Ok(()),
            Some(c) => Err(Error::Config(format!(
                "Expected '{}', got '{}'",
                expected, c
            ))),
            None => Err(Error::Config(format!("Expected '{}', got EOF", expected))),
        }
    }

    fn try_consume(&mut self, s: &str) -> bool {
        let start = self.chars.clone();
        for expected in s.chars() {
            if self.advance() != Some(expected) {
                self.chars = start;
                return false;
            }
        }
        true
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            self.skip_whitespace();

            match self.peek() {
                Some('#') => {
                    self.skip_to_newline();
                }
                Some('\n') | Some('\r') => {
                    self.advance();
                }
                _ => break,
            }
        }
    }

    fn skip_to_newline(&mut self) {
        while let Some(c) = self.peek() {
            if c == '\n' || c == '\r' {
                break;
            }
            self.advance();
        }
    }
}
