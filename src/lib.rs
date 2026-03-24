// use colored::Colorize;

pub mod benchmark;
pub mod slimes;

#[macro_export]
macro_rules! vprintln {
    ($verbose:expr, $($arg:tt)*) => {
        if $verbose {
            println!("{}", format!($($arg)*).dimmed());
        }
    };
}

pub fn application_header() -> &'static str {
    r#"
      .---.
    .'     '.     < CPU SLIME >
   /   ^ ^   \
  :     v     :   (Benchmarking)
  |           |
   \         /
    '._____.'
    "#
    // .bright_green()
    // .bold()
}
