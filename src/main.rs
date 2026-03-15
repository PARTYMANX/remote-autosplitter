mod remote;
mod ui;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    /*if args.len() < 3 {
        println!("Usage: remote-autosplitter <autosplitter wasm path> <address:port>");
        return;
    }*/

    //let filepath = args.get(1).unwrap().to_owned();
    //let address = args.get(2).unwrap().to_owned();

    //let remote = remote::Remote::new("".to_string(), "".to_string());
    ui::run_remote_app("".to_string(), "".to_string());
}
