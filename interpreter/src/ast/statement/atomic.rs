use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        block, display::{AstDisplay, Prefix}, node::{InstructionBuilder, Node, NodeBuilder}
    },
    compiler::CompilerState,
    error::{AlthreadError, AlthreadResult, ErrorType},
    parser::Rule,
    vm::instruction::{Instruction, InstructionType, JumpControl},
};

use super::{send, Statement};

#[derive(Debug, Clone)]
pub struct Atomic {
    pub statement: Box<Node<Statement>>,
    pub delegated: bool,
}

impl NodeBuilder for Atomic {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let mut statement: Box<Node<Statement>> = Box::new(Node::build(pairs.next().unwrap())?);
        let mut delegated = false;
        
        let mut first_statement = statement.as_mut();

        let start_atomic_lambda = |s: &mut Statement| {
            // if the statement is a wait block then tell it so
            match s {
                Statement::Wait(wait) => { wait.value.start_atomic = true; true },
                Statement::Send(send) => { send.value.start_atomic = true; true},
                _ => false,
            }
        };
        
        if start_atomic_lambda(&mut first_statement.value) {
            delegated = true;
            println!("delegated at level 0");
        } else {
            while let Statement::Block(block) = &mut first_statement.value {
                if let Some(child) = block.value.children.first_mut() {
                    first_statement = child;
                    if start_atomic_lambda(&mut first_statement.value) {
                        delegated = true;
                        println!("delegated at level *");
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        Ok(Self { statement, delegated })
    }
}

impl InstructionBuilder for Node<Atomic> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<Vec<Instruction>> {
        
        if state.is_atomic {
            return Err(AlthreadError::new(
                ErrorType::InstructionNotAllowed,
                Some(self.value.statement.as_ref().pos),
                "Atomic blocks cannot be nested".to_string(),
            ));
        }

        let mut instructions= Vec::new();

        if !self.value.delegated {
            instructions.push(Instruction {
                pos: Some(self.value.statement.as_ref().pos),
                control: InstructionType::AtomicStart,
            });
            state.is_atomic = true;
        }

        instructions.extend(self.value.statement.as_ref().compile(state)?);

        state.is_atomic = false;
        instructions.push(Instruction {
            pos: Some(self.value.statement.as_ref().pos),
            control: InstructionType::AtomicEnd,
        });

        Ok(instructions)
    }
}

impl AstDisplay for Atomic {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}loop_control")?;

        let prefix = prefix.switch();
        writeln!(f, "{prefix}do")?;
        {
            let prefix = prefix.add_leaf();
            self.statement.as_ref().ast_fmt(f, &prefix)?;
        }

        Ok(())
    }
}
