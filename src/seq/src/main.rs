use std::cmp;
use std::collections::HashSet;

#[macro_use(die)]
extern crate utils;
extern crate getopts;

#[derive(Debug)]
pub struct SeqConfig {
    separator: String,
    equal_width: bool,
    first: f64,
    inc: f64,
    last: f64,
    format: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precision_detection() {
        assert_eq!(detect_precision("3.14"), 2);
        assert_eq!(detect_precision(""), 0);
        assert_eq!(detect_precision("314"), 0);
    }

    #[test]
    fn simple_format() {
        for fmt in vec!["%a", "%e", "%f", "%g", "%A", "%E", "%F", "%G"] {
            assert!(validate_format(fmt).is_ok());
        }
    }

    #[test]
    fn simple_precision() {
        assert!(validate_format("%.3f").is_ok());
        assert!(validate_format("%.32g").is_ok());
    }

    #[test]
    fn no_format() {
        assert!(validate_format("").is_err());
        assert!(validate_format("%").is_err());
        assert!(validate_format("%%").is_err());
        assert!(validate_format("nothing").is_err());
    }

    #[test]
    fn bad_format() {
        assert!(validate_format("%00f").is_err());
        assert!(validate_format("%c").is_err());
        assert!(validate_format("%f%n").is_err());
    }

    #[test]
    fn percent_escape() {
        assert!(validate_format("%f%%").is_ok());
        assert!(validate_format("%f%%%").is_err());
        assert!(validate_format("%f%%%%").is_ok());
        assert!(validate_format("%%f").is_err());
    }

    #[test]
    fn bad_flag() {
        assert!(validate_format("%x3f").is_err());
        assert!(validate_format("%*3f").is_err());
    }

    #[test]
    fn good_flag() {
        for flag in vec!["%0f", "%+f", "%-f", "%#f", "%+#-f", "% f"] {
            assert!(validate_format(flag).is_ok());
        }
    }

    // TODO: Write more test-cases covering width parsing too.
}

extern crate libc;
use libc::c_char;
use libc::c_int;
use libc::c_double;
use std::ffi;

#[link(name = "c")]
extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

fn print_formatted_f64(fmt: &str, value: f64) {
    // TODO: make it locale-independent
    unsafe {
        let ret = printf(ffi::CString::new(fmt).unwrap().as_ptr(), value as c_double);
        if ret < 0 {
            die!("I/O error");
        }
    }
}

fn print_string(string: &str) {
    unsafe {
        let ret = printf(
            ffi::CString::new("%s").unwrap().as_ptr(),
            ffi::CString::new(string).unwrap().as_ptr(),
        );
        if ret < 0 {
            die!("I/O error");
        }
    }
}

fn seq(config: &SeqConfig) -> FormatParseResult {
    validate_format(&config.format)?;
    let mut k: u64 = 0;
    loop {
        let current = config.first + (k as f64) * config.inc;
        if current > config.last {
            break;
        }
        if k > 0 {
            print_string(&config.separator);
        }
        print_formatted_f64(&config.format, current);
        k += 1;
    }
    print_string("\n");
    Ok(())
}

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

fn detect_precision(float: &str) -> usize {
    match float.find('.') {
        Some(n) => float.len() - n - 1,
        None => 0,
    }
}

fn parse_float(float: &str) -> f64 {
    float.parse::<f64>().unwrap_or_else(|_| {
        die!("invalid floating point argument '{}'", float)
    })
}

type FormatParseResult = Result<(), String>;

/// Consumes printf's format flags '+', '-', ' ', '#', '0' till they occur. If a flag
/// is found multiple times an error is reported.
///
/// Each consumed character increments @p index by one.
fn consume_flags_if_any(format: &[u8], index: &mut usize) -> FormatParseResult {
    // TODO: HashSet is an overkill. Somebody please stop me!
    let mut flags_found: HashSet<char> = HashSet::new();
    for ch in format {
        match *ch as char {
            '+' | '-' | ' ' | '#' | '0' => {
                if !flags_found.insert(*ch as char) {
                    return Err("duplicated format flags".into());
                }
            }
            _ => break,
        }
        *index += 1;
    }
    Ok(())
}

/// Consumes a digit sequence till digits occur, expecting at least @p minimum_digits_expected
/// digits.
///
/// Each consumed character increments @p index by one.
fn consume_digits(
    format: &[u8],
    index: &mut usize,
    minimum_digits_expected: u32,
) -> FormatParseResult {
    let mut digits_found = 0u32;
    for ch in format {
        if !(*ch as char).is_digit(10) {
            break;
        }
        digits_found += 1;
        *index += 1;
    }
    if minimum_digits_expected > digits_found {
        Err(format!(
            "expected at least {} digits to be found",
            minimum_digits_expected
        ))
    } else {
        Ok(())
    }
}

/// Consumes printf's precision specifier '.prec'.
fn consume_precision_if_any(format: &[u8], index: &mut usize) -> FormatParseResult {
    if format.len() > 0 {
        if format[0] == b'.' {
            *index += 1;
            consume_digits(&format[1..], index, 1)?;
        }
    }
    Ok(())
}

/// Consumes printf's format specifier.
fn consume_specifier(format: &[u8], index: &mut usize) -> FormatParseResult {
    if format.len() == 0 {
        return Err("empty format specifier".into());
    }
    if !vec!['a', 'e', 'f', 'g', 'A', 'E', 'F', 'G'].contains(&(format[0] as char)) {
        return Err(format!("invalid specifier '{}'", format[0] as char));
    }
    *index += 1;
    Ok(())
}

fn validate_format(format: &str) -> FormatParseResult {
    let bytes = format.as_bytes();
    let mut p = 0;
    let mut found_format = false;

    while p < bytes.len() {
        // Possibbly a format string.
        if bytes[p] == b'%' {
            let num_percents = bytes[p..].iter().take_while(|c| **c == b'%').count();

            if !found_format && num_percents == 1 {
                // We should definitely expect format string, or else the format is broken.
                p += 1;

                // printf's [flags]
                consume_flags_if_any(&bytes[p..], &mut p)?;
                // printf's [width]
                consume_digits(&bytes[p..], &mut p, 0)?;
                // printf's [.prec]
                consume_precision_if_any(&bytes[p..], &mut p)?;
                // printf's [specifier]
                consume_specifier(&bytes[p..], &mut p)?;
                found_format = true;
            } else if num_percents % 2 != 0 {
                // Not fully escaped sequence of %-signs
                return Err("unescaped sequence of '%' is invalid".into());
            } else {
                p += num_percents + 1;
            }
        } else {
            // Nothing interesting
            p += 1;
        }
    }
    if found_format {
        Ok(())
    } else {
        Err("no format found".into())
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut opts = getopts::Options::new();
    // TODO: Support this
    //  opts.optflag(
    //  "w",
    //  "equal-width",
    //  "equalize width by padding with leading zeroes",
    // );
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
        "use STRING to separate numbers (default: \\n)",
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

    if options.free.is_empty() {
        die!("missing operand");
    } else if options.free.len() > 3 {
        die!("extra operand '{}'", options.free[3])
    }

    let mut precision = 0;
    let first: f64 = if options.free.len() > 1 {
        precision = detect_precision(&options.free[0]);
        parse_float(&options.free[0])
    } else {
        1.0f64
    };
    let inc: f64 = if options.free.len() > 2 {
        precision = cmp::max(precision, detect_precision(&options.free[1]));
        parse_float(&options.free[1])
    } else {
        1.0f64
    };
    let last: f64 = if options.free.len() > 2 {
        parse_float(&options.free[2])
    } else {
        parse_float(&options.free[0])
    };

    let config = SeqConfig {
        separator: options.opt_str("s").unwrap_or("\n".into()),
        equal_width: options.opt_present("w"),
        first: first,
        inc: inc,
        last: last,
        format: options.opt_str("f").unwrap_or(
            format!("%.{}f", precision).into(),
        ),
    };

    seq(&config).unwrap_or_else(|e| {
        die!("{}", e);
    });
}
