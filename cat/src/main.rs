use std::env;
use std::io;
use std::fs;
use std::path;

macro_rules! cat_die {
    ($fmt:expr, $($arg:tt)*) => ({
        eprintln!(concat!("cat-rs: ", $fmt), $($arg), *);
        ::std::process::exit(1);
    });
    ($fmt:expr) => ({
        eprintln!(concat!("cat-rs: ", $fmt));
        ::std::process::exit(1);
    });
}

fn copy_to_stdout_or_die(from: &mut std::io::Read) {
    io::copy(from, &mut io::stdout()).unwrap_or_else(|_| {
        cat_die!("I/O error");
    });
}

fn cat_file(name: &String) {
    match path::Path::new(name).metadata() {
        Err(e) => {
            match e.kind() {
                io::ErrorKind::NotFound => cat_die!("{}: no such file or directory", name),
                io::ErrorKind::PermissionDenied => cat_die!("{}: permission denied", name),
                _ => cat_die!("{}: unknown error", name),

            }
        }
        Ok(info) => {
            if info.is_dir() {
                cat_die!("{}: is a directory", name);
            }
        }
    };

    match fs::File::open(name) {
        Err(_) => cat_die!("{}: unknown error", name),
        Ok(mut f) => copy_to_stdout_or_die(&mut f),
    };
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.len() == 0 {
        copy_to_stdout_or_die(&mut io::stdin())
    } else {
        for arg in args {
            if arg == "-" {
                copy_to_stdout_or_die(&mut io::stdin())
            } else {
                cat_file(&arg);
            }
        }
    };
}
