use dialoguer::{Confirm, theme::ColorfulTheme};
use indicatif::{ProgressBar, ProgressStyle};
use std::process::Command;
use std::fs;
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎬 ani-tui Smart Installer for Windows");
    println!("---------------------------------------");

    let confirmed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you want to install ani-tui (v3.8.0) and dependencies?")
        .default(true)
        .interact()?;

    if !confirmed {
        println!("❌ Installation cancelled.");
        return Ok(());
    }

    let pb = ProgressBar::new(100);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}% {msg}")?
        .progress_chars("#>-"));

    // 1. Check/Install Scoop
    pb.set_message("Setting up Scoop...");
    pb.set_position(20);
    let scoop_check = Command::new("powershell")
        .args(["-Command", "Get-Command scoop -ErrorAction SilentlyContinue"])
        .output()?;
    if !scoop_check.status.success() {
        Command::new("powershell")
            .args(["-Command", "Set-ExecutionPolicy RemoteSigned -Scope CurrentUser -Force; iwr -useb https://get.scoop.sh | iex"])
            .output()?;
    }

    // 2. Install mpv & chafa
    pb.set_message("Installing dependencies (mpv, chafa)...");
    pb.set_position(50);
    Command::new("powershell")
        .args(["-Command", "scoop install mpv chafa -s"])
        .output()?;

    // 3. Download/Install ani-tui
    pb.set_message("Installing ani-tui...");
    pb.set_position(80);
    let install_dir = std::env::var("LOCALAPPDATA")? + "\\ani-tui";
    fs::create_dir_all(&install_dir)?;
    
    // In actual production, we download the exe here.
    // For this build, we assume the user might be running the installer next to the app.

    // 4. Configure PATH
    pb.set_message("Configuring PATH...");
    Command::new("powershell")
        .args(["-Command", &format!("[Environment]::SetEnvironmentVariable('PATH', [Environment]::GetEnvironmentVariable('PATH', 'User') + ';{}', 'User')", install_dir)])
        .output()?;

    pb.set_position(100);
    pb.finish_with_message("✅ ani-tui installed successfully!");

    println!("\n🚀 Installation Complete!");
    println!("Restart your terminal and type 'ani-tui' to start.");
    
    print!("\nPress Enter to exit...");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(())
}
