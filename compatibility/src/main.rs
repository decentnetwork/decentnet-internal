use std::path::{Path, PathBuf};

use clap::Parser;
use serde_bytes::ByteBuf;
use serde_json::Error as JsonError;
use zerucontent::Content;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// ZeroNet Data Directory
    #[arg(short, long)]
    data_dir: Option<String>,

    /// ZeroNet Site Directory
    #[arg(short, long)]
    site_dir: Option<String>,

    /// Skip Missing File Errors
    #[arg(short, long, default_value = "false")]
    print_missing: bool,
}

enum Error {
    MissingFile,
    Io(std::io::Error),
    ParseFailed(JsonError),
    VerificationFailed,
}

fn main() {
    let args = Args::parse();

    if let Some(site_dir) = args.site_dir {
        let site_dir = PathBuf::from(site_dir);
        if !site_dir.is_dir() {
            println!("{} is not a ZeroNet site directory", site_dir.display())
        }
        let content_path = site_dir.join("content.json");
        if !content_path.is_file() {
            println!("{} does not have a content.json", site_dir.display());
            return;
        }
        if let Ok(contents) = get_content_json(content_path) {
            for content in contents.1 {
                let valid = check_valid_content(site_dir.join(content.clone()));
                if valid.is_err() {
                    handle_error(
                        &contents.0,
                        content.to_str().unwrap(),
                        &valid.err().unwrap(),
                        args.print_missing,
                    );
                }
            }
        } else {
            println!("{}'s content.json is not a valid", site_dir.display());
        }
        return;
    }
    if args.data_dir.is_none() {
        println!("Please specify a data directory");
        return;
    }
    let data_dir = PathBuf::from(args.data_dir.unwrap());
    if !data_dir.is_dir() {
        println!("{} is not a ZeroNet data directory", data_dir.display())
    }
    let dir_list = data_dir.read_dir().unwrap();
    for dir in dir_list {
        let dir = dir.unwrap();
        if dir.file_type().unwrap().is_dir() {
            let dir_name = dir.file_name();
            let dir_name = dir_name.to_str().unwrap();
            if dir_name.starts_with('1') {
                let content_path = dir.path().join("content.json");
                if !content_path.is_file() {
                    continue;
                }
                if let Ok(contents) = get_content_json(content_path) {
                    for content in contents.1 {
                        let valid = check_valid_content(dir.path().join(content.clone()));
                        if valid.is_err() {
                            handle_error(
                                &contents.0,
                                content.to_str().unwrap(),
                                &valid.err().unwrap(),
                                args.print_missing,
                            );
                        }
                    }
                }
            }
        }
    }
}

fn handle_error(site: &str, path: &str, error: &Error, print_missing: bool) -> Option<String> {
    if let Error::MissingFile = error {
        if !print_missing {
            return None;
        }
    }
    let err = match error {
        Error::MissingFile => "MissingFile".into(),
        Error::Io(err) => format!("Io: {}", err),
        Error::ParseFailed(err) => format!("Unsupported content.json, ParseFailed: {}", err),
        Error::VerificationFailed => "VerificationFailed".into(),
    };
    let err = format!("Site: {site}, {path}: err: {err}");
    println!("{}", err);
    Some(err)
}

fn get_content_json(path: impl AsRef<Path>) -> Result<(String, Vec<PathBuf>), Error> {
    let bytes = std::fs::read(path.as_ref());
    if bytes.is_err() {
        return Err(Error::Io(bytes.err().unwrap()));
    }
    let bytes = bytes.unwrap();
    let content = Content::from_buf(ByteBuf::from(bytes));
    if content.is_err() {
        return Err(Error::ParseFailed(content.err().unwrap()));
    }
    let content = content.unwrap();
    let verified = content.verify(content.address.clone());
    if !verified {
        return Err(Error::VerificationFailed);
    }
    let mut paths = vec![];
    for (path, _) in content.includes {
        let path = PathBuf::from(path);
        paths.push(path);
    }
    Ok((content.address.to_string(), paths))
}

fn check_valid_content(path: impl AsRef<Path>) -> Result<bool, Error> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(Error::MissingFile);
    }
    let bytes = std::fs::read(path);
    let bytes = bytes.unwrap();
    let content = Content::from_buf(ByteBuf::from(bytes));
    if content.is_err() {
        return Err(Error::ParseFailed(content.err().unwrap()));
    }
    let content = content.unwrap();
    let verified = content.verify(content.address.clone());
    if !verified {
        return Err(Error::VerificationFailed);
    }
    Ok(true)
}
