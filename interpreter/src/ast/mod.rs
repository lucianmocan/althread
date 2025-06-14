pub mod block;
pub mod condition_block;
pub mod display;
pub mod node;
pub mod statement;
pub mod token;

use core::panic;
use std::{
    collections::{BTreeMap, HashMap},
    fmt::{self, Formatter},
    rc::Rc,
};

use block::Block;
use condition_block::ConditionBlock;
use display::{AstDisplay, Prefix};
use node::{InstructionBuilder, Node};
use pest::iterators::Pairs;
use statement::Statement;
use token::{args_list::ArgsList, condition_keyword::ConditionKeyword, datatype::DataType, identifier::Identifier};

use crate::{
    compiler::{CompiledProject, CompilerState, FunctionDefinition, Variable},
    error::{AlthreadError, AlthreadResult, ErrorType},
    no_rule,
    parser::Rule,
    vm::{
        instruction::{Instruction, InstructionType, ProgramCode},
        VM,
    },
    analysis::control_flow_graph::ControlFlowGraph,
};

#[derive(Debug)]
pub struct Ast {
    pub process_blocks: HashMap<String, (Node<ArgsList>, Node<Block>)>,
    pub condition_blocks: HashMap<ConditionKeyword, Node<ConditionBlock>>,
    pub global_block: Option<Node<Block>>,
    pub function_blocks: HashMap<String, (Node<ArgsList>, DataType, Node<Block>)>,
}

pub fn check_function_returns(func_name: &str,  func_body: &Node<Block>, return_type: &DataType) -> AlthreadResult<()> {
    if matches!(return_type, DataType::Void) {
        return Ok(());
    }

    let cfg = ControlFlowGraph::from_function(func_body);
    
    // display the control flow graph for debugging
    // cfg.display();


    // we need to return the function at line does not return a value
    // and say on which line it does not return a value
    
    if let Some(missing_return_pos) = cfg.find_first_missing_return_point(func_body.pos) {
        return Err(AlthreadError::new(
            ErrorType::FunctionMissingReturnStatement,
            Some(missing_return_pos), // Use the specific Pos found by the CFG analysis
            format!(
                "Function '{}' does not return a value on all code paths. Problem detected in construct starting at line {}.",
                func_name, missing_return_pos.line
            ),
        ));
    }

    Ok(())
}

impl Ast {
    pub fn new() -> Self {
        Self {
            process_blocks: HashMap::new(),
            condition_blocks: HashMap::new(),
            global_block: None,
            function_blocks: HashMap::new(),
        }
    }
    /// 
    pub fn build(pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let mut ast = Self::new();
        for pair in pairs {
            match pair.as_rule() {
                Rule::main_block => {
                    let mut pairs = pair.into_inner();

                    let main_block = Node::build(pairs.next().unwrap())?;
                    ast.process_blocks
                        .insert("main".to_string(), (Node::<ArgsList>::new(), main_block));
                }
                Rule::global_block => {
                    let mut pairs = pair.into_inner();

                    let global_block = Node::build(pairs.next().unwrap())?;
                    ast.global_block = Some(global_block);
                }
                Rule::condition_block => {
                    let mut pairs = pair.into_inner();

                    let keyword_pair = pairs.next().unwrap();
                    let condition_keyword = match keyword_pair.as_rule() {
                        Rule::ALWAYS_KW => ConditionKeyword::Always,
                        Rule::NEVER_KW => ConditionKeyword::Never,
                        _ => return Err(no_rule!(keyword_pair, "condition keyword")),
                    };
                    let condition_block = Node::build(pairs.next().unwrap())?;
                    ast.condition_blocks
                        .insert(condition_keyword, condition_block);
                }
                Rule::program_block => {
                    let mut pairs = pair.into_inner();

                    let process_identifier = pairs.next().unwrap().as_str().to_string();
                    let args_list: Node<token::args_list::ArgsList> =
                        Node::build(pairs.next().unwrap())?;
                    let program_block = Node::build(pairs.next().unwrap())?;
                    ast.process_blocks
                        .insert(process_identifier, (args_list, program_block));
                }
                Rule::function_block => {
                    let mut pairs  = pair.into_inner();

                    let function_identifier = pairs.next().unwrap().as_str().to_string();
                    
                    let args_list: Node<token::args_list::ArgsList> = Node::build(pairs.next().unwrap())?;
                    pairs.next(); // skip the "->" token
                    let return_datatype = DataType::from_str(pairs.next().unwrap().as_str());
                    
                    let function_block: Node<Block>  = Node::build(pairs.next().unwrap())?;
                    
                    // check if function definition is already defined
                    if ast.function_blocks.contains_key(&function_identifier) {
                        return Err(AlthreadError::new(
                            ErrorType::FunctionAlreadyDefined,
                            Some(function_block.pos),
                            format!("Function '{}' is already defined", function_identifier),
                        ));
                    }

                    ast.function_blocks
                        .insert(
                        function_identifier,
                        (args_list, return_datatype, function_block)
                    );

                }
                Rule::EOI => (),
                _ => return Err(no_rule!(pair, "root ast")),
            }
        }

        Ok(ast)
    }

    pub fn compile(&self) -> AlthreadResult<CompiledProject> {
        // "compile" the "shared" block to retrieve the set of
        // shared variables
        let mut state = CompilerState::new();
        let mut global_memory = BTreeMap::new();
        let mut global_table = HashMap::new();
        state.current_stack_depth = 1;
        state.is_shared = true;
        if let Some(global) = self.global_block.as_ref() {
            let mut memory = VM::new_memory();
            for node in global.value.children.iter() {
                match &node.value {
                    Statement::Declaration(decl) => {
                        let mut literal = None;
                        let node_compiled = node.compile(&mut state)?;
                        for gi in node_compiled.instructions {
                            match gi.control {
                                InstructionType::Expression(exp) => {
                                    literal = Some(exp.eval(&memory).or_else(|err| {
                                        Err(AlthreadError::new(
                                            ErrorType::ExpressionError,
                                            gi.pos,
                                            err,
                                        ))
                                    })?);
                                }
                                InstructionType::Declaration { unstack_len } => {
                                    // do nothing
                                    assert!(unstack_len == 1)
                                }
                                InstructionType::Push(pushed_literal) => {
                                    literal = Some(pushed_literal)
                                }
                                _ => {
                                    panic!("unexpected instruction in compiled declaration statement")
                                }
                            }
                          }
                            let literal = literal
                                .expect("declaration did not compile to expression nor PushNull");
                            memory.push(literal);

                            let var_name = &decl.value.identifier.value.value;
                            global_table.insert(
                                var_name.clone(),
                                state.program_stack.last().unwrap().clone(),
                            );
                            global_memory.insert(var_name.clone(), memory.last().unwrap().clone());
                    }
                    _ => {
                        return Err(AlthreadError::new(
                            ErrorType::InstructionNotAllowed,
                            Some(node.pos),
                            "The 'shared' block can only contains assignment from an expression"
                                .to_string(),
                        ))
                    }
                }
            }
        }

        state.global_table = global_table;

        state.unstack_current_depth();
        assert!(state.current_stack_depth == 0);


        // functions baby ??
        // allow cross-function calls, recursive calls
        // this creates FunctionDefinitions without the compiled body, so that
        // compilation can be done no matter the order of the functions
        // or if they are recursive
        for (func_name, (args_list, return_datatype, func_block)) in &self.function_blocks {
            // check if the function is already defined
            if state.user_functions.contains_key(func_name) {
                return Err(AlthreadError::new(
                    ErrorType::FunctionAlreadyDefined,
                    Some(func_block.pos),
                    format!("Function '{}' is already defined", func_name),
                ));
            }
            // add the function to the user functions
            let arguments: Vec<(Identifier, DataType)> = args_list.value
                .identifiers
                .iter()
                .zip(args_list.value.datatypes.iter())
                .map(|(id, dt)| (id.value.clone(), dt.value.clone()))
                .collect();

            let func_def = FunctionDefinition {
                name: func_name.clone(),
                arguments: arguments.clone(),
                return_type: return_datatype.clone(),
                body: Vec::new(),
                pos: func_block.pos,
            };

            // println!("Function body for {}: {:?}", func_name, func_block);

            if let Err(e) = check_function_returns(&func_name,func_block, return_datatype){
                return Err(e);
            }

            state.user_functions.insert(func_name.clone(), func_def);
        }


        for (func_name, (args_list, return_datatype, func_block)) in &self.function_blocks {

            state.in_function = true;
            state.current_stack_depth += 1;
            let initial_stack_len = state.program_stack.len();

            let arguments: Vec<(Identifier, DataType)> = args_list.value
                .identifiers
                .iter()
                .zip(args_list.value.datatypes.iter())
                .map(|(id, dt)| {
                    // add the arguments to the stack
                    state.program_stack.push(Variable {
                        name: id.value.value.clone(),
                        depth: state.current_stack_depth,
                        mutable: true,
                        datatype: dt.value.clone(),
                        declare_pos: Some(id.pos),
                    });
                    (id.value.clone(), dt.value.clone())
                })
                .collect();


            // compile the function body
            let mut compiled_body = func_block.compile(&mut state)?;
            
            // if the function's return datatype is Void
            if *return_datatype == DataType::Void {
                let mut has_return = false;
                // check if it has a return instruction as the last instruction
                match compiled_body.instructions.last() {
                    Some(last_instruction) => {
                        if let InstructionType::Return { has_value: false } = &last_instruction.control {
                            has_return = true;
                        }
                    }
                    None => {}
                }
                // if it does not have a return instruction, add one
                if !has_return {
                    compiled_body.instructions.push(
                        Instruction {
                            control: InstructionType::Return {
                                has_value: false,
                            },
                            pos: Some(func_block.pos),
                        },
                    );
                }
            }

            // clean up compiler state
            state.program_stack.truncate(initial_stack_len);
            state.current_stack_depth -= 1;
            state.in_function = false;


            let func_def = FunctionDefinition {
                name: func_name.clone(),
                arguments,
                return_type: return_datatype.clone(),
                body: compiled_body.instructions,
                pos: func_block.pos,
            };

            state.user_functions.insert(func_name.clone(), func_def);

        }

        // before compiling the programs, get the list of program names and their arguments
        state.program_arguments = self
            .process_blocks
            .iter()
            .map(|(name, (args, _))| {
                (
                    name.clone(),
                    args.value
                        .datatypes
                        .iter()
                        .map(|d| d.value.clone())
                        .collect::<Vec<_>>(),
                )
            })
            .collect();

        // Compile all the programs
        state.is_shared = false;
        let mut programs_code = HashMap::new();
        // start with the main program

        let code = self.compile_program("main", &mut state)?;
        programs_code.insert("main".to_string(), code);
        assert!(state.current_stack_depth == 0);

        for name in self.process_blocks.keys() {
            if name == "main" {
                continue;
            }
            let code = self.compile_program(name, &mut state)?;
            programs_code.insert(name.clone(), code);
            assert!(state.current_stack_depth == 0);
        }

        // check if all the channed used have been declared
        for (channel_name, (_, pos)) in &state.undefined_channels {
            return Err(AlthreadError::new(
                ErrorType::UndefinedChannel,
                Some(pos.clone()),
                format!(
                    "Channel '{}' used in program '{}' at line {} has not been declared",
                    channel_name.1, channel_name.0, pos.line
                ),
            ));
        }

        let mut always_conditions = Vec::new();
        for (name, condition_block) in &self.condition_blocks {
            match name {
                ConditionKeyword::Always => {
                    for condition in condition_block.value.children.iter() {
                        let compiled = condition.compile(&mut state)?.instructions;
                        if compiled.len() == 1 {
                            return Err(AlthreadError::new(
                                ErrorType::InstructionNotAllowed,
                                Some(condition.pos),
                                "The condition must depend on shared variable(s)".to_string(),
                            ));
                        }
                        if compiled.len() != 2 {
                            return Err(AlthreadError::new(
                                ErrorType::InstructionNotAllowed,
                                Some(condition.pos),
                                "The condition must be a single expression".to_string(),
                            ));
                        }
                        if let InstructionType::GlobalReads { variables, .. } = &compiled[0].control
                        {
                            if let InstructionType::Expression(exp) = &compiled[1].control {
                                always_conditions.push((
                                    variables.iter().map(|s| s.clone()).collect(),
                                    variables.clone(),
                                    exp.clone(),
                                    condition.pos,
                                ));
                            } else {
                                return Err(AlthreadError::new(
                                    ErrorType::InstructionNotAllowed,
                                    Some(condition.pos),
                                    "The condition must be a single expression".to_string(),
                                ));
                            }
                        } else {
                            return Err(AlthreadError::new(
                                ErrorType::InstructionNotAllowed,
                                Some(condition.pos),
                                "The condition must depend on shared variable(s)".to_string(),
                            ));
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(CompiledProject {
            global_memory,
            user_functions: state.user_functions.clone(),
            programs_code,
            always_conditions,
            stdlib: Rc::new(state.stdlib),
        })
    }
    fn compile_program(
        &self,
        name: &str,
        state: &mut CompilerState,
    ) -> AlthreadResult<ProgramCode> {
        let mut process_code = ProgramCode {
            instructions: Vec::new(),
            name: name.to_string(),
        };
        let (args, prog) = self
            .process_blocks
            .get(name)
            .expect("trying to compile a non-existant program");
        state.current_program_name = name.to_string();

        for (i, var) in args.value.identifiers.iter().enumerate() {
            state.program_stack.push(Variable {
                name: var.value.value.clone(),
                depth: state.current_stack_depth,
                mutable: true,
                datatype: args.value.datatypes[i].value.clone(),
                declare_pos: Some(var.pos),
            });
        }

        let compiled = prog.compile(state)?;
        if compiled.contains_jump() {
            unimplemented!("breaks or return statements in programs are not yet implemented");
        }
        if !args.value.identifiers.is_empty() {
            process_code.instructions.push(Instruction {
                control: InstructionType::Destruct,
                pos: Some(args.pos),
            });
        }
        process_code.instructions.extend(compiled.instructions);
        process_code.instructions.push(Instruction {
            control: InstructionType::EndProgram,
            pos: Some(prog.pos),
        });
        Ok(process_code)
    }
}

impl fmt::Display for Ast {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.ast_fmt(f, &Prefix::new())
    }
}

impl AstDisplay for Ast {
    fn ast_fmt(&self, f: &mut Formatter, prefix: &Prefix) -> fmt::Result {
        if let Some(global_node) = &self.global_block {
            writeln!(f, "{}shared", prefix)?;
            global_node.ast_fmt(f, &prefix.add_branch())?;
        }

        writeln!(f, "")?;

        for (condition_name, condition_node) in &self.condition_blocks {
            writeln!(f, "{}{}", prefix, condition_name)?;
            condition_node.ast_fmt(f, &prefix.add_branch())?;
            writeln!(f, "")?;
        }

        for (process_name, (_args, process_node)) in &self.process_blocks {
            writeln!(f, "{}{}", prefix, process_name)?;
            process_node.ast_fmt(f, &prefix.add_branch())?;
            writeln!(f, "")?;
        }

        for (function_name, (_args, return_type, function_node)) in &self.function_blocks {
            writeln!(f, "{}{} -> {}", prefix, function_name, return_type)?;
            function_node.ast_fmt(f, &prefix.add_branch())?;
            writeln!(f, "")?;
        }

        Ok(())
    }
}
