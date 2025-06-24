use anyhow::Result;
use console::style;

pub async fn stop_command() -> Result<()> {
    println!("{}", style("Stopping Watchtower...").cyan());

    // Try to find and stop the running process
    match find_watchtower_process().await {
        Some(pid) => {
            println!(
                "{} Found running process (PID: {})",
                style("✓").green(),
                pid
            );

            if stop_process(pid).await? {
                println!(
                    "{} Watchtower stopped successfully",
                    style("✓").green().bold()
                );

                // Clean up PID file if it exists
                cleanup_pid_file().await?;

                println!(
                    "{}",
                    style("All monitoring activities have been terminated.").dim()
                );
            } else {
                println!("{} Failed to stop process", style("✗").red().bold());
                std::process::exit(1);
            }
        }
        None => {
            println!("{} No running Watchtower process found", style("ⓘ").blue());

            // Check if there's a stale PID file
            if let Some(stale_pid) = check_stale_pid_file().await? {
                println!(
                    "{} Cleaning up stale PID file (PID: {})",
                    style("⚠️").yellow(),
                    stale_pid
                );
                cleanup_pid_file().await?;
            }

            println!("{}", style("Watchtower is not currently running.").dim());
        }
    }

    Ok(())
}

async fn find_watchtower_process() -> Option<u32> {
    // First, try to check if the metrics endpoint is responding
    if reqwest::get("http://127.0.0.1:9090/metrics").await.is_ok() {
        // Try to find the process by name
        #[cfg(unix)]
        {
            if let Ok(output) = tokio::process::Command::new("pgrep")
                .arg("-f")
                .arg("watchtower")
                .output()
                .await
            {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if let Ok(pid) = stdout.trim().parse::<u32>() {
                        return Some(pid);
                    }
                }
            }
        }

        #[cfg(windows)]
        {
            if let Ok(output) = tokio::process::Command::new("tasklist")
                .arg("/FI")
                .arg("IMAGENAME eq watchtower.exe")
                .arg("/FO")
                .arg("CSV")
                .output()
                .await
            {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    // Parse CSV output to extract PID
                    for line in stdout.lines().skip(1) {
                        let fields: Vec<&str> = line.split(',').collect();
                        if fields.len() >= 2 {
                            if let Ok(pid) = fields[1].trim_matches('"').parse::<u32>() {
                                return Some(pid);
                            }
                        }
                    }
                }
            }
        }
    }

    // Check PID file
    if let Ok(pid) = read_pid_file().await {
        if is_process_running(pid).await {
            return Some(pid);
        }
    }

    None
}

async fn stop_process(pid: u32) -> Result<bool> {
    println!(
        "{} Sending termination signal to process {}",
        style("Stopping").cyan(),
        pid
    );

    #[cfg(unix)]
    {
        // Send SIGTERM first
        if let Ok(mut child) = tokio::process::Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .spawn()
        {
            let _ = child.wait().await;

            // Wait a few seconds for graceful shutdown
            for i in 0..10 {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                if !is_process_running(pid).await {
                    println!("{} Process terminated gracefully", style("✓").green());
                    return Ok(true);
                }
                if i == 4 {
                    println!("{} Waiting for graceful shutdown...", style("⏳").yellow());
                }
            }

            // If still running, send SIGKILL
            println!("{} Force killing process...", style("⚠️").yellow());
            if let Ok(mut child) = tokio::process::Command::new("kill")
                .arg("-KILL")
                .arg(pid.to_string())
                .spawn()
            {
                let _ = child.wait().await;
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

                if !is_process_running(pid).await {
                    println!("{} Process force killed", style("✓").green());
                    return Ok(true);
                }
            }
        }
    }

    #[cfg(windows)]
    {
        if let Ok(mut child) = tokio::process::Command::new("taskkill")
            .arg("/PID")
            .arg(pid.to_string())
            .arg("/T")
            .spawn()
        {
            let _ = child.wait().await;
            tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

            if !is_process_running(pid).await {
                println!("{} Process terminated", style("✓").green());
                return Ok(true);
            }

            // Force kill if needed
            if let Ok(mut child) = tokio::process::Command::new("taskkill")
                .arg("/PID")
                .arg(pid.to_string())
                .arg("/F")
                .spawn()
            {
                let _ = child.wait().await;
                return Ok(true);
            }
        }
    }

    Ok(false)
}

async fn is_process_running(pid: u32) -> bool {
    #[cfg(unix)]
    {
        if let Ok(output) = tokio::process::Command::new("ps")
            .arg("-p")
            .arg(pid.to_string())
            .output()
            .await
        {
            return output.status.success();
        }
    }

    #[cfg(windows)]
    {
        if let Ok(output) = tokio::process::Command::new("tasklist")
            .arg("/FI")
            .arg(&format!("PID eq {}", pid))
            .output()
            .await
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout.contains(&pid.to_string());
        }
    }

    false
}

async fn read_pid_file() -> Result<u32> {
    let pid_path = get_pid_file_path();
    let content = tokio::fs::read_to_string(&pid_path).await?;
    Ok(content.trim().parse()?)
}

async fn check_stale_pid_file() -> Result<Option<u32>> {
    let pid_path = get_pid_file_path();

    if pid_path.exists() {
        match read_pid_file().await {
            Ok(pid) => {
                if !is_process_running(pid).await {
                    return Ok(Some(pid));
                }
            }
            Err(_) => {
                // Invalid PID file
                return Ok(Some(0));
            }
        }
    }

    Ok(None)
}

async fn cleanup_pid_file() -> Result<()> {
    let pid_path = get_pid_file_path();

    if pid_path.exists() {
        tokio::fs::remove_file(&pid_path).await?;
        println!("{} Cleaned up PID file", style("✓").green());
    }

    Ok(())
}

fn get_pid_file_path() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap())
        .join("watchtower.pid")
}
