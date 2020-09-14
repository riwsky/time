use chrono::offset::Utc;
use clap::{App, Arg, SubCommand};
use error_chain::error_chain;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::env;

error_chain! {}

#[derive(Deserialize, Debug)]
struct Token {
    token: String,
}

#[derive(Deserialize, Debug)]
struct Activity {
    id: String,
    name: String,
}

#[derive(Deserialize, Debug)]
struct Activities {
    activities: Vec<Activity>,
}

#[tokio::main]
pub async fn main() -> Result<()> {
    let matches = App::new("Timeular CLI")
        .about("Command line client for interacting with the Timeular API. 
        
Takes creds in the TIMEULAR_KEY and TIMEULAR_SECRET env vars.")
        .subcommand(SubCommand::with_name("list").help("list available activities"))
        .subcommand(
            SubCommand::with_name("start")
                .help("start activity by (regex of) name")
                .arg(Arg::with_name("name").required(true))
                .arg(Arg::with_name("note")),
        )
        .subcommand(SubCommand::with_name("stop").help("stop the current tracking"))
        .get_matches();
    let key = env::var("TIMEULAR_KEY").chain_err(|| "grabbing key")?;
    let secret = env::var("TIMEULAR_SECRET").chain_err(|| "grabbing secret")?;

    let sign_in_body = json!({
        "apiKey": key,
        "apiSecret": secret,
    });

    let base_url = "https://api.timeular.com/api/v3";
    let sign_in_url = format!("{}/developer/sign-in", base_url);
    let client = Client::new();
    let tok_response = client
        .post(&sign_in_url)
        .json(&sign_in_body)
        .send()
        .await
        .chain_err(|| "requesting token")?;
    let token: Token = tok_response.json().await.chain_err(|| "decode token")?;
    let activities_url = format!("{}/activities", base_url);
    let act_response = client
        .get(&activities_url)
        .bearer_auth(&token.token)
        .send()
        .await
        .chain_err(|| "requesting activities")?;
    let activities: Activities = act_response
        .json()
        .await
        .chain_err(|| "decode activities")?;
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%S.%3f").to_string();

    match matches.subcommand() {
        ("list", _) => {
            for activity in activities.activities {
                println!("{}", activity.name);
            }
        }
        ("start", Some(matches)) => {
            let name = matches
                .value_of("name")
                .chain_err(|| "no name passed to start")?;
            let pattern = format!(r"(?i){}", name);
            let re = Regex::new(&pattern).chain_err(|| "invalid regex for name")?;
            let note = matches.value_of("note").unwrap_or("");
            let activity = activities
                .activities
                .into_iter()
                .find(|activity| re.is_match(&activity.name))
                .chain_err(|| "no such activity found")?;

            let start_tracking_body = json!({
                "startedAt": now,
                "note": {"text": note},
            });

            let start_tracking_url = format!("{}/tracking/{}/start", base_url, activity.id);
            client
                .post(&start_tracking_url)
                .json(&start_tracking_body)
                .bearer_auth(&token.token)
                .send()
                .await
                .chain_err(|| "start tracking")?;
        }
        ("stop", _) => {
            let stop_tracking_url = format!("{}/tracking/stop", base_url);
            let stop_tracking_body = json!({
                "stoppedAt": now,
            });
            client
                .post(&stop_tracking_url)
                .json(&stop_tracking_body)
                .bearer_auth(&token.token)
                .send()
                .await
                .chain_err(|| "stop tracking")?;
        }
        _ => {unreachable!()}
    }

    Ok(())
}
