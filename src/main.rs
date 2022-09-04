use baus::{get_cache_file_path, get_lines_backup, get_stdin_lines, save, sort, Action, Args};
use clap::Parser;
use std::error::Error;
fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let cache_file_path = get_cache_file_path(&args)?;
    let mut lines_backup = get_lines_backup(&cache_file_path)?;
    let lines = get_stdin_lines()?;
    let output_lines = match &args.action {
        Action::Sort => sort(&args, lines, &mut lines_backup, &cache_file_path)?,
        Action::Save => save(&args, lines, lines_backup, &cache_file_path)?,
    };
    for line in &output_lines {
        println!("{}", line);
    }
    Ok(())
}
