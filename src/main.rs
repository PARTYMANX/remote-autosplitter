#![windows_subsystem = "windows"]

mod profile;
mod remote;
mod ui;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 2 {
        println!("Usage: remote-autosplitter [profile filepath]");
        return;
    }

    let profile_filepath = if args.len() == 2 {
        Some(args[1].clone())
    } else {
        None
    };

    ui::run_remote_app(profile_filepath);
}
