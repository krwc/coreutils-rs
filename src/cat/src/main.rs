use std::env;
use std::io::{self, Write, BufReader};
use std::fs;
use std::path;

#[macro_use(die)]
extern crate utils;
extern crate getopts;

pub struct Decorators {
    ends: bool,
    number: bool,
    squeeze: bool,
}

pub struct State {
    empty_streak: i32,
    current_line: i32,
}

impl Decorators {
    fn any(&self) -> bool {
        self.ends || self.number || self.squeeze
    }
}

fn copy_raw(from: &mut std::io::Read) -> io::Result<u64> {
    io::copy(from, &mut io::stdout())
}

fn copy_decorated(
    state: &mut State,
    reader: &mut std::io::Read,
    decorators: &Decorators,
    interactive: bool,
) -> io::Result<()> {
    const BUFSIZE: usize = 65536;
    let stdout = io::stdout();
    let mut writer = io::BufWriter::with_capacity(2 * BUFSIZE, stdout.lock());
    let mut input: [u8; BUFSIZE] = [0u8; BUFSIZE];

    while let Ok(len) = reader.read(&mut input) {
        if len == 0 {
            break;
        }

        let mut p = 0;
        while p < len {
            // Attempt to minimize write calls by looking ahead for '\n' character.
            let newline_offset = match input[p..].iter().position(|c| *c == b'\n') {
                Some(q) => q as i32,
                None => -1,
            };

            if newline_offset < 0 {
                // New line not found. We can write entire chunk of data at once.
                writer.write_all(&input[p..])?;
                state.empty_streak = 0;
                break;
            }

            if newline_offset == 0 {
                state.empty_streak += 1;
            } else {
                state.empty_streak = 1;
            }

            if decorators.squeeze && state.empty_streak >= 3 {
                p += 1;
                continue;
            }
            if decorators.number {
                write!(&mut writer, "{:6}: ", state.current_line)?;
                state.current_line += 1;
            }
            // Write everything till the new line.
            writer.write_all(&input[p..p + newline_offset as usize])?;

            if decorators.ends {
                writer.write_all(&[b'$'])?;
            }
            writer.write_all(&[b'\n'])?;
            p += 1 + newline_offset as usize;

            if interactive {
                writer.flush()?;
            }
        }
    }
    Ok(())
}

fn copy_or_die(
    state: &mut State,
    from: &mut std::io::Read,
    decorators: &Decorators,
    interactive: bool,
) {
    if decorators.any() {
        copy_decorated(state, from, decorators, interactive).unwrap();
    } else {
        copy_raw(from).unwrap();
    }
}

fn get_file(name: &str) -> io::BufReader<fs::File> {
    match path::Path::new(name).metadata() {
        Err(e) => {
            match e.kind() {
                io::ErrorKind::NotFound => die!("{}: no such file or directory", name),
                io::ErrorKind::PermissionDenied => die!("{}: permission denied", name),
                _ => die!("{}: unknown error", name),

            }
        }
        Ok(info) => {
            if info.is_dir() {
                die!("{}: is a directory", name);
            }
        }
    };

    match fs::File::open(name) {
        Err(_) => die!("{}: unknown error", name),
        Ok(f) => BufReader::new(f),
    }
}

fn cat_file(state: &mut State, file: &str, decorators: &Decorators) {
    if file == "-" {
        copy_or_die(state, &mut io::stdin(), decorators, true);
    } else {
        copy_or_die(state, &mut get_file(file), decorators, false);
    }
}

fn show_help(opts: &getopts::Options) {
    let brief =
        format!(
        "Usage: {}: [OPTION]... [FILENAME]...\n{}",
        env::args().nth(0).unwrap(),
        "Partial implementation of standard GNU cat. Concatenates FILE(s) to standard output.",
    );
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut opts = getopts::Options::new();
    opts.optflag("h", "help", "show this message and exit");
    opts.optflag("n", "number", "number all output lines");
    opts.optflag("E", "show-ends", "display $ at end of each line");
    opts.optflag(
        "s",
        "squeeze-blank",
        "squeeze consecutive empty lines into one",
    );
    opts.optflag("v", "version", "output version information and exit");
    let options = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => die!("{}", f.to_string()),
    };

    if options.opt_present("h") {
        return show_help(&opts);
    }
    if options.opt_present("v") {
        return println!(
            "Partial implementation of GNU cat, version {}",
            env!("CARGO_PKG_VERSION")
        );
    }
    let decorators = Decorators {
        ends: options.opt_present("E"),
        number: options.opt_present("n"),
        squeeze: options.opt_present("s"),
    };

    let mut files: Vec<String> = vec![];
    if options.free.is_empty() {
        files.push("-".to_owned());
    } else {
        files.append(&mut options.free.clone());
    }

    let mut state = State {
        empty_streak: 1,
        current_line: 1,
    };
    for file in files {
        cat_file(&mut state, &file, &decorators);
    }
}
