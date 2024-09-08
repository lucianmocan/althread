use std::{collections::HashSet, fmt};

use pest::iterators::Pair;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::Node,
        token::{datatype::DataType, literal::Literal, unary_operator::UnaryOperator},
    }, compiler::{CompilerState, Variable}, error::{AlthreadError, AlthreadResult, ErrorType, Pos}, parser::Rule
};

use super::{Expression, LocalExpressionNode};

#[derive(Debug)]
pub struct UnaryExpression {
    pub operator: Node<UnaryOperator>,
    pub operand: Box<Node<Expression>>,
}

#[derive(Debug)]
pub struct LocalUnaryExpressionNode {
    pub operator: UnaryOperator,
    pub operand: Box<LocalExpressionNode>,
}

impl UnaryExpression {
    pub fn build(operator: Pair<Rule>, operand: Node<Expression>) -> AlthreadResult<Node<Self>> {
        Ok(Node {
            pos: Pos {
                line: operator.line_col().0,
                col: operator.line_col().1,
                start: operator.as_span().start(),
                end: operand.pos.end,
            },
            value: Self {
                operator: Node::build(operator)?,
                operand: Box::new(operand),
            },
        })
    }
}

impl LocalUnaryExpressionNode {
    pub fn from_unary(un_expression: &UnaryExpression, program_stack: &Vec<Variable>) -> AlthreadResult<Self> {
        let e = LocalExpressionNode::from_expression(&un_expression.operand.value, program_stack)?;
        Ok(Self {
            operator: un_expression.operator.value.clone(),
            operand: Box::new(e),
        })
    }

    pub fn datatype(&self, state: &CompilerState) -> Result<DataType, String> {
        let operand_type = self.operand.as_ref().datatype(state)?;
        match self.operator {
            UnaryOperator::Positive => if operand_type.is_a_number() {
                Ok(operand_type)
            } else { Err("Can only apply operator '+' on a number".to_string()) },
            UnaryOperator::Negative =>  if operand_type.is_a_number() {
                Ok(operand_type)
            } else { Err("Can only apply operator '-' on a number".to_string()) },
            UnaryOperator::Not =>  if operand_type.is_boolean() {
                Ok(operand_type)
            } else { Err("Can only apply operator '!' on a boolean".to_string()) },
        }
    }
}

impl UnaryExpression {
    pub fn get_vars(&self, vars: &mut HashSet<String>) {
        self.operand.value.get_vars(vars);
    }
}

impl AstDisplay for UnaryExpression {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{}unary_expr", prefix)?;
        let prefix = &prefix.add_branch();
        writeln!(f, "{}op: {}", prefix, self.operator)?;

        let prefix = &prefix.switch();
        writeln!(f, "{}expr", prefix)?;
        self.operand.ast_fmt(f, &prefix.add_leaf())?;

        Ok(())
    }
}
