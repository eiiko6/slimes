use colored::Color;
use colored::Colorize;
use sysinfo::System;

use crate::vprintln;

pub trait Slime {
    fn label(&self) -> &str;
    fn values(&self, sys: &System, verbose: bool) -> Vec<String>;
    fn icon(&self) -> &str;
    fn color(&self) -> Color;

    fn print(&self, sys: &System, verbose: bool) {
        for (i, val) in self.values(sys, verbose).iter().enumerate() {
            if i == 0 {
                print!(
                    "{} {:<10} ",
                    self.icon().color(self.color()),
                    format!("{}:", self.label()).bold().color(self.color())
                );
            } else {
                print!("  {:<10} ", " ");
            }
            println!("{}", val.white());
        }
    }

    fn print_from_values(&self, values: &Vec<String>) {
        for (i, val) in values.iter().enumerate() {
            if i == 0 {
                print!(
                    "{} {:<10} ",
                    self.icon().color(self.color()),
                    format!("{}:", self.label()).bold().color(self.color())
                );
            } else {
                print!("  {:<10} ", " ");
            }
            println!("{}", val.white());
        }
    }
}

pub fn get_all_slimes() -> Vec<Box<dyn Slime>> {
    let mut slimes: Vec<Box<dyn Slime>> = vec![
        Box::new(OsSlime),
        Box::new(KernelSlime),
        Box::new(HostnameSlime),
        Box::new(BoardSlime),
        Box::new(CpuSlime),
        Box::new(GpuSlime),
        Box::new(RamSlime),
        Box::new(NetworkSlime),
    ];

    #[cfg(feature = "monitors")]
    slimes.push(Box::new(MonitorSlime));

    #[cfg(feature = "audio")]
    slimes.push(Box::new(AudioSlime));

    slimes
}

/// OS name
pub struct OsSlime;
impl Slime for OsSlime {
    fn label(&self) -> &str {
        "OS"
    }
    fn values(&self, _sys: &System, _verbose: bool) -> Vec<String> {
        vec![format!(
            "{}",
            System::long_os_version().unwrap_or_else(|| "Unknown".into()),
        )]
    }
    fn icon(&self) -> &str {
        ""
    }
    fn color(&self) -> Color {
        Color::Blue
    }
}

/// Kernel version
pub struct KernelSlime;
impl Slime for KernelSlime {
    fn label(&self) -> &str {
        "Kernel"
    }
    fn values(&self, _sys: &System, _verbose: bool) -> Vec<String> {
        vec![System::kernel_version().unwrap_or_else(|| "Unknown".into())]
    }
    fn icon(&self) -> &str {
        ""
    }
    fn color(&self) -> Color {
        Color::Blue
    }
}

/// Hostname
pub struct HostnameSlime;
impl Slime for HostnameSlime {
    fn label(&self) -> &str {
        "Hostname"
    }
    fn values(&self, _sys: &System, _verbose: bool) -> Vec<String> {
        vec![System::host_name().unwrap_or_else(|| "Unknown".into())]
    }
    fn icon(&self) -> &str {
        "󰒋"
    }
    fn color(&self) -> Color {
        Color::Blue
    }
}

/// CPU name with highest frequency found in all cores
pub struct CpuSlime;
impl Slime for CpuSlime {
    fn label(&self) -> &str {
        "CPU"
    }
    fn values(&self, sys: &System, verbose: bool) -> Vec<String> {
        // sys.cpus()
        //     .iter()
        //     .map(|cpu| format!("{} @ {:.2}GHz", cpu.name(), cpu.frequency() as f32 / 1000.0))
        //     .collect()

        vprintln!(verbose, "Querying and mapping CPU info");

        let cpus = sys.cpus();

        let max_freq_mhz = cpus.iter().map(|c| c.frequency()).max().unwrap_or(0) as f32 / 1000.0;

        if let Some(cpu) = cpus.first() {
            vec![format!(
                "{} @ ~{:.2}GHz",
                cpu.brand(),
                // c.vendor_id(),
                max_freq_mhz
            )]
        } else {
            vec![String::from("Unknown")]
        }
    }
    fn icon(&self) -> &str {
        ""
    }
    fn color(&self) -> Color {
        Color::Yellow
    }
}

/// RAM usage over available
pub struct RamSlime;
impl Slime for RamSlime {
    fn label(&self) -> &str {
        "RAM"
    }
    fn values(&self, sys: &System, _verbose: bool) -> Vec<String> {
        let total_ram = sys.total_memory() / 1024 / 1024;
        let used_ram = sys.used_memory() / 1024 / 1024;
        vec![format!(
            "{}MB / {}MB",
            used_ram.to_string(),
            total_ram.to_string()
        )]
    }
    fn icon(&self) -> &str {
        ""
    }
    fn color(&self) -> Color {
        Color::Magenta
    }
}

/// Motherboard / chassis
pub struct BoardSlime;
impl Slime for BoardSlime {
    fn label(&self) -> &str {
        "Board"
    }
    fn icon(&self) -> &str {
        ""
    }
    fn color(&self) -> Color {
        Color::Green
    }
    fn values(&self, _sys: &System, _verbose: bool) -> Vec<String> {
        let Some(mobo) = sysinfo::Motherboard::new() else {
            return vec!["Unknown Model".into()];
        };

        let parts = [
            mobo.vendor_name(),
            mobo.version().filter(|v| v != "Default string"),
            mobo.name(),
        ];

        let result = parts
            .into_iter()
            .flatten()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        vec![if result.is_empty() {
            "Unknown Model".into()
        } else {
            result
        }]
    }
}

/// GPUs
pub struct GpuSlime;
impl Slime for GpuSlime {
    fn label(&self) -> &str {
        "GPU"
    }
    fn icon(&self) -> &str {
        "󰢮"
    }
    fn color(&self) -> Color {
        Color::Cyan
    }
    fn values(&self, _sys: &System, verbose: bool) -> Vec<String> {
        let mut gpus = Vec::new();

        #[cfg(target_os = "windows")]
        {
            use std::process::Command;

            vprintln!(
                verbose,
                r#"Executing `wmic path win32_VideoController get name`"#
            );

            if let Ok(output) = Command::new("wmic")
                .args(["path", "win32_VideoController", "get", "name"])
                .output()
            {
                let s = String::from_utf8_lossy(&output.stdout);
                for line in s.lines().skip(1) {
                    let name = line.trim();
                    if !name.is_empty() {
                        gpus.push(name.to_string());
                    }
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            use std::process::Command;

            vprintln!(
                verbose,
                r#"Executing `sh -c "lspci | grep -E 'VGA|3D'"` and formatting"#
            );

            if let Ok(output) = Command::new("sh")
                .arg("-c")
                .arg("lspci | grep -E 'VGA|3D'")
                .output()
            {
                let s = String::from_utf8_lossy(&output.stdout);
                for line in s.lines() {
                    if let Some(pos) = line.find(": ") {
                        gpus.push(
                            line[pos + 2..]
                                .to_string()
                                .replace("Advanced Micro Devices, Inc. [AMD/ATI]", "AMD"),
                        );
                    }
                }
            }
        }

        if gpus.is_empty() {
            vec!["Unknown GPU".into()]
        } else {
            gpus
        }
    }
}

/// Monitors
#[cfg(feature = "monitors")]
pub struct MonitorSlime;
#[cfg(feature = "monitors")]
impl Slime for MonitorSlime {
    fn label(&self) -> &str {
        "Monitors"
    }
    fn icon(&self) -> &str {
        "󰍹"
    }
    fn color(&self) -> Color {
        Color::Blue
    }
    fn values(&self, _sys: &System, _verbose: bool) -> Vec<String> {
        // vprintln!(verbose, "Querying and mapping monitors");

        match display_info::DisplayInfo::all() {
            Ok(displays) => displays
                .iter()
                .map(|d| {
                    format!(
                        "{} {}x{} @ {}Hz {}",
                        d.friendly_name,
                        d.width,
                        d.height,
                        d.frequency,
                        if d.is_primary {
                            "[Primary]"
                        } else {
                            "[External]"
                        }
                    )
                })
                .collect(),
            Err(_) => vec!["No monitors found".into()],
        }
    }
}

/// Network (wifi/ethernet) hardware
pub struct NetworkSlime;
impl Slime for NetworkSlime {
    fn label(&self) -> &str {
        "Network"
    }
    fn icon(&self) -> &str {
        "󰖩"
    }
    fn color(&self) -> Color {
        Color::Cyan
    }
    fn values(&self, _sys: &System, verbose: bool) -> Vec<String> {
        let mut nets = Vec::new();

        #[cfg(target_os = "windows")]
        {
            use std::process::Command;

            vprintln!(
                verbose,
                r#"Executing `wmic path win32_networkadapter where PhysicalAdapter=True get name`"#
            );

            if let Ok(output) = Command::new("wmic")
                .args([
                    "path",
                    "win32_networkadapter",
                    "where",
                    "PhysicalAdapter=True",
                    "get",
                    "name",
                ])
                .output()
            {
                let s = String::from_utf8_lossy(&output.stdout);
                for line in s.lines().skip(1) {
                    let name = line.trim();
                    if !name.is_empty() {
                        nets.push(name.to_string());
                    }
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            use std::process::Command;

            vprintln!(
                verbose,
                r#"Executing `sh -c "lspci | grep -E 'Network|Ethernet'"` and formatting"#
            );

            if let Ok(output) = Command::new("sh")
                .arg("-c")
                .arg("lspci | grep -E 'Network|Ethernet'")
                .output()
            {
                let s = String::from_utf8_lossy(&output.stdout);
                for line in s.lines() {
                    if let Some(pos) = line.find(": ") {
                        nets.push(line[pos + 2..].trim().to_string());
                    }
                }
            }
        }

        if nets.is_empty() {
            vec!["Unknown Network Controller".into()]
        } else {
            nets
        }
    }
}

/// Audio hardware
#[cfg(feature = "audio")]
pub struct AudioSlime;
#[cfg(feature = "audio")]
impl Slime for AudioSlime {
    fn label(&self) -> &str {
        "Audio"
    }
    fn icon(&self) -> &str {
        "󰓃"
    }
    fn color(&self) -> Color {
        Color::Red
    }
    fn values(&self, _sys: &System, verbose: bool) -> Vec<String> {
        let mut audio_cards = Vec::new();

        #[cfg(target_os = "windows")]
        {
            use std::process::Command;

            vprintln!(
                verbose,
                r#"Executing `wmic path win32_sounddevice get name`"#
            );

            if let Ok(output) = Command::new("wmic")
                .args(["path", "win32_sounddevice", "get", "name"])
                .output()
            {
                let s = String::from_utf8_lossy(&output.stdout);
                for line in s.lines().skip(1) {
                    let name = line.trim();
                    if !name.is_empty() && name != "Microsoft Streaming Service Proxy" {
                        audio_cards.push(name.to_string());
                    }
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            use std::process::Command;

            vprintln!(
                verbose,
                r#"Executing `sh -c "lspci | grep -E 'Audio'"` and formatting"#
            );

            if let Ok(output) = Command::new("sh")
                .arg("-c")
                .arg("lspci | grep -i 'Audio'")
                .output()
            {
                let s = String::from_utf8_lossy(&output.stdout);
                for line in s.lines() {
                    if let Some(pos) = line.find(": ") {
                        audio_cards.push(
                            line[pos + 2..]
                                .trim()
                                .to_string()
                                .replace("Advanced Micro Devices, Inc. [AMD/ATI]", "AMD")
                                .replace("Advanced Micro Devices, Inc. [AMD]", "AMD"),
                        );
                    }
                }
            }
        }

        if audio_cards.is_empty() {
            vec!["No hardware audio found".into()]
        } else {
            // Deduplicate in case multiple logical devices refer to one controller
            audio_cards.dedup();
            audio_cards
        }
    }
}
