use miniserde::json;
use std::collections::HashMap;
use std::path::Path;
use std::fs::create_dir_all;
use std::fs::File;
use clap::Parser;
use clap::ValueEnum;
use std::io::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(ValueEnum, Clone, Debug)]
enum Action {
    Sort,
    Save,
}

#[derive(ValueEnum, Clone, Debug)]
enum SavedValue {
    Timestamp,
    Count,
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
   /// Name of the sort to backup
   #[clap(short, long, value_parser)]
   name: String,

   /// Mode to run
   #[clap(short, long, value_parser)]
   action: Action,

   /// Value to save
   #[clap(short, long, value_parser, default_value = "count")]
   value: SavedValue,
}

fn backup_lines(backup_file_path: &Path, lines_backup: &HashMap::<String, i64>) -> Result<(), &'static str> {
    let mut file = File::create(backup_file_path).map_err(|_x| "failed creating file")?;
    file.write_all(json::to_string(&lines_backup).as_bytes()).map_err(|_x| "failed writing file")?;
    Ok(())
}

fn load_lines(backup_file_path: &Path) -> Result<HashMap::<String, i64>, &'static str> {
    let mut file = File::open(backup_file_path).map_err(|_x| "failed opening file")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).map_err(|_x| "failed reading file")?;
    let content_str = &contents.as_str();
    json::from_str(&content_str).map_err(|_x| "from str")
}

fn get_value(lines_backup: &HashMap::<String, i64>, key: &String) -> i64 {
    *lines_backup.get(key).unwrap_or(&0)
}

fn trim_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
}

fn main() -> Result<(), &'static str> {
   let args = Args::parse();
   let cache_dir = dirs::cache_dir().ok_or("did not find cache dir")?;
   let backup_path  = Path::new(&cache_dir).join("baus");
   create_dir_all(&backup_path).map_err(|_x| "could not create backup path" )?;
   let backup_file_path = backup_path.join(&args.name);
   let lines_backup = HashMap::<String, i64>::new();
   if !backup_file_path.exists() {
       backup_lines(&backup_file_path, &lines_backup)?;
   }
   let mut lines_backup =  load_lines(&backup_file_path)?;
   match &args.action {
       Action::Sort => {
           let stdin = std::io::stdin();
           let mut lines : Vec<String> = stdin.lock().lines().map(|x| x.unwrap()).collect();
					 lines.sort_by_key(|k| get_value(&lines_backup, k) * -1);
           for line in lines {
               println!("{}", line);
           }
       },
       Action::Save => {
           let line = &mut String::new();
           std::io::stdin().read_line(line).map_err(|_x| "read line")?;
					 trim_newline(line);
           let value = match &args.value {
               SavedValue::Count => get_value(&lines_backup, line) + 1,
               SavedValue::Timestamp => SystemTime::now()
                   .duration_since(UNIX_EPOCH)
                   .expect("Time went backwards").as_secs() as i64,
           };
           lines_backup.insert(line.to_string(), value);
           backup_lines(&backup_file_path, &lines_backup)?;
					 println!("{}", line);
       },
   };
   Ok(())
}
