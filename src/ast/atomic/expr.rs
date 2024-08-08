use std::iter::Rev;

use pest::iterators::{Pair, Pairs};

use crate::{
    ast::node::Node,
    env::{symbol_table::SymbolTable, value::Value, Environment},
    error::{AlthreadError, AlthreadResult, ErrorType},
    no_rule,
    parser::Rule,
};

pub fn consume_condition(
    node: &Node,
    symbol_table: &SymbolTable,
    env: &Environment,
) -> AlthreadResult<bool> {
    Ok(match node {
        Node::Atomic(atomic) => match atomic.pair.as_rule() {
            Rule::expr => {
                println!("{:?}", atomic.pair.as_str());
                let val = consume_expr(atomic.pair.clone(), symbol_table, env)?;
                val.is_true()
            }
            _ => return Err(no_rule!(atomic.pair)),
        },
        _ => unreachable!(),
    })
}

pub fn consume_expr<'a>(
    pair: Pair<Rule>,
    symbol_table: &SymbolTable,
    env: &Environment,
) -> AlthreadResult<Value> {
    match pair.as_rule() {
        Rule::primary => Ok(consume_primary(
            pair.into_inner().next().unwrap(),
            symbol_table,
            env,
        )?),
        Rule::unary => Ok(consume_unary(pair.into_inner(), symbol_table, env)?),
        Rule::expr
        | Rule::logical_or
        | Rule::logical_and
        | Rule::equality
        | Rule::comparison
        | Rule::term
        | Rule::factor => Ok(consume_binary(pair.into_inner().rev(), symbol_table, env)?),
        _ => {
            return Err(no_rule!(pair));
        }
    }
}

fn consume_primary(
    pair: Pair<Rule>,
    symbol_table: &SymbolTable,
    env: &Environment,
) -> AlthreadResult<Value> {
    let val = pair.as_str();

    Ok(match pair.as_rule() {
        Rule::NULL => Value::Null,
        Rule::BOOLEAN => Value::Bool(val == "true"),
        Rule::INTEGER => Value::Int(val.parse::<i64>().unwrap()),
        Rule::FLOAT => Value::Float(val.parse::<f64>().unwrap()),
        Rule::STRING => Value::String(val.to_string()),
        Rule::IDENTIFIER => {
            let symbol = symbol_table.get(env, &pair)?;
            symbol.value.clone()
        }
        Rule::expr => consume_expr(pair, symbol_table, env)?,
        _ => return Err(no_rule!(pair)),
    })
}

fn consume_unary<'a>(
    mut pairs: Pairs<'a, Rule>,
    symbol_table: &SymbolTable,
    env: &Environment,
) -> AlthreadResult<Value> {
    let pair: Pair<'a, Rule> = pairs.next().unwrap();
    if let Some(val) = pairs.next() {
        let val = consume_expr(val, symbol_table, env)?;
        let op = pair;
        Ok(match op.as_str() {
            "+" => Ok(val),
            "-" => val.neg(),
            "!" => val.not(),
            _ => return Err(no_rule!(op)),
        }
        .map_err(|e| {
            AlthreadError::new(
                ErrorType::ArithmeticError,
                op.line_col().0,
                op.line_col().1,
                e.to_string(),
            )
        })?)
    } else {
        Ok(consume_expr(pair, symbol_table, env)?)
    }
}

fn consume_binary<'a>(
    mut pairs: Rev<Pairs<'a, Rule>>,
    symbol_table: &SymbolTable,
    env: &Environment,
) -> AlthreadResult<Value> {
    let right_value = consume_expr(pairs.next().unwrap(), symbol_table, env)?;
    if let Some(op) = pairs.next() {
        let left_value = consume_binary(pairs, symbol_table, env)?;

        Ok(match op.as_str() {
            "+" => left_value.add(&right_value),
            "-" => left_value.sub(&right_value),
            "*" => left_value.mul(&right_value),
            "/" => left_value.div(&right_value),
            "%" => left_value.rem(&right_value),
            "==" => left_value.eq(&right_value),
            "!=" => left_value.ne(&right_value),
            "<" => left_value.lt(&right_value),
            "<=" => left_value.le(&right_value),
            ">" => left_value.gt(&right_value),
            ">=" => left_value.ge(&right_value),
            "&&" => left_value.and(&right_value),
            "||" => left_value.or(&right_value),
            _ => return Err(no_rule!(op)),
        }
        .map_err(|e| {
            AlthreadError::new(
                ErrorType::ArithmeticError,
                op.line_col().0,
                op.line_col().1,
                e.to_string(),
            )
        })?)
    } else {
        Ok(right_value)
    }
}
