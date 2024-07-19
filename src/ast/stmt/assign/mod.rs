pub mod assign_binary;
pub mod assign_unary;

use assign_binary::AssignBinary;
use assign_unary::AssignUnary;
use pest::iterators::Pair;

use crate::{env::Environment, error::AlthreadError, parser::Rule};

#[derive(Debug)]
pub enum Assign {
    Unary(AssignUnary),
    Binary(AssignBinary),
}

impl Assign {
    pub fn build(pair: Pair<Rule>, env: &Environment) -> Result<Self, AlthreadError> {
        let pair = pair.into_inner().next().unwrap();
        Ok(match pair.as_rule() {
            Rule::assign_unary => Self::Unary(AssignUnary::build(pair, env)?),
            Rule::assign_binary => Self::Binary(AssignBinary::build(pair, env)?),
            _ => unreachable!(),
        })
    }
}