pub mod binary_assignment;

use std::fmt::{self};

use binary_assignment::BinaryAssignment;
use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        token::literal::Literal,
    }, compiler::CompilerState, vm::{instruction::Instruction}, error::{AlthreadError, AlthreadResult, ErrorType}, no_rule, parser::Rule
};

#[derive(Debug)]
pub enum Assignment {
    Binary(Node<BinaryAssignment>),
}

impl NodeBuilder for Assignment {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let pair = pairs.next().unwrap();

        match pair.as_rule() {
            Rule::binary_assignment => Ok(Self::Binary(Node::build(pair)?)),
            Rule::unary_assignment => Err(AlthreadError::new(
                ErrorType::SyntaxError,
                pair.line_col().0,
                pair.line_col().1,
                String::from("Unary assignment is not supported yet"),
            )),
            _ => Err(no_rule!(pair)),
        }
    }
}


impl InstructionBuilder for Assignment {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<Vec<Instruction>> {
        match self {
            Self::Binary(node) => node.compile(state),
        }
    }
}


impl AstDisplay for Assignment {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        match self {
            Self::Binary(node) => node.ast_fmt(f, prefix),
        }
    }
}