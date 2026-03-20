use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::error::Error;
use std::fs;

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[allow(dead_code)]
    expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Activity {
    id: i64,
    name: String,
    distance: f64,
    moving_time: i32,
    elapsed_time: i32,
    total_elevation_gain: f64,
    #[serde(rename = "type")]
    activity_type: String,
    sport_type: String,
    start_date: String,
    start_date_local: String,
    timezone: String,
    trainer: bool,
    commute: bool,
    average_speed: f64,
    max_speed: f64,
    average_watts: Option<f64>,
    weighted_average_watts: Option<f64>,
    max_watts: Option<f64>,
    kilojoules: Option<f64>,
    device_watts: Option<bool>,
    has_heartrate: bool,
    average_heartrate: Option<f64>,
    max_heartrate: Option<f64>,
    average_cadence: Option<f64>,
    suffer_score: Option<f64>,
    kudos_count: i32,
    achievement_count: i32,
    pr_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActivityStreams {
    time: Option<Vec<i32>>,
    watts: Option<Vec<f64>>,
    heartrate: Option<Vec<i32>>,
    cadence: Option<Vec<i32>>,
    velocity_smooth: Option<Vec<f64>>,
    altitude: Option<Vec<f64>>,
}

/// Combined activity with detailed stream data
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActivityWithStreams {
    #[serde(flatten)]
    activity: Activity,
    streams: Option<ActivityStreams>,
}

/// Index file - just metadata, no streams
#[derive(Debug, Serialize, Deserialize)]
struct ActivityIndex {
    last_updated: String,
    activities: Vec<ActivitySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActivitySummary {
    id: i64,
    name: String,
    start_date: String,
    start_date_local: String,
    distance: f64,
    moving_time: i32,
    average_watts: Option<f64>,
    weighted_average_watts: Option<f64>,
    max_watts: Option<f64>,
    average_heartrate: Option<f64>,
    max_heartrate: Option<f64>,
    average_cadence: Option<f64>,
    total_elevation_gain: f64,
    kilojoules: Option<f64>,
    // Computed from streams
    normalized_power: Option<f64>,
    efficiency_factor: Option<f64>,
    decoupling: Option<f64>,
    best_5s: Option<f64>,
    best_30s: Option<f64>,
    best_1min: Option<f64>,
    best_2min: Option<f64>,
    best_5min: Option<f64>,
    best_10min: Option<f64>,
    best_20min: Option<f64>,
    best_60min: Option<f64>,
    zone_seconds: Option<[i32; 6]>,
    hr_recovery: Option<f64>,
}

struct ActivityMetrics {
    normalized_power: Option<f64>,
    efficiency_factor: Option<f64>,
    decoupling: Option<f64>,
    best_5s: Option<f64>,
    best_30s: Option<f64>,
    best_1min: Option<f64>,
    best_2min: Option<f64>,
    best_5min: Option<f64>,
    best_10min: Option<f64>,
    best_20min: Option<f64>,
    best_60min: Option<f64>,
    zone_seconds: Option<[i32; 6]>,
    hr_recovery: Option<f64>,
}

fn trim_trailing_zeros(watts: &[f64]) -> &[f64] {
    let mut end = watts.len();
    while end > 0 && watts[end - 1] == 0.0 {
        end -= 1;
    }
    &watts[..end]
}

fn compute_metrics(streams: &ActivityStreams) -> ActivityMetrics {
    let empty = ActivityMetrics {
        normalized_power: None, efficiency_factor: None, decoupling: None,
        best_5s: None, best_30s: None, best_1min: None, best_2min: None,
        best_5min: None, best_10min: None, best_20min: None, best_60min: None,
        zone_seconds: None, hr_recovery: None,
    };

    let watts_raw = match &streams.watts {
        Some(w) if !w.is_empty() => w,
        _ => return empty,
    };
    let watts = trim_trailing_zeros(watts_raw);
    if watts.is_empty() {
        return empty;
    }

    // --- Normalized Power (30s rolling avg → 4th power) ---
    let normalized_power = if watts.len() >= 30 {
        let rolling: Vec<f64> = (29..watts.len())
            .map(|i| watts[i - 29..=i].iter().sum::<f64>() / 30.0)
            .collect();
        let avg_fourth = rolling.iter().map(|&p| p.powi(4)).sum::<f64>() / rolling.len() as f64;
        Some(avg_fourth.powf(0.25))
    } else {
        None
    };

    // --- Efficiency Factor (NP / avg HR) ---
    let efficiency_factor = normalized_power.and_then(|np| {
        streams.heartrate.as_ref().and_then(|hr_raw| {
            let hr: Vec<f64> = hr_raw.iter().take(watts.len()).map(|&h| h as f64).collect();
            let valid: Vec<f64> = hr.iter().copied().filter(|&h| h > 0.0).collect();
            if valid.is_empty() { return None; }
            let avg_hr = valid.iter().sum::<f64>() / valid.len() as f64;
            if avg_hr > 0.0 { Some(np / avg_hr) } else { None }
        })
    });

    // --- Aerobic Decoupling ---
    let decoupling = if watts.len() >= 600 {
        streams.heartrate.as_ref().and_then(|hr_raw| {
            let hr: Vec<f64> = hr_raw.iter().take(watts.len()).map(|&h| h as f64).collect();
            let mid = watts.len() / 2;
            let first_power = watts[..mid].iter().sum::<f64>() / mid as f64;
            let second_power = watts[mid..].iter().sum::<f64>() / (watts.len() - mid) as f64;
            let first_hr = hr[..mid].iter().sum::<f64>() / mid as f64;
            let second_hr = hr[mid..].iter().sum::<f64>() / (hr.len() - mid) as f64;
            if first_hr == 0.0 || first_power == 0.0 { return None; }
            let first_ef = first_power / first_hr;
            let second_ef = second_power / second_hr;
            Some(((first_ef - second_ef) / first_ef) * 100.0)
        })
    } else {
        None
    };

    // --- Best efforts (sliding window) ---
    let best_average = |window: usize| -> Option<f64> {
        if watts.len() < window { return None; }
        let mut sum: f64 = watts[..window].iter().sum();
        let mut best = sum / window as f64;
        for i in window..watts.len() {
            sum += watts[i] - watts[i - window];
            best = best.max(sum / window as f64);
        }
        Some(best)
    };
    let best_5s   = best_average(5);
    let best_30s  = best_average(30);
    let best_1min = best_average(60);
    let best_2min = best_average(120);
    let best_5min = best_average(300);
    let best_10min = best_average(600);
    let best_20min = best_average(1200);
    let best_60min = best_average(3600);

    // --- Zone distribution (% of FTP = 200W estimate) ---
    let ftp = 200.0_f64;
    let zone_thresholds = [0.55, 0.75, 0.90, 1.05, 1.20];
    let mut zones = [0i32; 6];
    for &w in watts {
        let pct = w / ftp;
        let zone = if pct < zone_thresholds[0] { 0 }
            else if pct < zone_thresholds[1] { 1 }
            else if pct < zone_thresholds[2] { 2 }
            else if pct < zone_thresholds[3] { 3 }
            else if pct < zone_thresholds[4] { 4 }
            else { 5 };
        zones[zone] += 1;
    }

    ActivityMetrics {
        normalized_power,
        efficiency_factor,
        decoupling,
        best_5s,
        best_30s,
        best_1min,
        best_2min,
        best_5min,
        best_10min,
        best_20min,
        best_60min,
        zone_seconds: Some(zones),
        hr_recovery: streams.heartrate.as_ref().and_then(|hr_raw| {
            let hr: Vec<f64> = hr_raw.iter().take(watts.len()).map(|&h| h as f64).collect();
            let n = hr.len();
            if n < 120 { return None; }
            // Peak HR in the last 5 minutes of effort
            let window = std::cmp::min(300, n);
            let peak = hr[n-window..n].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            // Average of last 30 seconds
            let end_avg = hr[n-30..n].iter().sum::<f64>() / 30.0;
            let drop = peak - end_avg;
            if drop > 0.0 { Some(drop) } else { None }
        }),
    }
}

impl ActivityIndex {
    fn load() -> Self {
        fs::read_to_string("data/index.json")
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| ActivityIndex {
                last_updated: String::new(),
                activities: Vec::new(),
            })
    }
    
    fn save(&self) -> Result<(), Box<dyn Error>> {
        fs::create_dir_all("data")?;
        let json = serde_json::to_string_pretty(self)?;
        fs::write("data/index.json", json)?;
        Ok(())
    }
    
    fn get_known_ids(&self) -> HashSet<i64> {
        self.activities.iter().map(|a| a.id).collect()
    }
    
    fn add_activity(&mut self, activity: &Activity, metrics: ActivityMetrics) {
        let summary = ActivitySummary {
            id: activity.id,
            name: activity.name.clone(),
            start_date: activity.start_date.clone(),
            start_date_local: activity.start_date_local.clone(),
            distance: activity.distance,
            moving_time: activity.moving_time,
            average_watts: activity.average_watts,
            weighted_average_watts: activity.weighted_average_watts,
            max_watts: activity.max_watts,
            average_heartrate: activity.average_heartrate,
            max_heartrate: activity.max_heartrate,
            average_cadence: activity.average_cadence,
            total_elevation_gain: activity.total_elevation_gain,
            kilojoules: activity.kilojoules,
            normalized_power: metrics.normalized_power,
            efficiency_factor: metrics.efficiency_factor,
            decoupling: metrics.decoupling,
            best_5s: metrics.best_5s,
            best_30s: metrics.best_30s,
            best_1min: metrics.best_1min,
            best_2min: metrics.best_2min,
            best_5min: metrics.best_5min,
            best_10min: metrics.best_10min,
            best_20min: metrics.best_20min,
            best_60min: metrics.best_60min,
            zone_seconds: metrics.zone_seconds,
            hr_recovery: metrics.hr_recovery,
        };
        // Remove existing entry if present, then re-insert
        self.activities.retain(|a| a.id != activity.id);
        self.activities.push(summary);
        self.activities.sort_by(|a, b| b.start_date.cmp(&a.start_date));
    }
}

fn save_activity_file(activity: &ActivityWithStreams) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all("data/activities")?;
    let filename = format!("data/activities/{}.json", activity.activity.id);
    let json = serde_json::to_string_pretty(activity)?;
    fs::write(&filename, json)?;
    Ok(())
}

fn activity_file_exists(id: i64) -> bool {
    std::path::Path::new(&format!("data/activities/{}.json", id)).exists()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();
    
    println!("🚴 Watts Happening - Strava Data Fetcher\n");
    
    // Load existing index
    let mut index = ActivityIndex::load();
    let known_ids = index.get_known_ids();
    println!("📂 Found {} existing Zwift activities in index", index.activities.len());
    
    // Get credentials from environment
    let client_id = std::env::var("STRAVA_CLIENT_ID")?;
    let client_secret = std::env::var("STRAVA_CLIENT_SECRET")?;
    let refresh_token = std::env::var("STRAVA_REFRESH_TOKEN")?;
    
    // Get fresh access token
    println!("📡 Refreshing access token...");
    let access_token = refresh_access_token(&client_id, &client_secret, &refresh_token).await?;
    
    // Fetch activities with pagination
    println!("📊 Fetching activities from Strava...\n");
    
    let per_page = 50;
    let mut page = 1;
    let mut total_fetched = 0;
    let mut new_zwift_activities: Vec<Activity> = Vec::new();
    let mut found_existing = false;
    
    // Paginate until we find activities we already have
    while !found_existing {
        println!("   Fetching page {} ({} per page)...", page, per_page);
        
        let activities = fetch_activities_page(&access_token, page, per_page).await?;
        
        if activities.is_empty() {
            println!("   No more activities found.");
            break;
        }
        
        total_fetched += activities.len();
        
        for activity in activities {
            // Check if we already have this activity
            if known_ids.contains(&activity.id) {
                println!("   ✓ Found existing activity: {} - stopping pagination", activity.name);
                found_existing = true;
                break;
            }
            
            // Only keep VirtualRide (Zwift) activities
            if activity.sport_type == "VirtualRide" {
                println!("   🆕 New Zwift activity: {}", activity.name);
                new_zwift_activities.push(activity);
            } else {
                println!("   ⏭️  Skipping outdoor activity: {} ({})", activity.name, activity.sport_type);
            }
        }
        
        page += 1;
        
        // Safety limit - don't fetch more than 5 pages (250 activities) in one run
        if page > 5 {
            println!("   ⚠️  Reached page limit, stopping pagination");
            break;
        }
    }
    
    println!("\n📈 Summary:");
    println!("   Total activities fetched from API: {}", total_fetched);
    println!("   New Zwift activities to process: {}", new_zwift_activities.len());
    
    // Fetch detailed streams for new activities
    if !new_zwift_activities.is_empty() {
        println!("\n🔍 Fetching detailed streams for new activities...\n");
        
        for (i, activity) in new_zwift_activities.iter().enumerate() {
            println!("   [{}/{}] {} (id: {})", 
                i + 1, 
                new_zwift_activities.len(), 
                activity.name, 
                activity.id
            );
            
            // Skip if file already exists (safety check)
            if activity_file_exists(activity.id) {
                println!("      ⏭️  File already exists, skipping");
                // Will be picked up in the rebuild pass below
                continue;
            }
            
            match fetch_activity_streams(&access_token, activity.id).await {
                Ok(streams) => {
                    let data_points = streams.time.as_ref().map(|t| t.len()).unwrap_or(0);
                    println!("      ✅ {} data points", data_points);
                    
                    let activity_with_streams = ActivityWithStreams {
                        activity: activity.clone(),
                        streams: Some(streams),
                    };
                    save_activity_file(&activity_with_streams)?;
                }
                Err(e) => {
                    println!("      ⚠️  Could not fetch streams: {}", e);
                    let activity_with_streams = ActivityWithStreams {
                        activity: activity.clone(),
                        streams: None,
                    };
                    save_activity_file(&activity_with_streams)?;
                }
            }
            
            // Rate limiting - be nice to the API
            if i < new_zwift_activities.len() - 1 {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        }
    }

    // Rebuild full index from all activity files (computes metrics for everyone)
    println!("\n🔧 Rebuilding index with computed metrics...");
    index.activities.clear();
    let mut activity_files: Vec<_> = fs::read_dir("data/activities")?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
        .collect();
    activity_files.sort_by_key(|e| e.path());

    for entry in &activity_files {
        let content = match fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let activity_with_streams: ActivityWithStreams = match serde_json::from_str(&content) {
            Ok(a) => a,
            Err(_) => continue,
        };
        let metrics = match &activity_with_streams.streams {
            Some(s) => compute_metrics(s),
            None => ActivityMetrics {
                normalized_power: None, efficiency_factor: None, decoupling: None,
                best_5s: None, best_30s: None, best_1min: None, best_2min: None,
                best_5min: None, best_10min: None, best_20min: None, best_60min: None,
                zone_seconds: None, hr_recovery: None,
            },
        };
        index.add_activity(&activity_with_streams.activity, metrics);
    }
    println!("   ✅ Indexed {} activities", index.activities.len());

    // Update timestamp and save index
    index.last_updated = chrono::Utc::now().to_rfc3339();
    index.save()?;
    
    println!("\n💾 Saved {} total Zwift activities", index.activities.len());
    println!("   📁 Individual files in data/activities/");
    println!("   📋 Index at data/index.json");
    println!("🕐 Last updated: {}", index.last_updated);
    
    Ok(())
}

async fn refresh_access_token(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<String, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://www.strava.com/oauth/token")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await?
        .json::<TokenResponse>()
        .await?;
    
    Ok(response.access_token)
}

async fn fetch_activities_page(access_token: &str, page: u32, per_page: u32) -> Result<Vec<Activity>, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://www.strava.com/api/v3/athlete/activities")
        .header("Authorization", format!("Bearer {}", access_token))
        .query(&[
            ("page", page.to_string()),
            ("per_page", per_page.to_string()),
        ])
        .send()
        .await?;
    
    let status = response.status();
    let text = response.text().await?;
    
    if !status.is_success() {
        eprintln!("❌ Strava API error ({}): {}", status, text);
        return Err(format!("API returned status {}", status).into());
    }
    
    let activities: Vec<Activity> = serde_json::from_str(&text)?;
    Ok(activities)
}

async fn fetch_activity_streams(access_token: &str, activity_id: i64) -> Result<ActivityStreams, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://www.strava.com/api/v3/activities/{}/streams",
        activity_id
    );
    
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .query(&[
            ("keys", "time,watts,heartrate,cadence,velocity_smooth,altitude"),
            ("key_by_type", "true"),
        ])
        .send()
        .await?;
    
    let status = response.status();
    let text = response.text().await?;
    
    if !status.is_success() {
        eprintln!("❌ Streams API error ({}): {}", status, text);
        return Err(format!("API returned status {}", status).into());
    }
    
    // Parse the keyed response
    let streams_map: serde_json::Value = serde_json::from_str(&text)?;
    
    let streams = ActivityStreams {
        time: streams_map.get("time")
            .and_then(|v| v.get("data"))
            .and_then(|v| serde_json::from_value(v.clone()).ok()),
        watts: streams_map.get("watts")
            .and_then(|v| v.get("data"))
            .and_then(|v| serde_json::from_value(v.clone()).ok()),
        heartrate: streams_map.get("heartrate")
            .and_then(|v| v.get("data"))
            .and_then(|v| serde_json::from_value(v.clone()).ok()),
        cadence: streams_map.get("cadence")
            .and_then(|v| v.get("data"))
            .and_then(|v| serde_json::from_value(v.clone()).ok()),
        velocity_smooth: streams_map.get("velocity_smooth")
            .and_then(|v| v.get("data"))
            .and_then(|v| serde_json::from_value(v.clone()).ok()),
        altitude: streams_map.get("altitude")
            .and_then(|v| v.get("data"))
            .and_then(|v| serde_json::from_value(v.clone()).ok()),
    };
    
    Ok(streams)
}

