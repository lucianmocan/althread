use std::fmt;

use pest::iterators::Pair;

use crate::{
    ast::{
        display::AstDisplay,
        node::{Build, Node},
        token::{datatype::DataType, decl_keyword::DeclKeyword, identifier::Identifier},
    },
    error::AlthreadResult,
    no_rule,
    parser::Rule,
    write_indent,
};

use super::expr::Expr;

#[derive(Debug)]
pub struct Decl {
    pub keyword: Node<DeclKeyword>,
    pub identifier: Node<Identifier>,
    pub datatype: Option<Node<DataType>>,
    pub value: Option<Node<Expr>>,
}

impl Build for Decl {
    fn build(pair: Pair<Rule>) -> AlthreadResult<Self> {
        let mut pairs = pair.into_inner();

        let keyword = Node::build(pairs.next().unwrap())?;
        let identifier = Node::build(pairs.next().unwrap())?;
        let mut datatype = None;
        let mut value = None;

        for pair in pairs {
            match pair.as_rule() {
                Rule::DATATYPE => {
                    datatype = Some(Node::build(pair)?);
                }
                Rule::expr => {
                    value = Some(Node::build(pair)?);
                }
                _ => return Err(no_rule!(pair)),
            }
        }

        Ok(Self {
            keyword,
            identifier,
            datatype,
            value,
        })
    }
}

impl AstDisplay for Decl {
    fn ast_fmt(&self, f: &mut fmt::Formatter, indent_level: usize) -> fmt::Result {
        write_indent!(f, indent_level, "decl")?;
        write_indent!(f, indent_level + 1, "keyword: {}", self.keyword)?;
        write_indent!(f, indent_level + 1, "ident: {}", self.identifier)?;

        if let Some(datatype) = &self.datatype {
            write_indent!(f, indent_level + 1, "datatype: {}", datatype)?;
        }

        if let Some(value) = &self.value {
            value.ast_fmt(f, indent_level + 1)?;
        }

        Ok(())
    }
}