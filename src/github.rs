// src/github.rs - FIXED VERSION (No Warnings)

use std::process::Command;
use std::fmt;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub enum GHError {
    CommandError(String),
    AuthError(String),
}

impl fmt::Display for GHError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GHError::CommandError(e) => write!(f, "Command failed: {}", e),
            GHError::AuthError(e) => write!(f, "Auth error: {}", e),
        }
    }
}

fn run_gh_command(token: &str, args: &[&str]) -> Result<String, GHError> {
    eprintln!("DEBUG: gh {}", args.join(" "));
    
    let output = Command::new("gh")
        .args(args)
        .env("GH_TOKEN", token)
        .output()
        .map_err(|e| GHError::CommandError(format!("Failed to execute gh: {}", e)))?;

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    
    if !stderr.is_empty() {
        eprintln!("DEBUG stderr: {}", stderr);
    }
    
    if !output.status.success() {
        if stderr.contains("Bad credentials") 
            || stderr.contains("authentication required")
            || stderr.contains("HTTP 401") {
            return Err(GHError::AuthError(stderr));
        }
        
        if stderr.contains("no codespaces found") || stdout.trim().is_empty() {
            return Ok("".to_string());
        }
        
        return Err(GHError::CommandError(stderr));
    }
    
    Ok(stdout.trim().to_string())
}

pub fn get_username(token: &str) -> Result<String, GHError> {
    run_gh_command(token, &["api", "user", "--jq", ".login"])
}

fn stop_codespace(token: &str, name: &str) -> Result<(), GHError> {
    println!("      Stopping '{}'...", name);
    match run_gh_command(token, &["codespace", "stop", "-c", name]) {
        Ok(_) => { 
            println!("      Stopped"); 
            thread::sleep(Duration::from_secs(3)); 
            Ok(()) 
        }
        Err(e) => { 
            eprintln!("      Warning while stopping: {}", e); 
            thread::sleep(Duration::from_secs(2)); 
            Ok(())
        }
    }
}

fn delete_codespace(token: &str, name: &str) -> Result<(), GHError> {
    println!("      Deleting '{}'...", name);
    for attempt in 1..=3 {
        match run_gh_command(token, &["codespace", "delete", "-c", name, "--force"]) {
            Ok(_) => { 
                println!("      Deleted"); 
                thread::sleep(Duration::from_secs(2)); 
                return Ok(()); 
            }
            Err(_) => {  // ✅ FIXED: _ instead of e
                if attempt < 3 { 
                    eprintln!("      Retry {}/3", attempt); 
                    thread::sleep(Duration::from_secs(3)); 
                } else { 
                    eprintln!("      Failed, continuing..."); 
                    return Ok(()); 
                }
            }
        }
    }
    Ok(())
}

pub fn verify_codespace(token: &str, name: &str) -> Result<bool, GHError> {
    let state_check = run_gh_command(token, &["codespace", "view", "-c", name, "--json", "state", "-q", ".state"]);
    match state_check {
        Ok(state) if state == "Available" => Ok(true),
        _ => Ok(false),
    }
}

fn health_check(token: &str, name: &str) -> bool {
    let check_cmd = "test -f /tmp/auto_start_done && echo 'healthy'";
    match run_gh_command(token, &["codespace", "ssh", "-c", name, "--", check_cmd]) {
        Ok(output) if output.contains("healthy") => true,
        _ => false,
    }
}

pub fn wait_and_run_startup_script(token: &str, name: &str, repo_name: &str) -> Result<(), GHError> {
    println!("   Verifying and starting node '{}'...", name);
    
    for attempt in 1..=10 {
        println!("      Attempt {}/10: Checking SSH readiness...", attempt);
        
        match run_gh_command(token, &["codespace", "ssh", "-c", name, "--", "echo 'ready'"]) {
            Ok(output) if output.contains("ready") => {
                println!("      SSH is ready!");
                
                let repo_basename = repo_name.split('/').last().unwrap_or("mawari-multi-wallet");
                let script_path = format!("/workspaces/{}/auto-start.sh", repo_basename);
                let exec_command = format!("bash -l -c 'bash {}'", script_path);
                
                println!("      Executing auto-start script...");
                match run_gh_command(token, &["codespace", "ssh", "-c", name, "--", &exec_command]) {
                    Ok(_) => {
                        println!("      Script execution successful");
                        return Ok(());
                    },
                    Err(e) => {
                        eprintln!("      Script warning: {}", e.to_string().lines().next().unwrap_or(""));
                        return Ok(());
                    }
                }
            },
            _ => {
                println!("      Not ready yet...");
            }
        }
        
        if attempt < 10 {
            println!("      Waiting 30s...");
            thread::sleep(Duration::from_secs(30));
        }
    }
    
    Err(GHError::CommandError(format!("Timeout: SSH not ready for '{}'", name)))
}

pub fn ensure_healthy_codespaces(token: &str, repo: &str) -> Result<(String, String), GHError> {
    println!("  Inspecting existing codespaces...");
    
    let mut node1_name = String::new();
    let mut node2_name = String::new();

    let list_output = run_gh_command(token, &["codespace", "list", "--json", "name,displayName,state"])?;
    
    if !list_output.is_empty() {
        if let Ok(codespaces) = serde_json::from_str::<Vec<serde_json::Value>>(&list_output) {
            for cs in codespaces {
                let display_name = cs["displayName"].as_str().unwrap_or("");
                let name = cs["name"].as_str().unwrap_or("").to_string();
                let state = cs["state"].as_str().unwrap_or("").to_string();

                let process_node = |current_name: &mut String, target_display: &str| -> Result<(), GHError> {
                    if display_name == target_display {
                        println!("  Found '{}': {} (State: {})", target_display, name, state);
                        
                        if state == "Available" && health_check(token, &name) {
                            println!("    Health check PASSED. Reusing.");
                            *current_name = name.clone();
                        } else {
                            println!("    Health check FAILED. Recreating...");
                            if state == "Available" || state == "Running" {
                                stop_codespace(token, &name)?;
                            }
                            delete_codespace(token, &name)?;
                            thread::sleep(Duration::from_secs(3));
                        }
                    }
                    Ok(())
                };

                process_node(&mut node1_name, "mawari-multi-node-1")?;
                process_node(&mut node2_name, "mawari-multi-node-2")?;
            }
        }
    }

    if node1_name.is_empty() {
        println!("  Creating 'mawari-multi-node-1'...");
        let new_name = run_gh_command(token, &[
            "codespace", "create", 
            "-r", repo, 
            "-m", "standardLinux32gb",
            "--display-name", "mawari-multi-node-1", 
            "--idle-timeout", "240m"
        ])?;
        
        if new_name.is_empty() { 
            return Err(GHError::CommandError("Failed to create node-1".to_string())); 
        }
        
        node1_name = new_name;
        println!("     Created: {}", node1_name);
        
        wait_and_run_startup_script(token, &node1_name, repo)?;
    }
    
    thread::sleep(Duration::from_secs(10));
    
    if node2_name.is_empty() {
        println!("  Creating 'mawari-multi-node-2'...");
        let new_name = run_gh_command(token, &[
            "codespace", "create", 
            "-r", repo, 
            "-m", "standardLinux32gb",
            "--display-name", "mawari-multi-node-2", 
            "--idle-timeout", "240m"
        ])?;
        
        if new_name.is_empty() { 
            return Err(GHError::CommandError("Failed to create node-2".to_string())); 
        }
        
        node2_name = new_name;
        println!("     Created: {}", node2_name);
        
        wait_and_run_startup_script(token, &node2_name, repo)?;
    }

    println!("\n  Both codespaces ready!");
    Ok((node1_name, node2_name))
}

pub fn ssh_command(token: &str, codespace_name: &str, cmd: &str) -> Result<String, GHError> {
    run_gh_command(token, &["codespace", "ssh", "-c", codespace_name, "--", cmd])
}
