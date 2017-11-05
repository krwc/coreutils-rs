pub mod utils {

    #[macro_export]
    macro_rules! die {
        ($fmt:expr, $($arg:tt)*) => ({
            eprintln!(concat!("{}: ", $fmt), std::env::args().nth(0).unwrap(), $($arg)*);
            ::std::process::exit(1);
        });
        ($fmt:expr) => ({
            eprintln!(concat!("{}: ", $fmt), std::env::args().nth(0).unwrap());
            ::std::process::exit(1);
        });
    }

}
