use std::env;
use std::path::PathBuf;

use crate::cli::{MemoryAction, MemoryArgs, Presenter};
use crate::core::{Memory, PersistenceError};
use crate::persistence::NextCmdMemory;

pub async fn run(args: MemoryArgs) {
    let cwd = match env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            Presenter::render_error(format!("failed to read cwd: {e}"));
            std::process::exit(1);
        }
    };

    let mut memory = NextCmdMemory::project_local();

    let result = match args.action {
        MemoryAction::Init => init(&mut memory, &cwd),
        MemoryAction::Delete => delete(&mut memory, &cwd),
        MemoryAction::Clear => clear(&mut memory, &cwd),
        MemoryAction::Show => show(&mut memory, &cwd),
    };

    if let Err(e) = result {
        Presenter::render_error(e.to_string());
        std::process::exit(1);
    }
}

fn init(memory: &mut NextCmdMemory, cwd: &PathBuf) -> Result<(), PersistenceError> {
    memory.register(cwd)?;
    Presenter::render_memory_registered(cwd);
    Ok(())
}

fn delete(memory: &mut NextCmdMemory, cwd: &PathBuf) -> Result<(), PersistenceError> {
    memory.unregister(cwd)?;
    Presenter::render_memory_deleted(cwd);
    Ok(())
}

fn clear(memory: &mut NextCmdMemory, cwd: &PathBuf) -> Result<(), PersistenceError> {
    if !memory.load(cwd)? {
        Presenter::render_memory_unregistered();
        return Ok(());
    }
    memory.clear()?;
    Presenter::render_memory_cleared(cwd);
    Ok(())
}

fn show(memory: &mut NextCmdMemory, cwd: &PathBuf) -> Result<(), PersistenceError> {
    if !memory.load(cwd)? {
        Presenter::render_memory_unregistered();
        return Ok(());
    }

    let conv = memory.current().expect("loaded");
    Presenter::render_memory_interactions(cwd, &conv.interactions);
    Ok(())
}
