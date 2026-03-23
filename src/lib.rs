// use colored::Colorize;

mod benchmark;
mod slimes;

pub fn application_header() -> &'static str {
    let ascii_art = r#"
      .---.
    .'     '.     < CPU SLIME >
   /   ^ ^   \
  :     v     :   (Benchmarking)
  |           |
   \         /
    '._____.'
    "#;
    // println!("{}", ascii_art.bright_green().bold());
    ascii_art
}
