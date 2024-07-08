use core::fmt;

#[derive(Debug)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug)]
pub struct AlthreadError {
    pos: Pos,
    message: String,
    error_type: ErrorType,
}

#[derive(Debug)]
pub enum ErrorType {
    SyntaxError,
    TypeError,
    VariableError,
    RuntimeError,
}

impl fmt::Display for ErrorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorType::SyntaxError => write!(f, "Syntax Error"),
            ErrorType::TypeError => write!(f, "Type Error"),
            ErrorType::VariableError => write!(f, "Variable Error"),
            ErrorType::RuntimeError => write!(f, "Runtime Error"),
        }
    }
}

impl AlthreadError {
    pub fn error(error_type: ErrorType, line: usize, col: usize, message: String) -> Self {
        Self {
            pos: Pos { line, col },
            message,
            error_type,
        }
    }

    pub fn print_err_line(&self, input: &str) {
        if self.pos.line == 0 {
            return;
        }
        let line = match input.lines().nth(self.pos.line - 1) {
            Some(line) => line.to_string(),
            None => return,
        };

        let line_indent = " ".repeat(self.pos.line.to_string().len());
        eprintln!("{} |", line_indent);
        eprintln!("{} | {}", self.pos.line, line);
        eprintln!("{} |{}^---", line_indent, " ".repeat(self.pos.col));
        eprintln!("{} |", line_indent);
    }

    pub fn report(&self, input: &str) {
        eprintln!("Error at {}:{}", self.pos.line, self.pos.col);
        self.print_err_line(input);
        eprintln!("{}: {}", self.error_type, self.message);
    }
}
