use pest::iterators::Pairs;

use crate::{env::Environment, error::AlthreadError, parser::Rule};

use super::block::Block;

#[derive(Debug)]
pub struct Program {
    pub main_block: Option<Block>,
    pub shared_block: Option<Block>,
    pub always_block: Option<Block>,
    pub line: usize,
    pub column: usize,
}

impl Program {
    pub fn build(pairs: Pairs<Rule>, env: &mut Environment) -> Result<Self, AlthreadError> {
        let (line, column) = pairs.clone().next().unwrap().line_col();
        let mut program = Self {
            main_block: None,
            shared_block: None,
            always_block: None,
            line,
            column,
        };

        for pair in pairs {
            match pair.as_rule() {
                Rule::main_block => {
                    program.main_block = Some(Block::parse_and_push(pair, env)?);
                }
                Rule::shared_block => {
                    program.shared_block = Some(Block::parse(pair, env)?);
                }
                Rule::always_block => {
                    // TODO : implement always block
                    unimplemented!();
                }
                Rule::EOI => break,
                rule => unreachable!("{:?}", rule),
            }
        }

        Ok(program)
    }
}
