#[macro_use(die)]
extern crate utils;
extern crate getopts;

fn show_help(opts: &getopts::Options) {
    let brief = format!(
        concat!(
            "Clone of the standard GNU seq.\n",
            "Usage: {0}: [OPTION]... LAST\n",
            "  or:  {0}: [OPTION]... FIRST LAST\n",
            "  or:  {0}: [OPTION]... FIRST INCREMENT LAST\n",
            "Print numbers from FIRST to LAST, in steps of INCREMENT."
        ),
        std::env::args().nth(0).unwrap()
    );
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut opts = getopts::Options::new();
    opts.optflag(
        "w",
        "equal-width",
        "equalize width by padding with leading zeroes",
    );
    opts.optflag("h", "help", "display this help and exit");
    opts.optflag("v", "version", "output version information and exit");
    opts.optopt(
        "f",
        "format",
        "use printf style floating-point FORMAT",
        "FORMAT",
    );
    opts.optopt(
        "s",
        "separator",
        "use STRING to separate numbers (default: \n)",
        "STRING",
    );
    let options = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => die!("{}", f.to_string()),
    };
    if options.opt_present("h") {
        return show_help(&opts);
    }
    if options.opt_present("v") {
        return println!(
            "Implementation of GNU seq, version {}",
            env!("CARGO_PKG_VERSION")
        );
    }
}
