use pest::iterators::Pair;

use crate::{
    env::Environment,
    error::{AlthreadError, ErrorType},
    nodes::{
        datatype::DataType,
        expr::{primary::PrimaryExpr, Expr, ExprKind},
    },
    parser::Rule,
};

use super::ExprResult;

impl PrimaryExpr {
    pub fn build(pair: Pair<Rule>, env: &Environment) -> ExprResult {
        // parenthesis priority
        if pair.as_rule() == Rule::expr {
            return Expr::build(pair, env);
        }

        let expr = match pair.as_rule() {
            Rule::NULL => Self::Null,
            Rule::INTEGER => Self::Int(pair.as_str().parse::<i64>().unwrap()),
            Rule::FLOAT => Self::Float(pair.as_str().parse::<f64>().unwrap()),
            Rule::BOOLEAN => Self::Bool(pair.as_str() == "true"),
            Rule::STRING => Self::String(pair.as_str().to_string()),
            Rule::IDENTIFIER => Self::Identifier(pair.as_str().to_string()),
            rule => unreachable!("{:?}", rule),
        };

        Ok(Expr {
            kind: ExprKind::Primary(expr),
            line: pair.as_span().start_pos().line_col().0,
            column: pair.as_span().start_pos().line_col().1,
        })
    }

    pub fn get_datatype(&self, env: &Environment) -> Result<DataType, AlthreadError> {
        match self {
            PrimaryExpr::Int(_) => Ok(DataType::Int),
            PrimaryExpr::Float(_) => Ok(DataType::Float),
            PrimaryExpr::Bool(_) => Ok(DataType::Bool),
            PrimaryExpr::String(_) => Ok(DataType::String),
            PrimaryExpr::Null => Ok(DataType::Void),
            PrimaryExpr::Identifier(ident) => {
                // TODO : implement error with line and col
                let symbol = env
                    .get_symbol(ident)
                    .map_err(|e| AlthreadError::error(ErrorType::TypeError, 0, 0, e))?;
                Ok(symbol.datatype.clone())
            }
        }
    }
}