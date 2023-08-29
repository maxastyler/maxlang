use anyhow::{Context, Result};

use crate::expression::Symbol;

pub struct Named {
    pub name: Symbol,
    pub depth: usize,
}

pub enum Local {
    Named(Named),
    Temporary,
    Reserved,
    None,
}

pub struct CompilerFrame {
    pub locals: Vec<Local>,
    pub captures: Vec<Named>,
    pub max_num_registers: usize,
    pub depth: usize,
}

impl CompilerFrame {
    fn new(arguments: Vec<Symbol>, depth: usize) -> Self {
        CompilerFrame {
            locals: arguments
                .into_iter()
                .map(|s| Local::Named(Named { name: s, depth }))
                .collect(),
            captures: vec![],
            max_num_registers: arguments.len(),
            depth: depth + 1,
        }
    }

    /// Finds the symbol with the same name at the highest depth <= self.depth
    fn find_local_symbol(&self, symbol: &Symbol) -> Option<usize> {
        self.locals
            .iter()
            .enumerate()
            .filter_map(|(i, l)| match l {
                Local::Named(n @ Named { name, depth })
                    if *name == *symbol && *depth <= self.depth =>
                {
                    Some((i, n))
                }
                _ => None,
            })
            .max_by_key(|(_, l)| l.depth)
            .map(|(i, _)| i)
    }
}

pub struct Compiler {
    pub frames: Vec<CompilerFrame>,
}

impl Compiler {
    fn last_frame(&self) -> Result<&CompilerFrame> {
        self.frames.last().context("Could not get last frame")
    }

    fn last_frame_mut(&mut self) -> Result<&mut CompilerFrame> {
        self.frames.last_mut().context("Could not get last frame")
    }

    fn find_nonlocal_symbol(&mut self, frame_index: usize, symbol: &Symbol) -> Option<usize> {
        if let Some(i) = self.frames[frame_index].find_local_symbol(symbol) {
	    
	}
    }
}
