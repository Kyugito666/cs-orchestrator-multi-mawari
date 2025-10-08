// src/billing.rs

use std::process::Command;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct BillingInfo {
    pub total_core_hours_used: f32,
    pub hours_remaining: f32,
    pub is_quota_ok: bool,
}

#[derive(Deserialize, Debug)]
struct UsageItem {
    product: String,
    sku: String,
    quantity: f32,
}

#[derive(Deserialize, Debug)]
struct BillingReport {
    #[serde(rename = "usageItems")]
    usage_items: Vec<UsageItem>,
}

fn run_gh_api(token: &str, endpoint: &str) -> Result<String, String> {
    let output = Command::new("gh")
        .args(&["api", endpoint, "-H", "Accept: application/vnd.github+json"])
        .env("GH_TOKEN", token)
        .output()
        .map_err(|e| format!("Failed to execute gh: {}", e))?;

    // PERBAIKAN DI SINI
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr.to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn get_billing_info(token: &str, username: &str) -> Result<BillingInfo, String> {
    let endpoint = format!("/users/{}/settings/billing/usage", username);
    
    let response = match run_gh_api(token, &endpoint) {
        Ok(r) => r,
        Err(e) => {
            println!("   WARNING: Gagal menghubungi API billing ({}). Anggap kuota habis.", e.lines().next().unwrap_or("API error"));
            return Ok(BillingInfo {
                total_core_hours_used: 999.0,
                hours_remaining: 0.0,
                is_quota_ok: false,
            });
        }
    };
    
    if let Ok(report) = serde_json::from_str::<BillingReport>(&response) {
        let mut total_core_hours_used = 0.0;

        for item in report.usage_items {
            if item.product == "codespaces" {
                if item.sku.contains("compute 2-core") {
                    total_core_hours_used += item.quantity * 2.0;
                } else if item.sku.contains("compute 4-core") {
                    total_core_hours_used += item.quantity * 4.0;
                }
            }
        }
        
        let included_core_hours = 120.0;
        let remaining_core_hours = included_core_hours - total_core_hours_used;
        
        // Dengan 2x 4-core (total 8 core), sisa jam dihitung dengan total core hours / 8
        let hours_remaining = (remaining_core_hours / 8.0).max(0.0);
        
        // Dianggap OK jika sisa waktu lebih dari 1 jam
        let is_quota_ok = hours_remaining > 1.0;
        
        return Ok(BillingInfo {
            total_core_hours_used,
            hours_remaining,
            is_quota_ok,
        });
    }

    println!("   WARNING: Format data billing tidak dikenal atau kosong. Anggap kuota habis.");
    Ok(BillingInfo {
        total_core_hours_used: 999.0,
        hours_remaining: 0.0,
        is_quota_ok: false,
    })
}

pub fn display_billing(billing: &BillingInfo, username: &str) {
    println!("Billing @{}: Used ~{:.1} of 120.0 core-hours | Approx. {:.1}h remaining (for 8-core usage)", 
        username, 
        billing.total_core_hours_used,
        billing.hours_remaining
    );
    
    if !billing.is_quota_ok {
        println!("   WARNING: Kuota rendah (< 1h) atau habis.");
    } else {
        println!("   Quota OK");
    }
}
