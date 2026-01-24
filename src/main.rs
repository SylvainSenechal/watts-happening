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
    distance: f64,
    moving_time: i32,
    average_watts: Option<f64>,
    average_heartrate: Option<f64>,
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
    
    fn add_activity(&mut self, activity: &Activity) {
        let summary = ActivitySummary {
            id: activity.id,
            name: activity.name.clone(),
            start_date: activity.start_date.clone(),
            distance: activity.distance,
            moving_time: activity.moving_time,
            average_watts: activity.average_watts,
            average_heartrate: activity.average_heartrate,
        };
        self.activities.insert(0, summary);
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
                index.add_activity(activity);
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
                    
                    // Save individual file
                    save_activity_file(&activity_with_streams)?;
                    
                    // Add to index
                    index.add_activity(activity);
                }
                Err(e) => {
                    println!("      ⚠️  Could not fetch streams: {}", e);
                    // Still save the activity without streams
                    let activity_with_streams = ActivityWithStreams {
                        activity: activity.clone(),
                        streams: None,
                    };
                    save_activity_file(&activity_with_streams)?;
                    index.add_activity(activity);
                }
            }
            
            // Rate limiting - be nice to the API
            if i < new_zwift_activities.len() - 1 {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        }
    }
    
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

