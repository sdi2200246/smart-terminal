use std::env;
use std::path::PathBuf;

use crate::agent::memory::FolderMemory;
use crate::cli::cli::{MemoryAction, MemoryArgs};
use crate::core::memory::{Memory, MemoryError};

pub async fn run(args: MemoryArgs) {
    let cwd = match env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("failed to read cwd: {e}");
            std::process::exit(1);
        }
    };

    let mut memory = match FolderMemory::project_local() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("failed to open memory store: {e}");
            std::process::exit(1);
        }
    };

    let result = match args.action {
        MemoryAction::Init => init(&mut memory, &cwd),
        MemoryAction::Delete => delete(&mut memory, &cwd),
        MemoryAction::Clear => clear(&mut memory, &cwd),
        MemoryAction::Show => show(&mut memory, &cwd),
    };

    if let Err(e) = result {
        eprintln!("\x1b[31m✗ {e}\x1b[0m");
        std::process::exit(1);
    }
}

fn init(memory: &mut FolderMemory, cwd: &PathBuf) -> Result<(), MemoryError> {
    memory.register(cwd)?;
    println!("\x1b[32m✓ registered\x1b[0m {}", cwd.display());
    Ok(())
}

fn delete(memory: &mut FolderMemory, cwd: &PathBuf) -> Result<(), MemoryError> {
    memory.unregister(cwd)?;
    println!("\x1b[32m✓ deleted\x1b[0m memory for {}", cwd.display());
    Ok(())
}

fn clear(memory: &mut FolderMemory, cwd: &PathBuf) -> Result<(), MemoryError> {
    if !memory.load(cwd)? {
        eprintln!("not registered — run `memory init` first");
        return Ok(());
    }
    memory.clear()?;
    println!(
        "\x1b[32m✓ cleared\x1b[0m interactions for {}",
        cwd.display()
    );
    Ok(())
}

fn show(memory: &mut FolderMemory, cwd: &PathBuf) -> Result<(), MemoryError> {
    if !memory.load(cwd)? {
        println!("not registered — run `memory init` first");
        return Ok(());
    }

    let conv = memory.current().expect("loaded");
    if conv.interactions.is_empty() {
        println!("no interactions yet for {}", cwd.display());
        return Ok(());
    }

    println!(
        "memory for {} ({} interactions):\n",
        cwd.display(),
        conv.interactions.len()
    );
    for (i, entry) in conv.interactions.iter().enumerate() {
        println!(
            "{:>3}. {} → {}",
            i + 1,
            entry.user_input.trim(),
            entry.predicted_cmd
        );
    }
    Ok(())
}
