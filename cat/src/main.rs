use std::env;
use std::io::{self, Write};
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

/// Trait used to perform text decoration when special cat options are enabled,
/// such as line numbering.
trait Decorator {
    /// Decorates the byte @ref byte writing the decoration into @p output. If the byte
    /// is to be removed from the result the function returns true, otherwise it returns
    /// false.
    fn decorate(&mut self, byte: u8, output: &mut Vec<u8>) -> bool;
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

fn copy_to_stdout_raw_or_die(from: &mut std::io::Read) {
    io::copy(from, &mut io::stdout()).unwrap_or_else(|_| {
        cat_die!("I/O error");
    });
}

fn copy_to_stdout_decorated_or_die(from: &mut std::io::Read, decorators: &mut [Box<Decorator>]) {
    loop {
        let mut input: Vec<u8> = Vec::with_capacity(8192);
        let mut output: Vec<u8> = Vec::with_capacity(8192);

        match from.read_to_end(&mut input) {
            Ok(0) => break,
            Ok(_) => {
                for character in input {
                    let mut truncate = false;
                    for decorator in decorators.iter_mut() {
                        truncate = decorator.decorate(character, &mut output);

                        if truncate {
                            break;
                        }
                    }

                    if !truncate {
                        output.push(character);
                    }
                }
                io::stdout().write_all(&output).unwrap_or_else(
                    |_| cat_die!("I/O error"),
                );
            }
            Err(_) => {
                cat_die!("I/O error");
            }
        }
    }
}

fn copy_to_stdout_or_die(from: &mut std::io::Read, decorators: &mut [Box<Decorator>]) {
    if decorators.len() > 0 {
        copy_to_stdout_decorated_or_die(from, decorators)
    } else {
        copy_to_stdout_raw_or_die(from)
    }
}

fn get_file(name: &String) -> Box<fs::File> {
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
        Ok(f) => Box::new(f),
    };
}

fn cat_arg(arg: CatArg, decorators: &mut [Box<Decorator>]) {
    let mut readable: Box<io::Read> = match arg {
        CatArg::File(ref name) => get_file(name),
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
    println!("Rust implementation of standard GNU cat. Concatenates FILE(s) to standard output.");
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

struct LineNumberDecorator {
    current_line: i32,
    last_byte: i32,
}

impl LineNumberDecorator {
    fn new() -> LineNumberDecorator {
        LineNumberDecorator {
            current_line: 1,
            last_byte: -1,
        }
    }
}

impl Decorator for LineNumberDecorator {
    fn decorate(&mut self, byte: u8, output: &mut Vec<u8>) -> bool {
        if self.last_byte < 0 || self.last_byte == '\n' as i32 {
            output.append(&mut format!("{:6}: ", self.current_line).into_bytes());
            self.current_line += 1;
        }
        self.last_byte = byte as i32;
        return false;
    }
}

struct EndLineDecorator;

impl EndLineDecorator {
    fn new() -> EndLineDecorator {
        EndLineDecorator {}
    }
}

impl Decorator for EndLineDecorator {
    fn decorate(&mut self, byte: u8, output: &mut Vec<u8>) -> bool {
        if byte == '\n' as u8 {
            output.push('$' as u8);
        }
        return false;
    }
}

struct LineSqueezerDecorator {
    last_byte: i32,
    streak: i32,
}

impl LineSqueezerDecorator {
    fn new() -> LineSqueezerDecorator {
        LineSqueezerDecorator {
            last_byte: -1,
            streak: 0,
        }
    }
}

impl Decorator for LineSqueezerDecorator {
    fn decorate(&mut self, byte: u8, _: &mut Vec<u8>) -> bool {
        if self.last_byte == '\n' as i32 && byte == '\n' as u8 {
            self.streak += 1;

            if self.streak != 1 {
                return true;
            }
        } else {
            self.streak = 0;
        }
        self.last_byte = byte as i32;
        return false;
    }
}

fn main() {
    let mut args = parse_args();

    if args.iter().any(|e| *e == CatArg::Help) {
        return show_help();
    }
    if args.iter().any(|e| *e == CatArg::Version) {
        return println!("cat, rust implementation - version 1.0");
    }

    let mut decorators: Vec<Box<Decorator>> = vec![];
    if args.iter().any(|e| *e == CatArg::Squeeze) {
        decorators.push(Box::new(LineSqueezerDecorator::new()))
    }
    if args.iter().any(|e| *e == CatArg::Number) {
        decorators.push(Box::new(LineNumberDecorator::new()));
    }
    if args.iter().any(|e| *e == CatArg::Ends) {
        decorators.push(Box::new(EndLineDecorator::new()));
    }

    if args.len() == 0 {
        args.push(CatArg::Stdin);
    }

    for arg in args {
        cat_arg(arg, &mut decorators);
    }
}
