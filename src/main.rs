use clap::Parser;
use clap::ValueEnum;
use miniserde::json;
use std::collections::HashMap;
use std::error::Error;
use std::fs::create_dir_all;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;
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

    /// sort in descending order instead of ascending
    #[clap(short, long, value_parser)]
    desc: bool,

    /// keep only entries in sort in the cache
    #[clap(short, long, value_parser)]
    cleanup: bool,
}

fn backup_lines(
    cache_file_path: &Path,
    lines_backup: &HashMap<String, i64>,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(cache_file_path)?;
    file.write_all(json::to_string(&lines_backup).as_bytes())?;
    Ok(())
}

fn load_lines(cache_file_path: &Path) -> Result<HashMap<String, i64>, Box<dyn Error>> {
    let mut file = File::open(cache_file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let content_str = &contents.as_str();
    json::from_str(&content_str).map_err(|x| x.into())
}

fn get_value(lines_backup: &HashMap<String, i64>, key: &String) -> i64 {
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

fn cleanup(
    lines_backup: &mut HashMap<String, i64>,
    cache_file_path: &Path,
    lines: &Vec<String>,
) -> Result<(), Box<dyn Error>> {
    lines_backup.retain(|k, _| lines.contains(&k));
    for line in lines {
        if !lines_backup.contains_key(line) {
            lines_backup.insert(line.clone(), 0);
        }
    }
    backup_lines(&cache_file_path, &lines_backup)?;
    Ok(())
}

fn get_stdin_lines() -> Result<Vec<String>, Box<dyn Error>> {
    let stdin = std::io::stdin();
    Ok(stdin.lock().lines().map(|x| x.unwrap()).collect())
}

fn get_asc_sorted_lines(
    mut lines: Vec<String>,
    lines_backup: &HashMap<String, i64>,
) -> Result<Vec<String>, Box<dyn Error>> {
    lines.sort_by(|a, b| {
        get_value(&lines_backup, a)
            .partial_cmp(&get_value(&lines_backup, b))
            .unwrap()
    });
    Ok(lines)
}

fn sort(
    args: &Args,
    lines: Vec<String>,
    lines_backup: &mut HashMap<String, i64>,
    cache_file_path: &Path,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut lines = get_asc_sorted_lines(lines, &lines_backup)?;
    if args.desc {
        lines = lines.into_iter().rev().collect();
    }
    if args.cleanup {
        cleanup(lines_backup, cache_file_path, &lines)?;
    }
    Ok(lines)
}

fn update_first_stdin_line(
    saved_value: &SavedValue,
    lines: &Vec<String>,
    lines_backup: &mut HashMap<String, i64>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut output_lines = Vec::new();
    lines.get(0).map(|l| {
        let mut line = l.clone();
        trim_newline(&mut line);
        let value = match &saved_value {
            SavedValue::Count => get_value(&lines_backup, &line) + 1,
            SavedValue::Timestamp => SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs() as i64,
        };
        lines_backup.insert(line.to_string(), value);
        output_lines.push(line)
    });
    Ok(output_lines)
}

fn get_cache_file_path(args: &Args) -> Result<PathBuf, Box<dyn Error>> {
    let cache_dir = dirs::cache_dir().ok_or("did not find cache dir")?;
    let backup_path = Path::new(&cache_dir).join("baus");
    create_dir_all(&backup_path)?;
    Ok(backup_path.join(&args.name))
}

fn get_lines_backup(cache_file_path: &Path) -> Result<HashMap<String, i64>, Box<dyn Error>> {
    let lines_backup = HashMap::<String, i64>::new();
    if !cache_file_path.exists() {
        backup_lines(&cache_file_path, &lines_backup)?;
    }
    Ok(load_lines(&cache_file_path)?)
}

fn save(
    args: &Args,
    lines: Vec<String>,
    mut lines_backup: HashMap<String, i64>,
    cache_file_path: &Path,
) -> Result<Vec<String>, Box<dyn Error>> {
    let output_lines = update_first_stdin_line(&args.value, &lines, &mut lines_backup)?;
    backup_lines(&cache_file_path, &lines_backup)?;
    Ok(output_lines)
}

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

#[test]
fn test_get_asc_sorted_lines() {
    assert_eq!(
        get_asc_sorted_lines(
            Vec::from(["horse".to_string(), "hamster".to_string()]),
            &HashMap::from([("horse".to_string(), 2), ("hamster".to_string(), 1)]),
        )
        .unwrap(),
        Vec::from(["hamster", "horse"])
    )
}

#[test]
fn test_update_first_stdin_line() {
    let mut cache = HashMap::from([("horse".to_string(), 2), ("hamster".to_string(), 1)]);
    assert_eq!(
        update_first_stdin_line(
            &SavedValue::Count,
            &Vec::from(["horse".to_string(), "hamster".to_string()]),
            &mut cache,
        )
        .unwrap(),
        Vec::from(["horse"])
    );
    assert_eq!(cache.get("horse"), Some(&3));
    assert_eq!(cache.get("hamster"), Some(&1))
}
