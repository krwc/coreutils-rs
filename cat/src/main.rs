use std::env;
use std::io::{self, Write, BufReader};
use std::fs;
use std::path;

macro_rules! cat_die {
    ($fmt:expr, $($arg:tt)*) => ({
        eprintln!(concat!("cat: ", $fmt), $($arg)*);
        ::std::process::exit(1);
    });
    ($fmt:expr) => ({
        eprintln!(concat!("cat: ", $fmt));
        ::std::process::exit(1);
    });
}

/// Classification of cat's arguments
#[derive(Debug, Clone, PartialEq)]
pub enum CatArg {
    Ends,
    File(String),
    Help,
    Number,
    Squeeze,
    Stdin,
    Version,
}

#[derive(Debug)]
pub struct CatArgDef {
    arg: CatArg,
    shortname: &'static str,
    longname: &'static str,
    help: &'static str,
}

static OPTIONS: &'static [CatArgDef] = &[
    CatArgDef {
        arg: CatArg::Help,
        shortname: "-h",
        longname: "--help",
        help: "show this message and exit",
    },
    CatArgDef {
        arg: CatArg::Number,
        shortname: "-n",
        longname: "--number",
        help: "number all output lines",
    },
    CatArgDef {
        arg: CatArg::Ends,
        shortname: "-E",
        longname: "--show-ends",
        help: "display $ at end of each line",
    },
    CatArgDef {
        arg: CatArg::Squeeze,
        shortname: "-s",
        longname: "--squeeze-blank",
        help: "squeeze consecutive empty lines into one",
    },
    CatArgDef {
        arg: CatArg::Version,
        shortname: "-v",
        longname: "--version",
        help: "output version information and exit",
    },
];

pub struct Decorators {
    ends: bool,
    number: bool,
    squeeze: bool,
}

impl Decorators {
    fn any(&self) -> bool {
        return self.ends || self.number || self.squeeze;
    }
}

fn copy_to_stdout_raw_or_die(from: &mut std::io::Read) {
    let stdout = io::stdout();
    io::copy(from, &mut stdout.lock()).unwrap_or_else(|_| {
        cat_die!("I/O error");
    });
}

fn copy_to_stdout_decorated_or_die(reader: &mut std::io::Read, decorators: &Decorators) {
    const BUFSIZE: usize = 8192;
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    let mut input: [u8; BUFSIZE] = [0u8; BUFSIZE];
    let mut output: Vec<u8> = vec![];

    let mut last_byte: i32 = -1;
    let mut empty_streak: i32 = 1;
    let mut current_line: i32 = 1;

    loop {
        let len = match reader.read(&mut input) {
            Ok(0) => break,
            Ok(len) => len,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            _ => cat_die!("I/O read error"),
        };

        for character in &input[..len] {
            if decorators.squeeze {
                if *character == '\n' as u8 {
                    empty_streak += 1;

                    if empty_streak >= 3 {
                        continue;
                    }
                } else {
                    empty_streak = 0;
                }
            }
            if decorators.number {
                if last_byte < 0 || last_byte == '\n' as i32 {
                    write!(&mut output, "{:6}: ", current_line).unwrap();
                    current_line += 1
                }
            }
            if decorators.ends {
                if *character == '\n' as u8 {
                    output.push('$' as u8);
                }
            }
            output.push(*character);
            last_byte = *character as i32;
        }

        writer.write_all(&output).unwrap_or_else(|_| {
            cat_die!("I/O write error");
        });
        output.clear();
    }
}

fn copy_to_stdout_or_die(from: &mut std::io::Read, decorators: &Decorators) {
    if decorators.any() {
        copy_to_stdout_decorated_or_die(from, decorators)
    } else {
        copy_to_stdout_raw_or_die(from)
    }
}

fn get_file(name: &String) -> io::BufReader<fs::File> {
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

    return match fs::File::open(name) {
        Err(_) => cat_die!("{}: unknown error", name),
        Ok(f) => BufReader::new(f),
    };
}

fn cat_arg(arg: CatArg, decorators: &Decorators) {
    let mut readable: Box<io::Read> = match arg {
        CatArg::File(ref name) => Box::new(get_file(name)),
        CatArg::Stdin => Box::new(io::stdin()),
        _ => return,
    };
    copy_to_stdout_or_die(&mut *readable, decorators);
}

fn show_help() {
    println!(
        "Usage: {}: [OPTION]... [FILENAME]...",
        env::args().nth(0).unwrap()
    );
    println!(
        "Partial implementation of standard GNU cat. Concatenates FILE(s) to standard output."
    );
    println!("");
    println!("With no FILE, or when FILE is -, read standard input.");
    for option in OPTIONS {
        println!(
            "  {}, {}\n\t{}",
            option.shortname,
            option.longname,
            option.help
        );
    }
}

fn parse_args() -> Vec<CatArg> {
    return env::args()
        .skip(1)
        .map(|arg| {
            if arg == "-" {
                return CatArg::Stdin;
            }
            for option in OPTIONS {
                if option.shortname == arg || option.longname == arg {
                    return option.arg.clone();
                }
            }
            if arg.starts_with("-") {
                cat_die!(
                    "unrecognized option '{}'\nTry '{} --help' for more information",
                    arg,
                    env::args().nth(0).unwrap()
                );
            }
            return CatArg::File(arg);
        })
        .collect();
}

fn main() {
    let mut args = parse_args();

    if args.iter().any(|e| *e == CatArg::Help) {
        return show_help();
    }
    if args.iter().any(|e| *e == CatArg::Version) {
        return println!("Partial implementation of GNU cat, version 1.0.1");
    }

    let decorators = Decorators {
        ends: args.iter().any(|e| *e == CatArg::Ends),
        number: args.iter().any(|e| *e == CatArg::Number),
        squeeze: args.iter().any(|e| *e == CatArg::Squeeze),
    };

    let mut has_any_input = false;
    for arg in args.iter() {
        match *arg {
            CatArg::File(_) | CatArg::Stdin => has_any_input = true,
            _ => {}
        }
    }
    if !has_any_input {
        args.push(CatArg::Stdin);
    }

    for arg in args {
        cat_arg(arg, &decorators);
    }
}
