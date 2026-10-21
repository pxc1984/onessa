mod expression;
mod function;
mod literal;
mod statement;
mod syntax;

use crate::types::Type;
use crate::{Diagnostic, Function, Module, Parameter};
use function::FunctionBuilder;
use onessa_ast::{Lang, SyntaxKind};
use onessa_core::object::StringTable;
use rowan::{SyntaxNode, TextSize};
use std::collections::HashMap;
use syntax::{child_nodes, has_token, identifier_text};

/// Maximum number of parameters of a method (`argc` is a single byte).
const MAX_PARAMS: usize = u8::MAX as usize;

/// Name of the synthetic method holding the module body.
pub const MODULE_BODY_NAME: &str = "<module>";

pub(crate) struct Signature {
    pub name: String,
    pub returns_value: bool,
    /// Strict mode type of the result of a function.
    pub return_type: Type,
    pub export: bool,
    pub params: Vec<Parameter>,
}

/// State shared by all methods of the module being lowered.
pub(crate) struct Context {
    source: String,
    line_starts: Vec<usize>,
    pub strict: bool,
    pub globals: Vec<String>,
    /// Strict mode types of the module variables; `None` until the first assignment.
    pub global_types: Vec<Option<Type>>,
    pub global_map: HashMap<String, u16>,
    pub signatures: Vec<Signature>,
    pub signature_map: HashMap<String, usize>,
    pub strings: StringTable,
    pub diagnostics: Vec<Diagnostic>,
}

impl Context {
    fn new(source: String, strict: bool) -> Self {
        let line_starts = std::iter::once(0)
            .chain(source.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        Self {
            source,
            line_starts,
            strict,
            globals: Vec::new(),
            global_types: Vec::new(),
            global_map: HashMap::new(),
            signatures: Vec::new(),
            signature_map: HashMap::new(),
            strings: StringTable::new(),
            diagnostics: Vec::new(),
        }
    }

    /// 1-based line and column (in characters) of a byte offset.
    pub fn position(&self, offset: TextSize) -> (u32, u32) {
        let offset = usize::from(offset).min(self.source.len());
        let line = self.line_starts.partition_point(|&start| start <= offset) - 1;
        let column = self.source[self.line_starts[line]..offset].chars().count();
        (line as u32 + 1, column as u32 + 1)
    }

    pub fn line(&self, node: &SyntaxNode<Lang>) -> u32 {
        self.position(first_token_offset(node)).0
    }

    pub fn error(&mut self, node: &SyntaxNode<Lang>, message: impl Into<String>) {
        let (line, column) = self.position(first_token_offset(node));
        self.diagnostics.push(Diagnostic {
            message: message.into(),
            line,
            column,
        });
    }
}

/// Offset of the first non-trivia token, so that errors point at code rather than whitespace.
fn first_token_offset(node: &SyntaxNode<Lang>) -> TextSize {
    node.descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .find(|token| token.kind() != SyntaxKind::Trivia)
        .map(|token| token.text_range().start())
        .unwrap_or_else(|| node.text_range().start())
}

/// Compilation options.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// Static typing (`--strict`), see [`crate::types`].
    pub strict: bool,
}

/// Maximum number of passes that refine the result types of functions in strict mode.
const MAX_TYPE_PASSES: usize = 8;

/// Lowers a parsed module into a control flow graph.
///
/// `root` must come from a syntax tree without parse errors.
pub fn build_cfg(root: &SyntaxNode<Lang>) -> Result<Module, Vec<Diagnostic>> {
    build_cfg_with(root, Options::default())
}

pub fn build_cfg_with(
    root: &SyntaxNode<Lang>,
    options: Options,
) -> Result<Module, Vec<Diagnostic>> {
    let mut ctx = Context::new(root.text().to_string(), options.strict);

    let Some(statements) = root
        .descendants()
        .find(|n| n.kind() == SyntaxKind::StatementList)
    else {
        return Err(vec![Diagnostic {
            message: "Syntax tree has no statement list".to_string(),
            line: 1,
            column: 1,
        }]);
    };

    // Pass 1: module variables and method signatures, so that methods can be called before
    // they are declared.
    let mut methods = Vec::new();
    let mut body = Vec::new();
    for statement in child_nodes(&statements) {
        match statement.kind() {
            SyntaxKind::VariableDeclaration => declare_globals(&mut ctx, &statement),
            SyntaxKind::ProcedureDeclaration | SyntaxKind::FunctionDeclaration => {
                if declare_method(&mut ctx, &statement) {
                    methods.push(statement);
                }
            }
            _ => body.push(statement),
        }
    }

    for (index, method) in methods.iter().enumerate() {
        let assigned = assigned_variables(method, &ctx.signature_map);
        for param in &mut ctx.signatures[index].params {
            param.assigned = assigned.contains(&param.name.to_lowercase());
        }
    }

    // Pass 2: method bodies, then the module body. In strict mode the result type of a
    // function depends on the bodies of the functions it calls, so the bodies are lowered
    // again until the result types stop changing; they start as "no value yet" and only
    // grow, which ends in a few passes.
    let declaration_errors = std::mem::take(&mut ctx.diagnostics);
    let mut passes = 0;
    let functions = loop {
        passes += 1;
        ctx.diagnostics = declaration_errors.clone();
        ctx.global_types.iter_mut().for_each(|t| *t = None);

        let mut functions = Vec::with_capacity(methods.len() + 1);
        let mut changed = false;
        for (index, method) in methods.iter().enumerate() {
            let (function, returned) = FunctionBuilder::lower_method(&mut ctx, index, method);
            let signature = &mut ctx.signatures[index];
            if ctx.strict && signature.returns_value {
                let return_type = if passes < MAX_TYPE_PASSES {
                    signature.return_type.join(returned)
                } else {
                    Type::Any
                };
                changed |= return_type != signature.return_type;
                signature.return_type = return_type;
            }
            functions.push(function);
        }
        functions.push(FunctionBuilder::lower_module_body(&mut ctx, &body));
        if !changed {
            break functions;
        }
    };
    let entry = functions.len() - 1;

    if !ctx.diagnostics.is_empty() {
        ctx.diagnostics.sort_by_key(|d| (d.line, d.column));
        ctx.diagnostics.dedup();
        return Err(ctx.diagnostics);
    }

    let globals = ctx.globals;
    let strings = ctx.strings;
    let functions = functions
        .into_iter()
        .map(|mut f: Function| {
            strings.push(&f.name);
            f.locals.shrink_to_fit();
            f
        })
        .collect();

    Ok(Module {
        globals,
        functions,
        entry,
        strings,
    })
}

fn declare_globals(ctx: &mut Context, declaration: &SyntaxNode<Lang>) {
    for identifier in child_nodes(declaration).filter(|n| n.kind() == SyntaxKind::Identifier) {
        let name = identifier_text(&identifier);
        let key = name.to_lowercase();
        if ctx.global_map.contains_key(&key) {
            ctx.error(&identifier, format!("Variable is already defined ({name})"));
            continue;
        }
        if ctx.globals.len() > u16::MAX as usize {
            ctx.error(&identifier, "Too many module variables");
            continue;
        }
        ctx.global_map.insert(key, ctx.globals.len() as u16);
        ctx.globals.push(name);
        ctx.global_types.push(None);
    }
}

/// Registers the signature of a procedure or function. Returns `false` if it is invalid.
fn declare_method(ctx: &mut Context, method: &SyntaxNode<Lang>) -> bool {
    let Some(name_node) = child_nodes(method).find(|n| n.kind() == SyntaxKind::Identifier) else {
        ctx.error(method, "Method has no name");
        return false;
    };
    let name = identifier_text(&name_node);
    let key = name.to_lowercase();
    if ctx.signature_map.contains_key(&key) {
        ctx.error(
            &name_node,
            format!("Procedure or function is already defined ({name})"),
        );
        return false;
    }

    let mut params: Vec<Parameter> = Vec::new();
    for parameter in child_nodes(method).filter(|n| n.kind() == SyntaxKind::Parameter) {
        let Some(param_name) = child_nodes(&parameter).find(|n| n.kind() == SyntaxKind::Identifier)
        else {
            continue;
        };
        let param_name = identifier_text(&param_name);
        if params
            .iter()
            .any(|p| p.name.to_lowercase() == param_name.to_lowercase())
        {
            ctx.error(
                &parameter,
                format!("Parameter is already defined ({param_name})"),
            );
        }
        let default = child_nodes(&parameter)
            .find(|n| n.kind() == SyntaxKind::Expression)
            .and_then(|expression| {
                let constant = literal::constant_expression(ctx, &expression);
                if constant.is_none() {
                    ctx.error(&expression, "Default parameter value must be a constant");
                }
                constant
            });
        params.push(Parameter {
            name: param_name,
            by_value: has_token(&parameter, SyntaxKind::Val),
            default,
            assigned: false,
        });
    }

    if params.len() > MAX_PARAMS {
        ctx.error(
            method,
            format!("Too many parameters (at most {MAX_PARAMS})"),
        );
        return false;
    }

    ctx.signature_map.insert(key, ctx.signatures.len());
    ctx.signatures.push(Signature {
        name,
        returns_value: method.kind() == SyntaxKind::FunctionDeclaration,
        return_type: if ctx.strict { Type::Never } else { Type::Any },
        export: has_token(method, SyntaxKind::Export),
        params,
    });
    true
}

/// Lowercased names of the variables a method may change: assignment targets, counters of
/// `Для`, and variables passed to procedures and functions of the module (which may take
/// them by reference). Global context functions and object methods never change their
/// arguments.
fn assigned_variables(
    method: &SyntaxNode<Lang>,
    methods: &HashMap<String, usize>,
) -> std::collections::HashSet<String> {
    let name = |node: Option<SyntaxNode<Lang>>| {
        node.filter(|n| n.kind() == SyntaxKind::Identifier)
            .map(|n| identifier_text(&n).to_lowercase())
    };
    let mut names = std::collections::HashSet::new();
    for node in method.descendants() {
        match node.kind() {
            SyntaxKind::AssignmentStatement | SyntaxKind::ForStatement => {
                names.extend(name(child_nodes(&node).next()));
            }
            SyntaxKind::CallExpression => {
                let callee = name(child_nodes(&node).next());
                if !callee.is_some_and(|callee| methods.contains_key(&callee)) {
                    continue;
                }
                for arg in child_nodes(&node).skip(1) {
                    let mut inner = child_nodes(&arg);
                    if let (Some(single), None) = (inner.next(), inner.next()) {
                        names.extend(name(Some(single)));
                    }
                }
            }
            _ => {}
        }
    }
    names
}

#[cfg(test)]
mod tests;
