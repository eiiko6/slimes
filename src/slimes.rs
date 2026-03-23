use colored::Color;
use colored::Colorize;

trait Slime {
    fn label(&self) -> String;
    fn value(&self) -> String;
    fn icon(&self) -> String;
    fn color(&self) -> Color;

    fn print(&self) {
        println!(
            "{} {:<12} {}",
            self.icon().color(self.color()),
            format!("{}:", self.label()).bold().color(self.color()),
            self.value().white()
        );
    }
}
