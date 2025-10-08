// src/main.rs - Stable Hybrid Version

mod config;
mod github;
mod billing;

use std::thread;
use std::time::{Duration, Instant};
use std::env;

const STATE_FILE: &str = "state.json";
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(3 * 3600 + 30 * 60); // 3.5 hours

fn show_status() {
    println!("╔════════════════════════════════════════════════╗");
    println!("║        ORCHESTRATOR STATUS                    ║");
    println!("╚════════════════════════════════════════════════╝");
    
    match config::load_state(STATE_FILE) {
        Ok(state) => {
            println!("State file found");
            println!("Current Token Index: {}", state.current_account_index);
            if !state.mawari_node_1_name.is_empty() {
                println!("Node 1: {}", state.mawari_node_1_name);
            }
            if !state.mawari_node_2_name.is_empty() {
                println!("Node 2: {}", state.mawari_node_2_name);
            }
        }
        Err(_) => {
            println!("No state file found");
        }
    }
    
    println!("\nTokens Available:");
    match config::load_config("tokens.json") {
        Ok(cfg) => {
            println!("   Total: {} tokens", cfg.tokens.len());
        }
        Err(e) => {
            eprintln!("   Error: {}", e);
        }
    }
}

fn verify_current() {
    println!("╔════════════════════════════════════════════════╗");
    println!("║        NODE VERIFICATION                      ║");
    println!("╚════════════════════════════════════════════════╝");
    
    let state = match config::load_state(STATE_FILE) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("No state file found");
            return;
        }
    };
    
    let config = match config::load_config("tokens.json") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error loading tokens: {}", e);
            return;
        }
    };
    
    if state.current_account_index >= config.tokens.len() {
        eprintln!("Invalid token index");
        return;
    }
    
    let token = &config.tokens[state.current_account_index];
    
    println!("Token Index: {}", state.current_account_index);
    
    if !state.mawari_node_1_name.is_empty() {
        println!("\n🔍 Verifying Node 1: {}", state.mawari_node_1_name);
        match github::verify_codespace(token, &state.mawari_node_1_name) {
            Ok(true) => println!("   ✅ RUNNING & AVAILABLE"),
            Ok(false) => println!("   ⚠️ NOT AVAILABLE or STOPPED"),
            Err(e) => eprintln!("   ❌ Error: {}", e),
        }
    }
    
    if !state.mawari_node_2_name.is_empty() {
        println!("\n🔍 Verifying Node 2: {}", state.mawari_node_2_name);
        match github::verify_codespace(token, &state.mawari_node_2_name) {
            Ok(true) => println!("   ✅ RUNNING & AVAILABLE"),
            Ok(false) => println!("   ⚠️ NOT AVAILABLE or STOPPED"),
            Err(e) => eprintln!("   ❌ Error: {}", e),
        }
    }
}

fn restart_nodes(token: &str, name1: &str, name2: &str, repo_name: &str) {
    println!("\n╔════════════════════════════════════════════════╗");
    println!("║        KEEP-ALIVE CYCLE                       ║");
    println!("╚════════════════════════════════════════════════╝");
    
    let repo_basename = repo_name.split('/').last().unwrap_or("mawari-multi-wallet");
    let script_path = format!("/workspaces/{}/auto-start.sh", repo_basename);
    let cmd = format!("bash -l -c 'bash {}'", script_path);

    if !name1.is_empty() {
        println!("  🔄 Restarting Node 1: {}", name1);
        match github::ssh_command(token, name1, &cmd) {
            Ok(_) => println!("    ✅ Restart successful"),
            Err(e) => eprintln!("    ⚠️ Warning: {}", e),
        }
        thread::sleep(Duration::from_secs(5));
    }
    
    if !name2.is_empty() {
        println!("  🔄 Restarting Node 2: {}", name2);
        match github::ssh_command(token, name2, &cmd) {
            Ok(_) => println!("    ✅ Restart successful"),
            Err(e) => eprintln!("    ⚠️ Warning: {}", e),
        }
    }
    
    println!("\n✅ Keep-alive cycle completed!\n");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 && args[1] == "status" {
        show_status();
        return;
    }
    
    if args.len() > 1 && args[1] == "verify" {
        verify_current();
        return;
    }
    
    if args.len() < 2 {
        eprintln!("❌ ERROR: Repository argument missing!");
        eprintln!("Usage: cargo run --release -- username/repo-name");
        eprintln!("   or: cargo run --release -- status");
        eprintln!("   or: cargo run --release -- verify");
        return;
    }
    
    let repo_name = &args[1];

    println!("╔════════════════════════════════════════════════╗");
    println!("║   MAWARI 12-NODE MULTI-WALLET ORCHESTRATOR    ║");
    println!("║            Stable Hybrid Version              ║");
    println!("╚════════════════════════════════════════════════╝");
    println!("📦 Repository: {}", repo_name);
    println!("");

    let config = match config::load_config("tokens.json") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ FATAL: {}", e);
            eprintln!("   Create tokens.json with your GitHub tokens");
            return;
        }
    };

    if config.tokens.is_empty() {
        eprintln!("❌ FATAL: No tokens in tokens.json");
        return;
    }

    println!("✅ Loaded {} token(s)", config.tokens.len());

    let mut state = config::load_state(STATE_FILE).unwrap_or_default();
    let mut i = state.current_account_index;
    
    if i >= config.tokens.len() {
        println!("⚠️ Resetting invalid index {} to 0", i);
        i = 0;
    }

    let mut consecutive_failures = 0;
    const MAX_FAILURES: usize = 3;

    println!("\n🚀 Starting orchestration loop...\n");

    loop {
        let token = &config.tokens[i];
        
        println!("\n╔════════════════════════════════════════════════╗");
        println!("║           TOKEN #{:<2} of {:<2}                   ║", i + 1, config.tokens.len());
        println!("╚════════════════════════════════════════════════╝");

        // Validate token
        let username = match github::get_username(token) {
            Ok(u) => {
                println!("✅ Valid token for: @{}", u);
                consecutive_failures = 0;
                u
            },
            Err(e) => {
                eprintln!("❌ Token error: {}", e);
                consecutive_failures += 1;
                
                if consecutive_failures >= MAX_FAILURES {
                    eprintln!("\n⚠️ Too many failures ({}). Cooldown 10 min...", consecutive_failures);
                    thread::sleep(Duration::from_secs(600));
                    consecutive_failures = 0;
                }
                
                i = (i + 1) % config.tokens.len();
                state.current_account_index = i;
                config::save_state(STATE_FILE, &state).ok();
                thread::sleep(Duration::from_secs(5));
                continue;
            }
        };

        // Check billing
        println!("\n📊 Checking billing quota...");
        let billing = match billing::get_billing_info(token, &username) {
            Ok(b) => b,
            Err(_) => {
                eprintln!("⚠️ Billing check failed. Assuming exhausted...");
                i = (i + 1) % config.tokens.len();
                state.current_account_index = i;
                config::save_state(STATE_FILE, &state).ok();
                thread::sleep(Duration::from_secs(5));
                continue;
            }
        };

        billing::display_billing(&billing, &username);

        if !billing.is_quota_ok {
            eprintln!("\n⚠️ Insufficient quota for @{}", username);
            eprintln!("   Switching to next account...\n");
            i = (i + 1) % config.tokens.len();
            state.current_account_index = i;
            config::save_state(STATE_FILE, &state).ok();
            thread::sleep(Duration::from_secs(5));
            continue;
        }

        // Deploy/ensure codespaces
        println!("\n🚀 Ensuring healthy codespaces for @{}...", username);
        let (node1_name, node2_name) = match github::ensure_healthy_codespaces(token, repo_name) {
            Ok(names) => {
                consecutive_failures = 0;
                names
            },
            Err(e) => {
                eprintln!("\n❌ Deployment failed: {}", e);
                consecutive_failures += 1;
                
                if consecutive_failures >= MAX_FAILURES {
                    eprintln!("\n⚠️ Too many deployment failures. Cooldown 15 min...");
                    thread::sleep(Duration::from_secs(900));
                    consecutive_failures = 0;
                } else {
                    eprintln!("   Retrying in 5 min...");
                    thread::sleep(Duration::from_secs(300));
                }
                continue;
            }
        };

        // Success
        println!("\n╔════════════════════════════════════════════════╗");
        println!("║         DEPLOYMENT SUCCESSFUL! 🎉              ║");
        println!("╚════════════════════════════════════════════════╝");
        println!("Account: @{}", username);
        println!("Node 1:  {}", node1_name);
        println!("Node 2:  {}", node2_name);
        
        state.current_account_index = i;
        state.mawari_node_1_name = node1_name.clone();
        state.mawari_node_2_name = node2_name.clone();
        config::save_state(STATE_FILE, &state).ok();

        // Calculate run duration
        let run_duration_hours = (billing.hours_remaining - 0.5).max(0.5).min(20.0);
        let run_duration = Duration::from_secs((run_duration_hours * 3600.0) as u64);
        
        println!("\n⏱️ Running for {:.1} hours", run_duration_hours);
        println!("   Keep-alive interval: {:.1}h", KEEP_ALIVE_INTERVAL.as_secs() as f32 / 3600.0);
        
        // Keep-alive loop
        let start_time = Instant::now();
        let mut cycle_count = 0;
        
        while start_time.elapsed() < run_duration {
            let remaining = run_duration.saturating_sub(start_time.elapsed());
            let sleep_for = std::cmp::min(remaining, KEEP_ALIVE_INTERVAL);
            
            if sleep_for.as_secs() < 60 {
                println!("\n⏰ Time's up! Switching account...");
                break;
            }

            let hours_left = remaining.as_secs() as f32 / 3600.0;
            println!("\n💤 Sleeping {:.1}h (remaining: {:.1}h)...", 
                sleep_for.as_secs() as f32 / 3600.0, hours_left);
            
            thread::sleep(sleep_for);

            if start_time.elapsed() >= run_duration {
                break;
            }
            
            cycle_count += 1;
            println!("\n🔄 Keep-alive cycle #{}", cycle_count);
            restart_nodes(token, &node1_name, &node2_name, repo_name);
        }
        
        // Cycle complete
        println!("\n╔════════════════════════════════════════════════╗");
        println!("║         CYCLE COMPLETE                        ║");
        println!("╚════════════════════════════════════════════════╝");
        println!("Account: @{}", username);
        println!("Duration: {:.1}h", run_duration_hours);
        println!("Keep-alive cycles: {}", cycle_count);
        println!("⏭️ Switching to next token...\n");
        
        i = (i + 1) % config.tokens.len();
        state.current_account_index = i;
        config::save_state(STATE_FILE, &state).ok();
        
        println!("⏸️ Cooldown 30s...");
        thread::sleep(Duration::from_secs(30));
    }
}
