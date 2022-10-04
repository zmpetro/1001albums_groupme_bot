use chrono::{Datelike, Local};
use dotenv::dotenv;
use reqwest::blocking::Client;
use serde::Serialize;
use std::env;
use std::{thread, time::Duration};

const GENERATOR_URL: &str = "https://1001albumsgenerator.com";
const GROUPME_API_URL: &str = "https://api.groupme.com/v3/bots/post";
const SPOTIFY_URL: &str = "https://open.spotify.com/album";

#[derive(Debug)]
struct Album {
    album: String,
    artist: String,
    release_year: String,
    spotify_link: String,
}

#[derive(Serialize)]
struct Message {
    bot_id: String,
    text: String,
}

fn get_album(
    client: &Client,
    generator_api_url: &str,
    retry_limit: u8,
    sleep_secs: u64,
) -> Result<Album, reqwest::Error> {
    let mut retry: u8 = 0;
    loop {
        let resp = client.get(generator_api_url).send()?.error_for_status();
        match resp {
            Ok(r) => {
                let json = r.json::<serde_json::Value>()?;
                let album = Album {
                    album: json["currentAlbum"]["name"].as_str().unwrap().to_string(),
                    artist: json["currentAlbum"]["artist"].as_str().unwrap().to_string(),
                    release_year: json["currentAlbum"]["releaseDate"]
                        .as_str()
                        .unwrap()
                        .to_string(),
                    spotify_link: format!(
                        "{}/{}",
                        SPOTIFY_URL,
                        json["currentAlbum"]["spotifyId"].as_str().unwrap()
                    ),
                };
                return Ok(album);
            }
            Err(e) => {
                if retry < retry_limit {
                    retry += 1;
                    println!("Could not get album: {}", e);
                    println!(
                        "Waiting {} seconds and retrying... (Retry {}/{})",
                        sleep_secs, retry, retry_limit,
                    );
                    thread::sleep(Duration::from_secs(sleep_secs));
                } else {
                    return Err(e);
                }
            }
        }
    }
}

fn get_message(album: &Album, generator_group_url: &str) -> String {
    let dt = Local::now();

    let message = format!(
        "1001albumsgenerator {}/{}/{}\n\n\
        {} by {} ({})\n\n\
        {}\n\n\
        Group: {}\n",
        dt.month(),
        dt.day(),
        dt.year(),
        album.album,
        album.artist,
        album.release_year,
        album.spotify_link,
        generator_group_url,
    );

    return message;
}

fn send_message(
    client: &Client,
    bot_id: &str,
    message: &str,
    retry_limit: u8,
    sleep_secs: u64,
) -> Result<(), reqwest::Error> {
    let json = Message {
        bot_id: bot_id.to_string(),
        text: message.to_string(),
    };

    let mut retry: u8 = 0;
    loop {
        let resp = client
            .post(GROUPME_API_URL)
            .json(&json)
            .send()?
            .error_for_status();
        match resp {
            Ok(_) => {
                println!("Message sent:\n{}", message);
                return Ok(());
            }
            Err(e) => {
                if retry < retry_limit {
                    retry += 1;
                    println!("Could not send message: {}", e);
                    println!(
                        "Waiting {} seconds and retrying... (Retry {}/{})",
                        sleep_secs, retry, retry_limit,
                    );
                    thread::sleep(Duration::from_secs(sleep_secs));
                } else {
                    return Err(e);
                }
            }
        }
    }
}

fn main() {
    dotenv().ok();

    // Create a .env file with the following 2 variables: BOT_ID, GROUP
    // GroupMe group bot ID
    let bot_id = env::var("BOT_ID").expect("BOT_ID is not set");
    // https://1001albumsgenerator.com/ group name
    let group = env::var("GROUP").expect("GROUP is not set");

    let client = Client::new();

    let generator_api_url = format!("{}/api/v1/groups/{}", GENERATOR_URL, group);

    let retry_limit: u8 = 10;
    let sleep_secs: u64 = 60;

    let album = get_album(&client, &generator_api_url, retry_limit, sleep_secs)
        .expect("Could not get album");

    let generator_group_url = format!("{}/groups/{}", GENERATOR_URL, group);
    let message = get_message(&album, &generator_group_url);

    send_message(&client, &bot_id, &message, retry_limit, sleep_secs)
        .expect("Could not send message");
}
