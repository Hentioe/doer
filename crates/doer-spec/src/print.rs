#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        use $crate::colored::Colorize;

        eprintln!("{}: {}", "error".red().bold(), format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        use $crate::colored::Colorize;

        eprintln!("{}: {}", "warn".yellow().bold(), format_args!($($arg)*));
    };
}
