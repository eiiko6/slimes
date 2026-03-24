// use colored::Colorize;

pub mod benchmark;
pub mod slimes;

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
