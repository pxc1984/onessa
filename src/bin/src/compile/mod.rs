use crate::cli::CompileArgs;

mod build_ast;
mod lexer;
mod source;

pub fn compile(args: CompileArgs) {
    let source = source::open_source(args.input);
    let mut lexer = lexer::init_lexer(&source.content);
    let _syntax_tree = build_ast::build_ast(&mut lexer);
}
