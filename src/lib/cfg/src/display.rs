use crate::{Function, Module, Terminator};
use std::fmt::{Display, Formatter};

impl Display for Module {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if !self.globals.is_empty() {
            writeln!(f, "globals: {}", self.globals.join(", "))?;
        }
        for (idx, function) in self.functions.iter().enumerate() {
            let entry = if idx == self.entry { " (entry)" } else { "" };
            write!(f, "fn#{idx}{entry} {function}")?;
        }
        Ok(())
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let params: Vec<_> = self
            .params
            .iter()
            .map(|p| {
                let mut s = String::new();
                if p.by_value {
                    s.push_str("val ");
                }
                s.push_str(&p.name);
                if let Some(default) = p.default {
                    s.push_str(&format!(" = {default}"));
                }
                s
            })
            .collect();
        writeln!(f, "{}({})", self.name, params.join(", "))?;
        writeln!(f, "  locals: [{}]", self.locals.join(", "))?;
        for id in self.reachable_blocks() {
            let block = &self.blocks[id];
            writeln!(f, "  b{id}:")?;
            for (instruction, _) in &block.instructions {
                writeln!(f, "    {}", instruction.map_target(|b| format!("b{b}")))?;
            }
            match block.terminator {
                Some((Terminator::Jump(target), _)) => writeln!(f, "    jump b{target}")?,
                Some((Terminator::Branch { then, otherwise }, _)) => {
                    writeln!(f, "    branch b{then} b{otherwise}")?
                }
                Some((Terminator::Return, _)) => writeln!(f, "    return")?,
                Some((Terminator::Raise, _)) => writeln!(f, "    raise")?,
                Some((Terminator::ReRaise, _)) => writeln!(f, "    reraise")?,
                None => writeln!(f, "    <unterminated>")?,
            }
        }
        Ok(())
    }
}
