use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
    },
    compiler::{CompilerState, InstructionBuilderOk},
    error::AlthreadResult,
    parser::Rule,
    vm::instruction::{self, Instruction, InstructionType, JumpControl},
};

use super::Statement;


#[derive(Debug, Clone)]
pub struct LoopControl {
    pub statement: Box<Node<Statement>>,
}

impl NodeBuilder for LoopControl {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let statement = Box::new(Node::build(pairs.next().unwrap())?);

        Ok(Self { statement })
    }
}

impl InstructionBuilder for Node<LoopControl> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        let stack_len = state.program_stack.len();

        let mut builder = self.value.statement.as_ref().compile(state)?;

        builder.instructions.push(Instruction {
            pos: Some(self.value.statement.as_ref().pos),
            control: InstructionType::Jump(JumpControl {
                jump: -(builder.instructions.len() as i64),
            }),
        });
        if builder.contains_jump() {
            for idx in builder.break_indexes.get("").unwrap_or(&Vec::new()) {
                if let InstructionType::Break(bc) =  &builder.instructions[*idx as usize].control {
                    builder.instructions[*idx as usize].control = InstructionType::Break(instruction::BreakLoopControl {
                        jump: (builder.instructions.len() - idx) as i64,
                        unstack_len: bc.unstack_len - stack_len,
                    });
                }
                else {
                    panic!("Expected Break instruction");
                }
            }
            builder.break_indexes.remove("");
            for idx in builder.continue_indexes.get("").unwrap_or(&Vec::new()) {
                if let InstructionType::Break(bc) =  &builder.instructions[*idx as usize].control {
                    builder.instructions[*idx as usize].control = InstructionType::Break(instruction::BreakLoopControl {
                        jump: -(*idx as i64),
                        unstack_len: bc.unstack_len - stack_len,
                    });
                }
                else {
                    panic!("Expected Break instruction");
                }
            }
            builder.continue_indexes.remove("");
        }
        Ok(builder)
    }
}

impl AstDisplay for LoopControl {
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
