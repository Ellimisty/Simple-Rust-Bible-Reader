//! `sbr convert <input> <input-format|0> <output> <output-format>`
//!
//! Format numbers are the original filter indices (`F02_ZEFANIA_BIBLE = 1`,
//! `F13_VPL = 12`, ...); `0` autodetects the input format.

use sbr_core::{bible_format, global_memory};
use std::process::ExitCode;

fn usage() -> ExitCode {
    eprintln!(
        "usage: sbr convert [--newline lf|crlf] <input> <input-format|0> <output> <output-format>"
    );
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(i) = args.iter().position(|a| a == "--newline") {
        let nl = match args.get(i + 1).map(String::as_str) {
            Some("lf") => "\n",
            Some("crlf") => "\r\n",
            _ => return usage(),
        };
        global_memory::get_mut().newline = nl.into();
        args.drain(i..i + 2);
    }
    if args.len() != 5 || args[0] != "convert" {
        return usage();
    }
    let (Ok(from), Ok(to)) = (args[2].parse(), args[4].parse()) else {
        return usage();
    };
    global_memory::get_mut().console_mode = true;
    let result = bible_format::load(&args[1], from)
        .and_then(|mut b| bible_format::export(&mut b, &args[3], to));
    match result {
        Ok(()) => {
            println!("ok");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
