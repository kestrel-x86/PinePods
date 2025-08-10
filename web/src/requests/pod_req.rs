use anyhow::{Context, Error};
// use futures_util::stream::StreamExt;
use crate::components::context::AppState;
use crate::components::notification_center::TaskProgress;
use futures::StreamExt;
use gloo::net::websocket::WebSocketError;
use gloo::net::websocket::{futures::WebSocket, Message};
use gloo_net::http::Request;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt;
use web_sys::console;
use yewdux::Dispatch;

fn bool_from_int<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    struct BoolOrIntVisitor;

    impl<'de> serde::de::Visitor<'de> for BoolOrIntVisitor {
        type Value = bool;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a boolean or an integer")
        }

        fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
            Ok(value)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(value != 0)
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(value != 0)
        }
    }

    deserializer.deserialize_any(BoolOrIntVisitor)
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
#[allow(non_snake_case)]
#[serde(rename_all = "lowercase")]
pub struct Episode {
    pub podcastname: String,
    pub episodetitle: String,
    pub episodepubdate: String,
    pub episodedescription: String,
    pub episodeartwork: String,
    pub episodeurl: String,
    pub episodeduration: i32,
    pub listenduration: Option<i32>,
    pub episodeid: i32,
    pub completed: bool,
    pub saved: bool,
    pub queued: bool,
    pub downloaded: bool,
    pub is_youtube: bool,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct RecentEps {
    pub episodes: Option<Vec<Episode>>,
}

pub async fn call_get_recent_eps(
    server_name: &String,
    api_key: &Option<String>,
    user_id: &i32,
) -> Result<Vec<Episode>, anyhow::Error> {
    let url = format!("{}/api/data/return_episodes/{}", server_name, user_id);

    // Convert Option<String> to Option<&str>
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let response = Request::get(&url)
        .header("Api-Key", api_key_ref)
        .send()
        .await?;
    if !response.ok() {
        return Err(anyhow::Error::msg(format!(
            "Failed to fetch episodes: {}",
            response.status_text()
        )));
    }

    // First, capture the response text for diagnostic purposes
    let response_text = response
        .text()
        .await
        .unwrap_or_else(|_| "Failed to get response text".to_string());
    // Try to deserialize the response text
    match serde_json::from_str::<RecentEps>(&response_text) {
        Ok(response_body) => Ok(response_body.episodes.unwrap_or_else(Vec::new)),
        Err(_e) => Err(anyhow::Error::msg("Failed to deserialize response")),
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PodcastValues {
    pub pod_title: String,
    pub pod_artwork: String,
    pub pod_author: String,
    pub categories: HashMap<String, String>,
    pub pod_description: String,
    pub pod_episode_count: i32,
    pub pod_feed_url: String,
    pub pod_website: String,
    pub pod_explicit: bool,
    pub user_id: i32,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct FirstEpisodeInfo {
    pub episode_id: i32,
    pub podcast_id: i32,
    pub title: String,
    pub description: String,
    pub audio_url: String,
    pub artwork_url: String,
    pub release_datetime: String,
    pub duration: i32,
    pub completed: bool,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct PodcastStatusResponse {
    pub success: bool,
    pub podcast_id: i32,
    pub first_episode_id: i32,
}

pub async fn call_add_podcast(
    server_name: &str,
    api_key: &Option<String>,
    _user_id: i32,
    added_podcast: &PodcastValues,
    podcast_index_id: Option<i64>,
) -> Result<PodcastStatusResponse, Error> {
    let url = format!("{}/api/data/add_podcast", server_name);
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| Error::msg("API key is missing"))?;

    // Create a new struct that includes PodcastValues and the optional podcast_index_id
    #[derive(Serialize)]
    struct AddPodcastRequest {
        podcast_values: PodcastValues,
        podcast_index_id: Option<i64>,
    }

    let request_body = AddPodcastRequest {
        podcast_values: added_podcast.clone(),
        podcast_index_id,
    };

    // Serialize the new struct into JSON
    let json_body = serde_json::to_string(&request_body)?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(json_body)?
        .send()
        .await?;

    if response.ok() {
        let response_body = response.json::<PodcastStatusResponse>().await?;
        Ok(response_body)
    } else {
        Err(Error::msg(format!(
            "Error adding podcast: {}",
            response.status_text()
        )))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RemovePodcastValues {
    pub podcast_id: i32,
    pub user_id: i32,
    pub is_youtube: bool,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct RemovePodcastResponse {
    pub success: bool,
}

pub async fn call_remove_podcasts(
    server_name: &String,
    api_key: &Option<String>,
    remove_podcast: &RemovePodcastValues,
) -> Result<bool, Error> {
    let url = format!("{}/api/data/remove_podcast_id", server_name);
    // Convert Option<String> to Option<&str>
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;
    // Serialize `added_podcast` into JSON
    let json_body = serde_json::to_string(remove_podcast)?;
    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(json_body)?
        .send()
        .await?;
    let response_text = response
        .text()
        .await
        .unwrap_or_else(|_| "Failed to get response text".to_string());
    if response.ok() {
        match serde_json::from_str::<RemovePodcastResponse>(&response_text) {
            Ok(parsed_response) => Ok(parsed_response.success),
            Err(_parse_error) => {
                // Add debug logging to see what's being received
                web_sys::console::log_1(&format!("Response text: {}", response_text).into());
                Err(anyhow::Error::msg("Failed to parse response"))
            }
        }
    } else {
        Err(anyhow::Error::msg(format!(
            "Error removing podcast: {}",
            response.status_text()
        )))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RemovePodcastValuesName {
    pub user_id: i32,
    pub podcast_name: String,
    pub podcast_url: String,
}

pub async fn call_remove_podcasts_name(
    server_name: &String,
    api_key: &Option<String>,
    remove_podcast: &RemovePodcastValuesName,
) -> Result<bool, Error> {
    let url = format!("{}/api/data/remove_podcast", server_name);
    // Convert Option<String> to Option<&str>
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;
    // Serialize `added_podcast` into JSON
    let json_body = serde_json::to_string(remove_podcast)?;
    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(json_body)?
        .send()
        .await?;
    let response_text = response
        .text()
        .await
        .unwrap_or_else(|_| "Failed to get response text".to_string());
    if response.ok() {
        // Use the new simpler response struct
        match serde_json::from_str::<RemovePodcastResponse>(&response_text) {
            Ok(parsed_response) => Ok(parsed_response.success),
            Err(_parse_error) => {
                // Add debug logging
                web_sys::console::log_1(&format!("Response text: {}", response_text).into());
                Err(anyhow::Error::msg("Failed to parse response"))
            }
        }
    } else {
        Err(anyhow::Error::msg(format!(
            "Error removing podcast: {}",
            response.status_text()
        )))
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct PodcastResponse {
    pub pods: Option<Vec<Podcast>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[allow(non_snake_case)]
#[serde(rename_all = "lowercase")]
pub struct Podcast {
    pub podcastid: i32,
    pub podcastname: String,
    pub artworkurl: Option<String>,
    pub description: Option<String>,
    pub episodecount: Option<i32>,
    pub websiteurl: Option<String>,
    pub feedurl: String,
    pub author: Option<String>,
    pub categories: Option<HashMap<String, String>>,
    #[serde(deserialize_with = "bool_from_int")]
    pub explicit: bool,
    #[serde(default)] // Add this line
    pub podcastindexid: Option<i64>,
}

pub async fn call_get_podcasts(
    server_name: &String,
    api_key: &Option<String>,
    user_id: &i32,
) -> Result<Vec<Podcast>, anyhow::Error> {
    let url = format!("{}/api/data/return_pods/{}", server_name, user_id);

    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let response = Request::get(&url)
        .header("Api-Key", api_key_ref)
        .send()
        .await?;

    if !response.ok() {
        return Err(anyhow::Error::msg(format!(
            "Failed to fetch podcasts: {}",
            response.status_text()
        )));
    }

    let response_text = response
        .text()
        .await
        .unwrap_or_else(|_| "Failed to get response text".to_string());

    match serde_json::from_str::<PodcastResponse>(&response_text) {
        Ok(response_body) => Ok(response_body.pods.unwrap_or_else(Vec::new)),
        Err(e) => {
            web_sys::console::log_1(
                &format!("Unable to parse Podcasts: {}", &response_text).into(),
            );
            Err(anyhow::Error::msg(format!(
                "Failed to deserialize response: {}",
                e
            )))
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct PodcastResponseExtra {
    pub pods: Option<Vec<PodcastExtra>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[allow(non_snake_case)]
#[serde(rename_all = "lowercase")]
pub struct PodcastExtra {
    pub podcastid: i32,
    pub podcastname: String,
    pub artworkurl: Option<String>,
    pub description: Option<String>,
    pub episodecount: Option<i32>,
    pub websiteurl: Option<String>,
    pub feedurl: String,
    pub author: Option<String>,
    pub categories: Option<HashMap<String, String>>,
    #[serde(deserialize_with = "bool_from_int")]
    pub explicit: bool,
    pub podcastindexid: i64,
    #[serde(default)]
    pub play_count: i64,
    #[serde(default)]
    pub episodes_played: i32,
    #[serde(default)]
    pub oldest_episode_date: Option<String>,
    #[serde(default)]
    pub is_youtube: bool,
}

impl From<Podcast> for PodcastExtra {
    fn from(podcast: Podcast) -> Self {
        let is_youtube = podcast.feedurl.contains("youtube.com");

        PodcastExtra {
            podcastid: podcast.podcastid,
            podcastname: podcast.podcastname,
            artworkurl: podcast.artworkurl,
            description: podcast.description,
            episodecount: podcast.episodecount,
            websiteurl: podcast.websiteurl,
            feedurl: podcast.feedurl.clone(),
            author: podcast.author,
            categories: podcast.categories,
            explicit: podcast.explicit,
            podcastindexid: podcast.podcastindexid.unwrap_or(0),
            play_count: 0,
            episodes_played: 0,
            oldest_episode_date: None,
            is_youtube,
        }
    }
}

impl From<PodcastExtra> for Podcast {
    fn from(podcast_extra: PodcastExtra) -> Self {
        Podcast {
            podcastid: podcast_extra.podcastid,
            podcastname: podcast_extra.podcastname,
            artworkurl: podcast_extra.artworkurl,
            description: podcast_extra.description,
            episodecount: podcast_extra.episodecount,
            websiteurl: podcast_extra.websiteurl,
            feedurl: podcast_extra.feedurl,
            author: podcast_extra.author,
            categories: podcast_extra.categories,
            explicit: podcast_extra.explicit,
            podcastindexid: Some(podcast_extra.podcastindexid),
        }
    }
}

pub async fn call_get_podcasts_extra(
    server_name: &String,
    api_key: &Option<String>,
    user_id: &i32,
) -> Result<Vec<PodcastExtra>, anyhow::Error> {
    let url = format!("{}/api/data/return_pods/{}", server_name, user_id);
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;
    let response = Request::get(&url)
        .header("Api-Key", api_key_ref)
        .send()
        .await?;
    if !response.ok() {
        return Err(anyhow::Error::msg(format!(
            "Failed to fetch podcasts: {}",
            response.status_text()
        )));
    }
    let response_text = response
        .text()
        .await
        .unwrap_or_else(|_| "Failed to get response text".to_string());
    match serde_json::from_str::<PodcastResponseExtra>(&response_text) {
        Ok(response_body) => {
            let mut pods = response_body.pods.unwrap_or_else(Vec::new);
            // Update is_youtube flag based on feedurl
            for pod in &mut pods {
                pod.is_youtube = pod.feedurl.contains("youtube.com");
            }
            Ok(pods)
        }
        Err(e) => {
            web_sys::console::log_1(
                &format!("Unable to parse Podcasts: {}", &response_text).into(),
            );
            Err(anyhow::Error::msg(format!(
                "Failed to deserialize response: {}",
                e
            )))
        }
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct TimeInfoResponse {
    pub timezone: String,
    pub hour_pref: i32,
}
#[allow(dead_code)]
pub async fn call_get_time_info(
    server: &str,
    key: String,
    user_id: i32,
) -> Result<TimeInfoResponse, anyhow::Error> {
    let endpoint = format!("{}/api/data/get_time_info?user_id={}", server, user_id);

    let resp = Request::get(&endpoint)
        .header("Api-Key", key.as_str())
        .send()
        .await
        .context("Network Request Error")?;

    if resp.ok() {
        resp.json::<TimeInfoResponse>()
            .await
            .context("Response Parsing Error")
    } else {
        Err(anyhow::anyhow!(
            "Error fetching time info. Server Response: {}",
            resp.status_text()
        ))
    }
}

#[derive(Default, Deserialize, Debug)]
pub struct CheckPodcastResponse {
    pub exists: bool,
}

pub async fn call_check_podcast(
    server: &str,
    api_key: &str,
    user_id: i32,
    podcast_name: &str,
    podcast_url: &str,
) -> Result<CheckPodcastResponse, Error> {
    let encoded_name = utf8_percent_encode(podcast_name, NON_ALPHANUMERIC).to_string();
    let encoded_url = utf8_percent_encode(podcast_url, NON_ALPHANUMERIC).to_string();
    let endpoint = format!(
        "{}/api/data/check_podcast?user_id={}&podcast_name={}&podcast_url={}",
        server, user_id, encoded_name, encoded_url
    );

    let resp = Request::get(&endpoint)
        .header("Api-Key", api_key)
        .send()
        .await
        .context("Network Request Error")?;

    if resp.ok() {
        resp.json::<CheckPodcastResponse>()
            .await
            .context("Response Parsing Error")
    } else {
        Err(anyhow::anyhow!(
            "Error checking podcast. Server Response: {}",
            resp.status_text()
        ))
    }
}

#[derive(Deserialize, Debug)]
pub struct EpisodeInDbResponse {
    pub episode_in_db: bool,
}
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use super::setting_reqs::NotificationResponse;

pub async fn call_check_episode_in_db(
    server: &str,
    api_key: &str,
    user_id: i32,
    episode_title: &str,
    episode_url: &str,
) -> Result<bool, anyhow::Error> {
    let encoded_title = utf8_percent_encode(episode_title, NON_ALPHANUMERIC).to_string();
    let encoded_url = utf8_percent_encode(episode_url, NON_ALPHANUMERIC).to_string();
    let endpoint = format!(
        "{}/api/data/check_episode_in_db/{}?episode_title={}&episode_url={}",
        server, user_id, encoded_title, encoded_url
    );

    let resp = Request::get(&endpoint)
        .header("Api-Key", api_key)
        .send()
        .await
        .context("Network Request Error")?;

    if resp.ok() {
        let episode_in_db_response = resp
            .json::<EpisodeInDbResponse>()
            .await
            .context("Response Parsing Error")?;
        Ok(episode_in_db_response.episode_in_db)
    } else {
        Err(anyhow::anyhow!(
            "Error checking episode in db. Server Response: {}",
            resp.status_text()
        ))
    }
}

// Queue calls

#[derive(Serialize, Deserialize, Debug)]
pub struct QueuePodcastRequest {
    pub episode_id: i32,
    pub user_id: i32,
    pub is_youtube: bool,
}

// Define a struct to match the response JSON structure
#[derive(Deserialize, Debug)]
struct QueueResponse {
    data: String,
}

pub async fn call_queue_episode(
    server_name: &String,
    api_key: &Option<String>,
    request_data: &QueuePodcastRequest,
) -> Result<String, Error> {
    let url = format!("{}/api/data/queue_pod", server_name);
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let request_body = serde_json::to_string(request_data)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;

    if response.ok() {
        let response_body: QueueResponse =
            response.json().await.map_err(|e| anyhow::Error::new(e))?;
        Ok(response_body.data)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to queue {}: {} - {}",
            if request_data.is_youtube {
                "video"
            } else {
                "episode"
            },
            response.status_text(),
            error_text
        )))
    }
}

pub async fn call_remove_queued_episode(
    server_name: &String,
    api_key: &Option<String>,
    request_data: &QueuePodcastRequest,
) -> Result<String, Error> {
    let url = format!("{}/api/data/remove_queued_pod", server_name);
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;
    let request_body = serde_json::to_string(request_data)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;
    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;
    if response.ok() {
        // Use the same QueueResponse struct to deserialize the response
        let response_body: QueueResponse =
            response.json().await.map_err(|e| anyhow::Error::new(e))?;
        Ok(response_body.data)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to remove queued {}: {} - {}",
            if request_data.is_youtube {
                "video"
            } else {
                "episode"
            },
            response.status_text(),
            error_text
        )))
    }
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct QueuedEpisodesResponse {
    pub episodes: Vec<QueuedEpisode>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[allow(non_snake_case)]
#[serde(rename_all = "lowercase")]
pub struct QueuedEpisode {
    pub episodetitle: String,
    pub podcastname: String,
    pub episodepubdate: String,
    pub episodedescription: String,
    pub episodeartwork: String,
    pub episodeurl: String,
    #[serde(default)]
    pub queueposition: Option<i32>,
    pub episodeduration: i32,
    pub queuedate: String,
    pub listenduration: Option<i32>,
    pub episodeid: i32,
    pub completed: bool,
    pub saved: bool,      // Added field
    pub queued: bool,     // Added field
    pub downloaded: bool, // Added field
    pub is_youtube: bool,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct DataResponse {
    pub data: Vec<QueuedEpisode>,
}

pub async fn call_get_queued_episodes(
    server_name: &str,
    api_key: &Option<String>,
    user_id: &i32,
) -> Result<Vec<QueuedEpisode>, anyhow::Error> {
    // Append the user_id as a query parameter
    let url = format!(
        "{}/api/data/get_queued_episodes?user_id={}",
        server_name, user_id
    );

    // Convert Option<String> to Option<&str>
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let response = Request::get(&url)
        .header("Api-Key", api_key_ref)
        .send()
        .await?;

    if !response.ok() {
        return Err(anyhow::Error::msg(format!(
            "Failed to fetch queued episodes: {}",
            response.status_text()
        )));
    }
    let response_text = response.text().await?;

    let response_data: DataResponse = serde_json::from_str(&response_text)?;
    Ok(response_data.data)
}

#[derive(Serialize)]
struct ReorderPayload {
    episode_ids: Vec<i32>,
}

pub async fn call_reorder_queue(
    server_name: &str,
    api_key: &Option<String>,
    user_id: &i32,
    episode_ids: &Vec<i32>,
) -> Result<(), Error> {
    // Build the URL
    let url = format!("{}/api/data/reorder_queue?user_id={}", server_name, user_id);

    // Convert Option<String> to Option<&str>
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    // Create the payload
    let payload = ReorderPayload {
        episode_ids: episode_ids.clone(),
    };

    // Send the request
    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .json(&payload)?
        .send()
        .await?;

    if !response.ok() {
        return Err(anyhow::Error::msg(format!(
            "Failed to reorder queue: {}",
            response.status_text()
        )));
    }

    Ok(())
}

// Save episode calls

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct SavedEpisodesResponse {
    pub episodes: Vec<SavedEpisode>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[allow(non_snake_case)]
#[serde(rename_all = "lowercase")]
pub struct SavedEpisode {
    pub episodetitle: String,
    pub podcastname: String,
    pub episodepubdate: String,
    pub episodedescription: String,
    pub episodeartwork: String,
    pub episodeurl: String,
    pub episodeduration: i32,
    pub listenduration: Option<i32>,
    pub episodeid: i32,
    pub websiteurl: String,
    pub completed: bool,
    pub saved: bool,      // Added field
    pub queued: bool,     // Added field
    pub downloaded: bool, // Added field
    pub is_youtube: bool,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct SavedDataResponse {
    pub saved_episodes: Vec<SavedEpisode>,
}

pub async fn call_get_saved_episodes(
    server_name: &str,
    api_key: &Option<String>,
    user_id: &i32,
) -> Result<Vec<SavedEpisode>, anyhow::Error> {
    // Append the user_id as a query parameter
    let url = format!("{}/api/data/saved_episode_list/{}", server_name, user_id);

    // Convert Option<String> to Option<&str>
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let response = Request::get(&url)
        .header("Api-Key", api_key_ref)
        .send()
        .await?;

    if !response.ok() {
        return Err(anyhow::Error::msg(format!(
            "Failed to fetch saved episodes: {}",
            response.status_text()
        )));
    }

    let response_text = response.text().await?;
    // let response_text = response.text().await?;

    let response_data: SavedDataResponse = serde_json::from_str(&response_text)?;
    Ok(response_data.saved_episodes)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SavePodcastRequest {
    pub episode_id: i32,
    pub user_id: i32,
    pub is_youtube: bool,
}

#[derive(Deserialize, Debug)]
struct SaveResponse {
    detail: String,
}

pub async fn call_save_episode(
    server_name: &String,
    api_key: &Option<String>,
    request_data: &SavePodcastRequest,
) -> Result<String, Error> {
    let url = format!("{}/api/data/save_episode", server_name);
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let request_body = serde_json::to_string(request_data)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;

    if response.ok() {
        let response_body: SaveResponse =
            response.json().await.map_err(|e| anyhow::Error::new(e))?;
        Ok(response_body.detail)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to save {}: {} - {}",
            if request_data.is_youtube {
                "video"
            } else {
                "episode"
            },
            response.status_text(),
            error_text
        )))
    }
}

pub async fn call_remove_saved_episode(
    server_name: &String,
    api_key: &Option<String>,
    request_data: &SavePodcastRequest,
) -> Result<String, Error> {
    let url = format!("{}/api/data/remove_saved_episode", server_name);
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let request_body = serde_json::to_string(request_data)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;

    if response.ok() {
        let success_message = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Item removed successfully"));
        Ok(success_message)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to remove saved {}: {} - {}",
            if request_data.is_youtube {
                "video"
            } else {
                "episode"
            },
            response.status_text(),
            error_text
        )))
    }
}

// History calls

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct HistoryEpisodesResponse {
    pub episodes: Vec<HistoryEpisode>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[allow(non_snake_case)]
#[serde(rename_all = "lowercase")]
pub struct HistoryEpisode {
    pub episodetitle: String,
    pub podcastname: String,
    pub episodepubdate: String,
    pub episodedescription: String,
    pub episodeartwork: String,
    pub episodeurl: String,
    pub episodeduration: i32,
    pub listenduration: Option<i32>,
    pub episodeid: i32,
    pub completed: bool,
    pub is_youtube: bool,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct HistoryDataResponse {
    pub data: Vec<HistoryEpisode>,
}

pub async fn call_get_user_history(
    server_name: &str,
    api_key: &Option<String>,
    user_id: &i32,
) -> Result<Vec<HistoryEpisode>, anyhow::Error> {
    // Append the user_id as a query parameter
    let url = format!("{}/api/data/user_history/{}", server_name, user_id);

    // Convert Option<String> to Option<&str>
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let response = Request::get(&url)
        .header("Api-Key", api_key_ref)
        .send()
        .await?;

    if !response.ok() {
        return Err(anyhow::Error::msg(format!(
            "Failed to fetch history: {}",
            response.status_text()
        )));
    }

    let response_text = response.text().await?;

    let response_data: HistoryDataResponse = serde_json::from_str(&response_text)?;
    Ok(response_data.data)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HistoryAddRequest {
    pub episode_id: i32,
    pub episode_pos: f32,
    pub user_id: i32,
    pub is_youtube: bool, // Add this field
}

pub async fn call_add_history(
    server_name: &String,
    api_key: String,
    request_data: &HistoryAddRequest,
) -> Result<(), Error> {
    let url = format!("{}/api/data/record_podcast_history", server_name);
    let api_key_ref = api_key.as_str();

    let request_body = serde_json::to_string(request_data)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;

    if !response.ok() {
        return Err(anyhow::Error::msg(format!(
            "Failed to record history: {}",
            response.status_text()
        )));
    }
    Ok(())
}
// Download calls

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct EpisodeDownloadResponse {
    pub episodes: Vec<EpisodeDownload>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[allow(non_snake_case)]
#[serde(rename_all = "lowercase")]
pub struct EpisodeDownload {
    pub episodetitle: String,
    pub podcastname: String,
    pub episodepubdate: String,
    pub episodedescription: String,
    pub episodeartwork: String,
    pub episodeurl: String,
    pub episodeduration: i32,
    pub listenduration: Option<i32>,
    pub episodeid: i32,
    pub downloadedlocation: Option<String>,
    pub podcastid: i32,
    pub podcastindexid: Option<i64>,
    pub completed: bool,
    pub queued: bool,     // Remove #[serde(rename = "is_queued")]
    pub saved: bool,      // Remove #[serde(rename = "is_saved")]
    pub downloaded: bool, // Remove #[serde(rename = "is_downloaded")]
    pub is_youtube: bool,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct DownloadDataResponse {
    #[serde(rename = "downloaded_episodes")]
    pub episodes: Vec<EpisodeDownload>,
}

pub async fn call_get_episode_downloads(
    server_name: &str,
    api_key: &Option<String>,
    user_id: &i32,
) -> Result<Vec<EpisodeDownload>, anyhow::Error> {
    // Append the user_id as a query parameter
    let url = format!(
        "{}/api/data/download_episode_list?user_id={}",
        server_name, user_id
    );

    // Convert Option<String> to Option<&str>
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let response = Request::get(&url)
        .header("Api-Key", api_key_ref)
        .send()
        .await?;

    if !response.ok() {
        return Err(anyhow::Error::msg(format!(
            "Failed to episode downloads: {}",
            response.status_text()
        )));
    }

    let response_text = response.text().await?;

    let response_data: DownloadDataResponse = serde_json::from_str(&response_text)?;
    Ok(response_data.episodes)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DownloadEpisodeRequest {
    pub episode_id: i32,
    pub user_id: i32,
    pub is_youtube: bool, // Add is_youtube field
}

#[derive(Deserialize, Debug)]
struct DownloadResponse {
    detail: String,
}

pub async fn call_download_episode(
    server_name: &String,
    api_key: &Option<String>,
    request_data: &DownloadEpisodeRequest,
) -> Result<String, Error> {
    let url = format!("{}/api/data/download_podcast", server_name);

    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let request_body = serde_json::to_string(request_data)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;

    if response.ok() {
        let response_body: DownloadResponse =
            response.json().await.map_err(|e| anyhow::Error::new(e))?;
        Ok(response_body.detail)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));

        Err(anyhow::Error::msg(format!(
            "Failed to download {}: {} - {}",
            if request_data.is_youtube {
                "video"
            } else {
                "episode"
            },
            response.status_text(),
            error_text
        )))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DownloadAllPodcastRequest {
    pub podcast_id: i32,
    pub user_id: i32,
}

pub async fn call_download_all_podcast(
    server_name: &String,
    api_key: &Option<String>,
    request_data: &DownloadAllPodcastRequest,
) -> Result<String, Error> {
    let url = format!("{}/api/data/download_all_podcast", server_name);

    // Convert Option<String> to Option<&str>
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let request_body = serde_json::to_string(request_data)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;

    if response.ok() {
        // Deserialize the JSON response into DownloadResponse
        let response_body: DownloadResponse =
            response.json().await.map_err(|e| anyhow::Error::new(e))?;
        // Extract and return the detail string
        Ok(response_body.detail)
    } else {
        // Read the error response body as text to include in the error
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to download all episodes: {} - {}",
            response.status_text(),
            error_text
        )))
    }
}

pub async fn call_remove_downloaded_episode(
    server_name: &String,
    api_key: &Option<String>,
    request_data: &DownloadEpisodeRequest,
) -> Result<String, Error> {
    let url = format!("{}/api/data/delete_episode", server_name);
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;
    let request_body = serde_json::to_string(request_data)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;
    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;
    if response.ok() {
        let response_text = response.text().await.unwrap_or_else(|_| {
            if request_data.is_youtube {
                String::from("Video deleted successfully")
            } else {
                String::from("Episode deleted successfully")
            }
        });

        // Try to parse as JSON first
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&response_text) {
            // If it's a JSON object with a "detail" field, extract that message
            if let Some(detail) = json_value.get("detail") {
                if let Some(message) = detail.as_str() {
                    return Ok(message.to_string());
                }
            }
        }

        // If not JSON or no "detail" field, return the text as is
        Ok(response_text)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to delete {}: {} - {}",
            if request_data.is_youtube {
                "video"
            } else {
                "episode"
            },
            response.status_text(),
            error_text
        )))
    }
}

// Get Single Epsiode

#[derive(Debug, Deserialize, Default, Serialize, Clone, PartialEq)]
#[allow(non_snake_case)]
#[serde(default)]
pub struct EpisodeInfo {
    pub episodetitle: String,
    pub podcastname: String,
    pub podcastid: i32,
    pub podcastindexid: Option<i64>,
    pub feedurl: String,
    pub episodepubdate: String,
    pub episodedescription: String,
    pub episodeartwork: String,
    pub episodeurl: String,
    pub episodeduration: i32,
    pub listenduration: Option<i32>,
    pub episodeid: i32,
    pub completed: bool,
    pub is_queued: bool,
    pub is_saved: bool,
    pub is_downloaded: bool,
    pub is_youtube: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EpisodeRequest {
    pub episode_id: i32,
    pub user_id: i32,
    #[serde(default)]
    pub person_episode: bool,
    #[serde(default)]
    pub is_youtube: bool,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct EpisodeMetadataResponse {
    pub episode: EpisodeInfo,
}

pub async fn call_get_episode_metadata(
    server_name: &str,
    api_key: Option<String>,
    episode_request: &EpisodeRequest,
) -> Result<EpisodeInfo, anyhow::Error> {
    let url = format!("{}/api/data/get_episode_metadata", server_name);

    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let request_body = serde_json::to_string(episode_request)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;

    if !response.ok() {
        return Err(anyhow::Error::msg(format!(
            "Failed to episode downloads: {}",
            response.status_text()
        )));
    }

    let response_text = response.text().await?;

    let response_data: EpisodeMetadataResponse = serde_json::from_str(&response_text)
        .map_err(|e| anyhow::Error::msg(format!("Deserialization Error: {}", e)))?;
    Ok(response_data.episode)
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
#[allow(non_snake_case)]
pub struct Chapter {
    #[serde(deserialize_with = "deserialize_start_time")]
    pub startTime: Option<i32>, // Changed to Option<i32> with custom deserializer
    pub title: String,
    pub url: Option<String>,
    pub img: Option<String>,
}

fn deserialize_start_time<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    struct StartTimeVisitor;

    impl<'de> Visitor<'de> for StartTimeVisitor {
        type Value = Option<i32>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an integer or a floating point number as start time")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_any(self)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value as i32))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value as i32))
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value.round() as i32))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value
                .parse::<f64>()
                .map(|v| Some(v.round() as i32))
                .map_err(de::Error::custom)
        }
    }

    deserializer.deserialize_option(StartTimeVisitor)
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Transcript {
    pub url: String,
    pub mime_type: String,
    pub language: Option<String>,
    pub rel: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Person {
    pub name: String,
    pub role: Option<String>,
    pub group: Option<String>,
    pub img: Option<String>,
    pub href: Option<String>,
    pub id: Option<i32>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Podcasting2Data {
    pub chapters: Vec<Chapter>,
    pub transcripts: Vec<Transcript>,
    pub people: Vec<Person>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FetchPodcasting2DataRequest {
    pub episode_id: i32,
    pub user_id: i32,
}

pub async fn call_fetch_podcasting_2_data(
    server_name: &str,
    api_key: &Option<String>,
    episode_request: &FetchPodcasting2DataRequest,
) -> Result<Podcasting2Data, Error> {
    let url = format!(
        "{}/api/data/fetch_podcasting_2_data?episode_id={}&user_id={}",
        server_name, episode_request.episode_id, episode_request.user_id
    );

    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| Error::msg("API key is missing"))?;

    let response = Request::get(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| Error::msg(format!("Request Error: {}", e)))?;

    if !response.ok() {
        return Err(Error::msg(format!(
            "Failed to fetch podcasting 2.0 data: {}",
            response.status()
        )));
    }

    let response_text = response
        .text()
        .await
        .map_err(|e| Error::msg(format!("Failed to read response text: {}", e)))?;

    let response_data: Podcasting2Data = serde_json::from_str(&response_text)
        .map_err(|e| Error::msg(format!("Deserialization Error: {}", e)))?;

    Ok(response_data)
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct PodrollItem {
    pub feed_guid: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Funding {
    pub url: String,
    pub description: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ValueRecipient {
    pub name: String,
    pub r#type: String,
    pub address: String,
    pub split: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Value {
    pub r#type: String,
    pub method: String,
    pub suggested: Option<String>,
    pub recipients: Vec<ValueRecipient>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct Podcasting2PodData {
    pub people: Vec<Person>,
    pub podroll: Vec<PodrollItem>,
    pub funding: Vec<Funding>,
    pub value: Vec<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FetchPodcasting2PodDataRequest {
    pub podcast_id: i32,
    pub user_id: i32,
}

pub async fn call_fetch_podcasting_2_pod_data(
    server_name: &str,
    api_key: &Option<String>,
    podcast_request: &FetchPodcasting2PodDataRequest,
) -> Result<Podcasting2PodData, Error> {
    let url = format!(
        "{}/api/data/fetch_podcasting_2_pod_data?podcast_id={}&user_id={}",
        server_name, podcast_request.podcast_id, podcast_request.user_id
    );

    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| Error::msg("API key is missing"))?;

    let response = Request::get(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| Error::msg(format!("Request Error: {}", e)))?;

    if !response.ok() {
        return Err(Error::msg(format!(
            "Failed to fetch podcasting 2.0 pod data: {}",
            response.status()
        )));
    }

    let response_text = response
        .text()
        .await
        .map_err(|e| Error::msg(format!("Failed to read response text: {}", e)))?;

    let response_data: Podcasting2PodData = serde_json::from_str(&response_text)
        .map_err(|e| Error::msg(format!("Deserialization Error: {}", e)))?;

    Ok(response_data)
}

#[derive(Serialize)]
pub struct RecordListenDurationRequest {
    pub episode_id: i32,
    pub user_id: i32,
    pub listen_duration: f64,
    pub is_youtube: Option<bool>, // Add the optional is_youtube field
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct RecordListenDurationResponse {
    pub detail: String, // Assuming a simple status response; adjust according to actual API response
}

pub async fn call_record_listen_duration(
    server_name: &str,
    api_key: &str,
    request_data: RecordListenDurationRequest,
) -> Result<RecordListenDurationResponse, Error> {
    let url = format!("{}/api/data/record_listen_duration", server_name);
    let request_body = serde_json::to_string(&request_data)
        .map_err(|e| Error::msg(format!("Serialization Error: {}", e)))?;

    let response = Request::post(&url)
        .header("Content-Type", "application/json")
        .header("Api-Key", api_key)
        .body(&request_body)
        .map_err(|e| Error::msg(format!("Request Building Error: {}", e)))?
        .send()
        .await
        .map_err(|e| Error::msg(format!("Network Request Error: {}", e)))?;

    if response.ok() {
        response
            .json::<RecordListenDurationResponse>()
            .await
            .map_err(|e| Error::msg(format!("Response Parsing Error: {}", e)))
    } else {
        Err(Error::msg(format!(
            "Error recording listen duration. Server Response: {}",
            response.status_text()
        )))
    }
}

pub async fn call_increment_listen_time(
    server_name: &str,
    api_key: &str,
    user_id: i32, // Assuming user_id is an integer based on your endpoint definition
) -> Result<String, Error> {
    let url = format!("{}/api/data/increment_listen_time/{}", server_name, user_id);

    let response = Request::put(&url)
        .header("Content-Type", "application/json")
        .header("Api-Key", api_key)
        .send()
        .await
        .map_err(|e| Error::msg(format!("Network Request Error: {}", e)))?;

    if response.ok() {
        Ok("Listen time incremented.".to_string()) // Assuming a simple success message for now
    } else {
        Err(Error::msg(format!(
            "Error incrementing listen time. Server Response: {}",
            response.status_text()
        )))
    }
}

pub async fn call_increment_played(
    server_name: &str,
    api_key: &str,
    user_id: i32, // Assuming user_id is an integer based on your endpoint definition
) -> Result<String, Error> {
    let url = format!("{}/api/data/increment_played/{}", server_name, user_id);

    let response = Request::put(&url)
        .header("Content-Type", "application/json")
        .header("Api-Key", api_key)
        .send()
        .await
        .map_err(|e| Error::msg(format!("Network Request Error: {}", e)))?;

    if response.ok() {
        Ok("Played count incremented.".to_string()) // Assuming a simple success message for now
    } else {
        Err(Error::msg(format!(
            "Error incrementing played count. Server Response: {}",
            response.status_text()
        )))
    }
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct PodcastIdResponse {
    pub episodes: i32,
}

pub async fn call_get_podcast_id(
    server_name: &str,
    api_key: &Option<String>,
    user_id: &i32,
    podcast_feed: &str,
    podcast_title: &str,
) -> Result<i32, anyhow::Error> {
    // Append the user_id, podcast_feed, and podcast_title as query parameters
    let encoded_feed = utf8_percent_encode(podcast_feed, NON_ALPHANUMERIC).to_string();
    let encoded_title = utf8_percent_encode(podcast_title, NON_ALPHANUMERIC).to_string();
    let url = format!(
        "{}/api/data/get_podcast_id?user_id={}&podcast_feed={}&podcast_title={}",
        server_name, user_id, encoded_feed, encoded_title
    );

    // Convert Option<String> to Option<&str>
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let response = Request::get(&url)
        .header("Api-Key", api_key_ref)
        .send()
        .await?;

    if !response.ok() {
        return Err(anyhow::Error::msg(format!(
            "Failed to get podcast id: {}",
            response.status_text()
        )));
    }

    let response_text = response.text().await?;

    let response_data: PodcastIdResponse = serde_json::from_str(&response_text)?;
    Ok(response_data.episodes)
}

pub async fn call_get_episode_id(
    server_name: &str,
    api_key: &String,
    user_id: &i32,
    episode_title: &str,
    episode_url: &str,
    is_youtube: bool,
) -> Result<i32, anyhow::Error> {
    // Append the user_id, podcast_feed, and podcast_title as query parameters
    let encoded_feed = utf8_percent_encode(episode_url, NON_ALPHANUMERIC).to_string();
    let encoded_title = utf8_percent_encode(episode_title, NON_ALPHANUMERIC).to_string();
    let url = format!(
        "{}/api/data/get_episode_id_ep_name?user_id={}&episode_url={}&episode_title={}&is_youtube={}",
        server_name, user_id, encoded_feed, encoded_title, is_youtube
    );

    let response = Request::get(&url).header("Api-Key", api_key).send().await?;

    if !response.ok() {
        return Err(anyhow::Error::msg(format!(
            "Failed to get podcast id: {}",
            response.status_text()
        )));
    }

    let response_text = response.text().await?;

    // Try to parse the response text into an i32
    match response_text.trim().parse::<i32>() {
        Ok(episode_id) => Ok(episode_id),
        Err(_) => Err(anyhow::Error::msg(
            "Failed to parse episode ID from response",
        )),
    }
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct PodcastIdEpResponse {
    pub podcast_id: i32,
}

pub async fn call_get_podcast_id_from_ep(
    server_name: &str,
    api_key: &Option<String>,
    episode_id: i32,
    user_id: i32,
    is_youtube: Option<bool>,
) -> Result<i32, Error> {
    let mut url = format!(
        "{}/api/data/get_podcast_id_from_ep_id?episode_id={}&user_id={}",
        server_name, episode_id, user_id
    );

    // Add is_youtube parameter if it's true
    if let Some(true) = is_youtube {
        url.push_str("&is_youtube=true");
    }

    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let response = Request::get(&url)
        .header("Api-Key", api_key_ref)
        .send()
        .await?;

    if !response.ok() {
        return Err(anyhow::Error::msg(format!(
            "Failed to get podcast id: {}",
            response.status_text()
        )));
    }

    let response_text = response.text().await?;
    let response_data: PodcastIdEpResponse = serde_json::from_str(&response_text)?;
    Ok(response_data.podcast_id)
}

pub async fn call_get_podcast_id_from_ep_name(
    server_name: &str,
    api_key: &Option<String>,
    episode_name: String,
    episode_url: String,
    user_id: i32,
) -> Result<i32, Error> {
    let url = format!(
        "{}/api/data/get_podcast_id_from_ep_name?episode_name={}&episode_url={}&user_id={}",
        server_name,
        urlencoding::encode(&episode_name),
        urlencoding::encode(&episode_url),
        user_id
    );

    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let response = Request::get(&url)
        .header("Api-Key", api_key_ref)
        .send()
        .await?;

    if !response.ok() {
        return Err(anyhow::Error::msg(format!(
            "Failed to get podcast id: {}",
            response.status_text()
        )));
    }

    let response_text = response.text().await?;

    let response_data: PodcastIdEpResponse = serde_json::from_str(&response_text)?;
    Ok(response_data.podcast_id)
}

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct PodcastDetails {
    pub podcastid: i32,
    pub podcastname: String,
    pub artworkurl: String,
    pub author: String,
    pub categories: HashMap<String, String>,
    pub description: String,
    pub episodecount: i32,
    pub feedurl: String,
    pub websiteurl: String,
    pub explicit: bool,
    pub userid: i32,
    #[serde(default)] // Add this to handle null more gracefully
    pub podcastindexid: Option<i64>,
    #[serde(rename = "isyoutubechannel")]
    pub is_youtube: bool,
}

#[derive(Deserialize)]
struct PodcastDetailsResponse {
    details: PodcastDetails,
}

pub async fn call_get_podcast_details(
    server_name: &str,
    api_key: &str,
    user_id: i32,
    podcast_id: &i32,
) -> Result<PodcastDetails, Error> {
    let url = format!(
        "{}/api/data/get_podcast_details?user_id={}&podcast_id={}",
        server_name, user_id, podcast_id
    );

    let response = Request::get(&url)
        .header("Content-Type", "application/json")
        .header("Api-Key", api_key)
        .send()
        .await
        .map_err(|e| Error::msg(format!("Network request error: {}", e)))?;

    if response.ok() {
        let response_data: PodcastDetailsResponse = response
            .json()
            .await
            .map_err(|e| Error::msg(format!("Failed to parse response: {}", e)))?;

        Ok(response_data.details)
    } else {
        Err(Error::msg(format!(
            "Error retrieving podcast details. Server response: {}",
            response.status_text()
        )))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MarkEpisodeCompletedRequest {
    pub episode_id: i32,
    pub user_id: i32,
    pub is_youtube: bool,
}

#[derive(Deserialize, Debug)]
struct MarkEpisodeCompletedResponse {
    detail: String,
}

pub async fn call_mark_episode_completed(
    server_name: &String,
    api_key: &Option<String>,
    request_data: &MarkEpisodeCompletedRequest,
) -> Result<String, Error> {
    let url = format!("{}/api/data/mark_episode_completed", server_name);
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let request_body = serde_json::to_string(request_data)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;

    if response.ok() {
        let response_body: MarkEpisodeCompletedResponse =
            response.json().await.map_err(|e| anyhow::Error::new(e))?;
        Ok(response_body.detail)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to mark episode completed: {} - {}",
            response.status_text(),
            error_text
        )))
    }
}

pub async fn call_mark_episode_uncompleted(
    server_name: &String,
    api_key: &Option<String>,
    request_data: &MarkEpisodeCompletedRequest,
) -> Result<String, Error> {
    let url = format!("{}/api/data/mark_episode_uncompleted", server_name);
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let request_body = serde_json::to_string(request_data)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;

    if response.ok() {
        let response_body: MarkEpisodeCompletedResponse =
            response.json().await.map_err(|e| anyhow::Error::new(e))?;
        Ok(response_body.detail)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to mark episode uncompleted: {} - {}",
            response.status_text(),
            error_text
        )))
    }
}

// Bulk episode action structures and functions
#[derive(Serialize, Deserialize, Debug)]
pub struct BulkEpisodeActionRequest {
    pub episode_ids: Vec<i32>,
    pub user_id: i32,
    pub is_youtube: Option<bool>,
}

#[derive(Deserialize, Debug)]
struct BulkEpisodeActionResponse {
    pub message: String,
    pub processed_count: i32,
    pub failed_count: Option<i32>,
}

pub async fn call_bulk_mark_episodes_completed(
    server_name: &String,
    api_key: &Option<String>,
    request_data: &BulkEpisodeActionRequest,
) -> Result<String, Error> {
    let url = format!("{}/api/data/bulk_mark_episodes_completed", server_name);
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let request_body = serde_json::to_string(request_data)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;

    if response.ok() {
        let response_body: BulkEpisodeActionResponse =
            response.json().await.map_err(|e| anyhow::Error::new(e))?;
        Ok(response_body.message)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to bulk mark episodes completed: {} - {}",
            response.status_text(),
            error_text
        )))
    }
}

pub async fn call_bulk_save_episodes(
    server_name: &String,
    api_key: &Option<String>,
    request_data: &BulkEpisodeActionRequest,
) -> Result<String, Error> {
    let url = format!("{}/api/data/bulk_save_episodes", server_name);
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let request_body = serde_json::to_string(request_data)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;

    if response.ok() {
        let response_body: BulkEpisodeActionResponse =
            response.json().await.map_err(|e| anyhow::Error::new(e))?;
        Ok(response_body.message)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to bulk save episodes: {} - {}",
            response.status_text(),
            error_text
        )))
    }
}

pub async fn call_bulk_queue_episodes(
    server_name: &String,
    api_key: &Option<String>,
    request_data: &BulkEpisodeActionRequest,
) -> Result<String, Error> {
    let url = format!("{}/api/data/bulk_queue_episodes", server_name);
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let request_body = serde_json::to_string(request_data)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;

    if response.ok() {
        let response_body: BulkEpisodeActionResponse =
            response.json().await.map_err(|e| anyhow::Error::new(e))?;
        Ok(response_body.message)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to bulk queue episodes: {} - {}",
            response.status_text(),
            error_text
        )))
    }
}

pub async fn call_bulk_download_episodes(
    server_name: &String,
    api_key: &Option<String>,
    request_data: &BulkEpisodeActionRequest,
) -> Result<String, Error> {
    let url = format!("{}/api/data/bulk_download_episodes", server_name);
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let request_body = serde_json::to_string(request_data)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;

    if response.ok() {
        let response_body: BulkEpisodeActionResponse =
            response.json().await.map_err(|e| anyhow::Error::new(e))?;
        Ok(response_body.message)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to bulk download episodes: {} - {}",
            response.status_text(),
            error_text
        )))
    }
}

pub async fn call_bulk_delete_downloaded_episodes(
    server_name: &String,
    api_key: &Option<String>,
    request_data: &BulkEpisodeActionRequest,
) -> Result<String, Error> {
    let url = format!("{}/api/data/bulk_delete_downloaded_episodes", server_name);
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let request_body = serde_json::to_string(request_data)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;

    if response.ok() {
        let response_body: BulkEpisodeActionResponse =
            response.json().await.map_err(|e| anyhow::Error::new(e))?;
        Ok(response_body.message)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to bulk delete downloaded episodes: {} - {}",
            response.status_text(),
            error_text
        )))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AutoDownloadRequest {
    pub podcast_id: i32,
    pub user_id: i32,
    pub auto_download: bool,
}

#[derive(Deserialize, Debug)]
struct AutoDownloadResponse {
    detail: String,
}

pub async fn call_enable_auto_download(
    server_name: &String,
    api_key: &String,
    request_data: &AutoDownloadRequest,
) -> Result<String, Error> {
    let url = format!("{}/api/data/enable_auto_download", server_name);

    let request_body = serde_json::to_string(request_data)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;

    let response = Request::post(&url)
        .header("Api-Key", api_key)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;

    if response.ok() {
        let response_body: AutoDownloadResponse =
            response.json().await.map_err(|e| anyhow::Error::new(e))?;
        Ok(response_body.detail)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to enable auto-download: {} - {}",
            response.status_text(),
            error_text
        )))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AutoDownloadStatusRequest {
    pub podcast_id: i32,
    user_id: i32,
}

#[derive(Deserialize, Debug)]
pub struct AutoDownloadStatusResponse {
    pub auto_download: bool,
}

pub async fn call_get_auto_download_status(
    server_name: &str,
    user_id: i32,
    api_key: &Option<String>,
    podcast_id: i32,
) -> Result<bool, anyhow::Error> {
    let url = format!("{}/api/data/get_auto_download_status", server_name);

    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;
    let request_body = serde_json::to_string(&AutoDownloadStatusRequest {
        podcast_id,
        user_id,
    })
    .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;

    if response.ok() {
        let response_body: AutoDownloadStatusResponse =
            response.json().await.map_err(|e| anyhow::Error::new(e))?;
        Ok(response_body.auto_download)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to get auto-download status: {} - {}",
            response.status_text(),
            error_text
        )))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlaybackSpeedRequest {
    pub podcast_id: i32,
    pub user_id: i32,
    pub playback_speed: f64,
}

#[derive(Deserialize, Debug)]
struct PlaybackSpeedResponse {
    detail: String,
}

pub async fn call_set_playback_speed(
    server_name: &String,
    api_key: &Option<String>,
    request_data: &PlaybackSpeedRequest,
) -> Result<String, Error> {
    let url = format!("{}/api/data/podcast/set_playback_speed", server_name);
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;
    let request_body = serde_json::to_string(request_data)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;
    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;
    if response.ok() {
        let response_body: PlaybackSpeedResponse =
            response.json().await.map_err(|e| anyhow::Error::new(e))?;
        Ok(response_body.detail)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to set playback speed: {} - {}",
            response.status_text(),
            error_text
        )))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClearPlaybackSpeedRequest {
    pub podcast_id: i32,
    pub user_id: i32,
}

#[derive(Deserialize, Debug)]
struct ClearPlaybackSpeedResponse {
    message: String,
}

pub async fn call_clear_playback_speed(
    server_name: &String,
    api_key: &Option<String>,
    request_data: &ClearPlaybackSpeedRequest,
) -> Result<String, Error> {
    let url = format!("{}/api/data/clear_podcast_playback_speed", server_name);
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;
    let request_body = serde_json::to_string(request_data)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;
    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;
    if response.ok() {
        let response_body: ClearPlaybackSpeedResponse =
            response.json().await.map_err(|e| anyhow::Error::new(e))?;
        Ok(response_body.message)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to clear playback speed: {} - {}",
            response.status_text(),
            error_text
        )))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetPlaybackSpeedRequest {
    pub user_id: i32,
    pub podcast_id: Option<i32>,
}

#[derive(Deserialize, Debug)]
struct PlaybackSpeedGetResponse {
    playback_speed: f64,
}

pub async fn call_get_podcast_playback_speed(
    server_name: &String,
    api_key: &Option<String>,
    podcast_id: i32,
    user_id: i32,
) -> Result<f64, Error> {
    let url = format!("{}/api/data/get_playback_speed", server_name);
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let request_data = GetPlaybackSpeedRequest {
        user_id,
        podcast_id: Some(podcast_id),
    };

    let request_body = serde_json::to_string(&request_data)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;

    if response.ok() {
        let response_body: PlaybackSpeedGetResponse =
            response.json().await.map_err(|e| anyhow::Error::new(e))?;
        Ok(response_body.playback_speed)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to get playback speed: {} - {}",
            response.status_text(),
            error_text
        )))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SkipTimesRequest {
    pub podcast_id: i32,
    pub start_skip: i32,
    pub end_skip: i32,
    pub user_id: i32,
}

#[derive(Deserialize, Debug)]
struct SkipTimesResponse {
    detail: String,
}

pub async fn call_adjust_skip_times(
    server_name: &String,
    api_key: &Option<String>,
    request_data: &SkipTimesRequest,
) -> Result<String, Error> {
    let url = format!("{}/api/data/adjust_skip_times", server_name);

    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;
    let request_body = serde_json::to_string(request_data)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;

    if response.ok() {
        let response_body: SkipTimesResponse =
            response.json().await.map_err(|e| anyhow::Error::new(e))?;
        Ok(response_body.detail)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to adjust skip times: {} - {}",
            response.status_text(),
            error_text
        )))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AutoSkipTimesRequest {
    pub podcast_id: i32,
    pub user_id: i32,
}

#[derive(Deserialize, Debug)]
pub struct AutoSkipTimesResponse {
    pub start_skip: i32,
    pub end_skip: i32,
}

pub async fn call_get_auto_skip_times(
    server_name: &str,
    api_key: &Option<String>,
    user_id: i32,
    podcast_id: i32,
) -> Result<(i32, i32), Error> {
    let url = format!("{}/api/data/get_auto_skip_times", server_name);
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let request_body = serde_json::to_string(&AutoSkipTimesRequest {
        podcast_id,
        user_id,
    })?;

    let response = Request::post(&url)
        .header("Content-Type", "application/json")
        .header("Api-Key", api_key_ref)
        .body(request_body)?
        .send()
        .await?;

    if response.ok() {
        let response_data: AutoSkipTimesResponse = response.json().await?;
        Ok((response_data.start_skip, response_data.end_skip))
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error message".to_string());
        Err(Error::msg(format!(
            "Failed to get auto skip times: {}",
            error_text
        )))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayEpisodeDetailsRequest {
    pub podcast_id: i32,
    pub user_id: i32,
    pub is_youtube: bool,
}

#[derive(Deserialize, Debug)]
pub struct PlayEpisodeDetailsResponse {
    pub playback_speed: f32,
    pub start_skip: i32,
    pub end_skip: i32,
}

pub async fn call_get_play_episode_details(
    server_name: &str,
    api_key: &Option<String>,
    user_id: i32,
    podcast_id: i32,
    is_youtube: bool,
) -> Result<(f32, i32, i32), Error> {
    let url = format!("{}/api/data/get_play_episode_details", server_name);
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let request_body = serde_json::to_string(&PlayEpisodeDetailsRequest {
        podcast_id,
        user_id,
        is_youtube,
    })?;

    let response = Request::post(&url)
        .header("Content-Type", "application/json")
        .header("Api-Key", api_key_ref)
        .body(request_body)?
        .send()
        .await?;

    if response.ok() {
        let response_data: PlayEpisodeDetailsResponse = response.json().await?;
        Ok((
            response_data.playback_speed,
            response_data.start_skip,
            response_data.end_skip,
        ))
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error message".to_string());
        Err(Error::msg(format!(
            "Failed to get episode playback details: {}",
            error_text
        )))
    }
}

#[derive(Deserialize)]
struct VersionResponse {
    data: String,
}

pub async fn call_get_pinepods_version(
    server_name: String,
    api_key: &Option<String>,
) -> Result<String, Error> {
    let url = format!("{}/api/data/get_pinepods_version", server_name);
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let response = Request::get(&url)
        .header("Content-Type", "application/json")
        .header("Api-Key", api_key_ref)
        .send()
        .await?;

    if response.ok() {
        let response_text: String = response.text().await?;

        // Deserialize the JSON response
        let version_response: VersionResponse = serde_json::from_str(&response_text)
            .map_err(|e| anyhow::Error::msg(format!("Failed to parse response: {}", e)))?;

        Ok(version_response.data)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error message".to_string());
        Err(Error::msg(format!(
            "Failed to get Pinepods Version: {}",
            error_text
        )))
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[allow(non_snake_case)]
pub struct EpisodeWebsocketResponse {
    pub episode_id: i32,
    pub podcast_id: i32,
    pub title: String,
    pub description: Option<String>,
    pub audio_url: String,
    pub artwork_url: Option<String>,
    pub release_datetime: String,
    pub duration: i32,
    pub completed: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct EpisodeResponse {
    pub new_episode: EpisodeWebsocketResponse,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct RefreshProgress {
    pub current: i32,
    pub total: i32,
    pub current_podcast: String,
}

pub async fn connect_to_episode_websocket(
    server_name: &String,
    user_id: &i32,
    api_key: &str,
    nextcloud_refresh: bool,
    dispatch: Dispatch<AppState>,
) -> Result<Vec<EpisodeWebsocketResponse>, Error> {
    let clean_server_name = server_name
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let ws_protocol = if server_name.starts_with("https://") {
        "wss://"
    } else {
        "ws://"
    };
    let url = format!(
        "{}{}/ws/api/data/episodes/{}?api_key={}&nextcloud_refresh={}",
        ws_protocol, clean_server_name, user_id, api_key, nextcloud_refresh
    );

    let ws_result = WebSocket::open(&url);
    if ws_result.is_err() {
        return Err(Error::msg(format!(
            "Failed to open WebSocket: {:?}",
            ws_result.err()
        )));
    }
    let websocket = ws_result.unwrap();
    let (_write, mut read) = websocket.split();
    let mut episodes = Vec::new();

    // Create a task for the refresh operation
    let refresh_task_id = format!("feed_refresh_{}", js_sys::Date::now());

    // Add a starting task to show in notification center
    dispatch.reduce_mut(|state| {
        // Initialize active_tasks if it doesn't exist
        if state.active_tasks.is_none() {
            state.active_tasks = Some(Vec::new());
        }

        // Create an initial task
        if let Some(tasks) = &mut state.active_tasks {
            let initial_task = TaskProgress {
                task_id: refresh_task_id.clone(),
                user_id: *user_id,
                item_id: None,
                r#type: "feed_refresh".to_string(),
                progress: 0.0,
                status: "STARTED".to_string(),
                started_at: format!("{}", js_sys::Date::now()),
                completed_at: None,
                details: Some({
                    let mut details = HashMap::new();
                    details.insert(
                        "status_text".to_string(),
                        "Starting feed refresh...".to_string(),
                    );
                    details
                }),
                completion_time: None,
            };

            // Check if there's already a feed refresh task and remove it
            tasks.retain(|task| {
                task.r#type != "feed_refresh" || task.status == "SUCCESS" || task.status == "FAILED"
            });

            // Add the new task
            tasks.push(initial_task);
        }
    });

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                console::log_1(&format!("Received message: {}", text).into());
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(progress) = json.get("progress") {
                        // Handle progress updates
                        match serde_json::from_value::<RefreshProgress>(progress.clone()) {
                            Ok(progress_data) => {
                                // Update the state for the drawer display
                                dispatch.reduce_mut(|state| {
                                    state.refresh_progress = Some(progress_data.clone());

                                    // Also update the task in the notification center
                                    if let Some(tasks) = &mut state.active_tasks {
                                        // Find and update the feed refresh task
                                        if let Some(task) =
                                            tasks.iter_mut().find(|t| t.task_id == refresh_task_id)
                                        {
                                            let progress_percentage = if progress_data.total > 0 {
                                                (progress_data.current as f64
                                                    / progress_data.total as f64)
                                                    * 100.0
                                            } else {
                                                0.0
                                            };

                                            task.progress = progress_percentage;
                                            task.status = "PROGRESS".to_string();

                                            // Update the details
                                            if let Some(details) = &mut task.details {
                                                details.insert(
                                                    "current_podcast".to_string(),
                                                    progress_data.current_podcast.clone(),
                                                );
                                                details.insert(
                                                    "current".to_string(),
                                                    progress_data.current.to_string(),
                                                );
                                                details.insert(
                                                    "total".to_string(),
                                                    progress_data.total.to_string(),
                                                );
                                                details.insert(
                                                    "status_text".to_string(),
                                                    format!(
                                                        "Refreshing {}/{}: {}",
                                                        progress_data.current,
                                                        progress_data.total,
                                                        progress_data.current_podcast
                                                    ),
                                                );
                                            }
                                        }
                                    }
                                });
                            }
                            Err(e) => {
                                console::log_1(&format!("Failed to parse progress: {}", e).into());
                            }
                        }
                    } else if let Some(new_episode) = json.get("new_episode") {
                        match serde_json::from_value::<EpisodeWebsocketResponse>(
                            new_episode.clone(),
                        ) {
                            Ok(episode) => {
                                episodes.push(episode.clone());
                                console::log_1(
                                    &format!("Received new episode: {:?}", episode).into(),
                                );
                            }
                            Err(e) => {
                                console::log_1(
                                    &format!(
                                        "Failed to parse episode: {}. Raw episode data: {:?}",
                                        e, new_episode
                                    )
                                    .into(),
                                );
                            }
                        }
                    } else if let Some(detail) = json.get("detail") {
                        console::log_1(&format!("Received status message: {}", detail).into());
                    }
                } else {
                    console::log_1(&format!("Failed to parse JSON: {}", text).into());
                }
            }
            Ok(Message::Bytes(_)) => {
                console::log_1(&"Binary message received, ignoring".into());
            }
            Err(WebSocketError::ConnectionClose(close_event)) => {
                console::log_1(&format!("WebSocket closed: {:?}", close_event).into());

                // Mark task as completed when websocket closes
                dispatch.reduce_mut(|state| {
                    // Clear progress indicator
                    state.refresh_progress = None;

                    // Update task status
                    if let Some(tasks) = &mut state.active_tasks {
                        if let Some(task) = tasks.iter_mut().find(|t| t.task_id == refresh_task_id)
                        {
                            task.status = "SUCCESS".to_string();
                            task.progress = 100.0;
                            task.completed_at = Some(format!("{}", js_sys::Date::now()));
                            task.completion_time = Some(js_sys::Date::now());

                            // Update status text
                            if let Some(details) = &mut task.details {
                                details.insert(
                                    "status_text".to_string(),
                                    "Feed refresh completed".to_string(),
                                );
                            }
                        }
                    }
                });
                break;
            }
            Err(e) => {
                console::log_1(&format!("WebSocket error: {:?}", e).into());

                // Mark task as failed on error
                dispatch.reduce_mut(|state| {
                    // Clear progress indicator
                    state.refresh_progress = None;

                    // Update task status to failed
                    if let Some(tasks) = &mut state.active_tasks {
                        if let Some(task) = tasks.iter_mut().find(|t| t.task_id == refresh_task_id)
                        {
                            task.status = "FAILED".to_string();
                            task.completed_at = Some(format!("{}", js_sys::Date::now()));
                            task.completion_time = Some(js_sys::Date::now());

                            // Update status text
                            if let Some(details) = &mut task.details {
                                details.insert(
                                    "status_text".to_string(),
                                    format!("Feed refresh failed: {:?}", e),
                                );
                            }
                        }
                    }
                });
                break;
            }
        }
    }
    Ok(episodes)
}

#[derive(Deserialize, Debug)]
struct ShareLinkResponse {
    url_key: String,
}

pub async fn call_create_share_link(
    server_name: &String,
    api_key: &String,
    episode_id: i32,
) -> Result<String, anyhow::Error> {
    let url = format!("{}/api/data/share_episode/{}", server_name, episode_id);

    let response = Request::post(&url)
        .header("Api-Key", api_key)
        .send()
        .await?;

    if response.ok() {
        let response_body: ShareLinkResponse =
            response.json().await.map_err(|e| anyhow::Error::new(e))?;
        Ok(response_body.url_key)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to create share link: {} - {}",
            response.status_text(),
            error_text
        )))
    }
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct EpisodeMetadata {
    pub podcastid: i32,
    pub podcastname: String,
    pub feedurl: String,
    pub artworkurl: String,
    pub episodeid: i32,
    pub episodetitle: String,
    pub episodepubdate: String,
    pub episodedescription: String,
    pub episodeartwork: String,
    pub episodeurl: String,
    pub episodeduration: i32,
    pub completed: bool,
    pub is_youtube: bool,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct SharedEpisodeResponse {
    pub episode: EpisodeMetadata,
}

pub async fn call_get_episode_by_url_key(
    server_name: &String,
    url_key: &str,
) -> Result<SharedEpisodeResponse, anyhow::Error> {
    let url = format!("{}/api/data/episode_by_url/{}", server_name, url_key);

    let response = Request::get(&url).send().await?;

    if response.ok() {
        let response_body: SharedEpisodeResponse = response.json().await?;
        Ok(response_body)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to fetch episode data"));
        Err(anyhow::Error::msg(format!(
            "Failed to fetch episode by url_key: {} - {}",
            response.status_text(),
            error_text
        )))
    }
}

#[derive(Serialize)]
pub struct AddCategoryRequest {
    pub(crate) podcast_id: i32,
    pub(crate) user_id: i32,
    pub(crate) category: String,
}

pub async fn call_add_category(
    server_name: &String,
    api_key: &Option<String>,
    request_data: &AddCategoryRequest,
) -> Result<String, Error> {
    let url = format!("{}/api/data/add_category", server_name);

    // Convert Option<String> to Option<&str>
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let request_body = serde_json::to_string(request_data)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;

    if response.ok() {
        let success_message = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Category added successfully"));
        Ok(success_message)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to add category: {} - {}",
            response.status_text(),
            error_text
        )))
    }
}

#[derive(Serialize)]
pub struct UpdateFeedCutoffDaysRequest {
    pub(crate) podcast_id: i32,
    pub(crate) user_id: i32,
    pub(crate) feed_cutoff_days: i32,
}

#[derive(Serialize)]
pub struct GetFeedCutoffDaysRequest {
    pub(crate) podcast_id: i32,
    pub(crate) user_id: i32,
}

pub async fn call_update_feed_cutoff_days(
    server_name: &String,
    api_key: &Option<String>,
    request_data: &UpdateFeedCutoffDaysRequest,
) -> Result<String, Error> {
    let url = format!("{}/api/data/update_feed_cutoff_days", server_name);

    // Convert Option<String> to Option<&str>
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let request_body = serde_json::to_string(request_data)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;

    if response.ok() {
        let success_message = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Feed cutoff days updated successfully"));
        Ok(success_message)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to update feed cutoff days: {} - {}",
            response.status_text(),
            error_text
        )))
    }
}

pub async fn call_get_feed_cutoff_days(
    server_name: &String,
    api_key: &Option<String>,
    podcast_id: i32,
    user_id: i32,
) -> Result<i32, Error> {
    let url = format!(
        "{}/api/data/get_feed_cutoff_days?podcast_id={}&user_id={}",
        server_name, podcast_id, user_id
    );

    // Convert Option<String> to Option<&str>
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let response = Request::get(&url)
        .header("Api-Key", api_key_ref)
        .send()
        .await?;

    if response.ok() {
        let response_text = response
            .text()
            .await
            .map_err(|e| anyhow::Error::msg(format!("Failed to read response: {}", e)))?;

        let response_data: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| anyhow::Error::msg(format!("Failed to parse JSON: {}", e)))?;

        let feed_cutoff_days = response_data["feed_cutoff_days"]
            .as_i64()
            .ok_or_else(|| anyhow::Error::msg("Feed cutoff days not found in response"))?;

        Ok(feed_cutoff_days as i32)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to get feed cutoff days: {} - {}",
            response.status_text(),
            error_text
        )))
    }
}

#[derive(Serialize)]
pub struct RemoveCategoryRequest {
    pub(crate) podcast_id: i32,
    pub(crate) user_id: i32,
    pub(crate) category: String,
}

pub async fn call_remove_category(
    server_name: &String,
    api_key: &Option<String>,
    request_data: &RemoveCategoryRequest,
) -> Result<String, Error> {
    let url = format!("{}/api/data/remove_category", server_name);

    // Convert Option<String> to Option<&str>
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let request_body = serde_json::to_string(request_data)
        .map_err(|e| anyhow::Error::msg(format!("Serialization Error: {}", e)))?;

    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(request_body)?
        .send()
        .await?;

    if response.ok() {
        let success_message = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Category removed successfully"));
        Ok(success_message)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to remove category: {} - {}",
            response.status_text(),
            error_text
        )))
    }
}

pub async fn call_toggle_podcast_notifications(
    server_name: String,
    api_key: String,
    user_id: i32,
    podcast_id: i32,
    enabled: bool,
) -> Result<NotificationResponse, Error> {
    let url = format!("{}/api/data/podcast/toggle_notifications", server_name);
    let body = serde_json::json!({
        "user_id": user_id,
        "podcast_id": podcast_id,
        "enabled": enabled
    });

    let response = Request::put(&url)
        .header("Api-Key", &api_key)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .map_err(|e| Error::msg(format!("Failed to create request: {}", e)))?
        .send()
        .await
        .map_err(|e| Error::msg(format!("Network error: {}", e)))?;

    if response.ok() {
        response
            .json::<NotificationResponse>()
            .await
            .map_err(|e| Error::msg(format!("Error parsing JSON: {}", e)))
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(Error::msg(format!(
            "Error toggling podcast notifications: {}",
            error_text
        )))
    }
}

pub async fn call_get_podcast_notifications_status(
    server_name: String,
    api_key: String,
    user_id: i32,
    podcast_id: i32,
) -> Result<bool, Error> {
    let url = format!("{}/api/data/podcast/notification_status", server_name);
    let body = serde_json::json!({
        "user_id": user_id,
        "podcast_id": podcast_id,
    });

    let response = Request::post(&url)
        .header("Api-Key", &api_key)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .map_err(|e| Error::msg(format!("Failed to create request: {}", e)))?
        .send()
        .await
        .map_err(|e| Error::msg(format!("Network error: {}", e)))?;

    if response.ok() {
        #[derive(Deserialize)]
        struct NotificationResponse {
            enabled: bool,
        }

        response
            .json::<NotificationResponse>()
            .await
            .map(|res| res.enabled)
            .map_err(|e| Error::msg(format!("Error parsing JSON: {}", e)))
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(Error::msg(format!(
            "Error fetching podcast notification status: {}",
            error_text
        )))
    }
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct YouTubeSubscribeResponse {
    pub success: bool,
    pub podcast_id: i32,
    pub message: String,
}

pub async fn call_subscribe_to_channel(
    server: &str,
    api_key: &str,
    user_id: i32,
    channel_id: &str,
) -> Result<YouTubeSubscribeResponse, anyhow::Error> {
    // Build endpoint with both parameters in query string
    let endpoint = format!(
        "{}/api/data/youtube/subscribe?channel_id={}&user_id={}",
        server, channel_id, user_id
    );

    let resp = Request::post(&endpoint)
        .header("Api-Key", api_key)
        .send()
        .await
        .context("Network Request Error")?;

    if resp.ok() {
        resp.json::<YouTubeSubscribeResponse>()
            .await
            .context("Response Parsing Error")
    } else {
        let status = resp.status();
        let error_text = resp.text().await.unwrap_or_default();
        Err(anyhow::anyhow!(
            "Error subscribing to channel. Status: {}, Error: {}",
            status,
            error_text
        ))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RemoveYouTubeChannelValues {
    pub user_id: i32,
    pub channel_name: String,
    pub channel_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct YoutubeChannelResponse {
    pub success: bool,
}

pub async fn call_remove_youtube_channel(
    server_name: &String,
    api_key: &Option<String>,
    remove_channel: &RemoveYouTubeChannelValues,
) -> Result<bool, Error> {
    let url = format!("{}/api/data/remove_youtube_channel", server_name);
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;
    let json_body = serde_json::to_string(remove_channel)?;
    let response = Request::post(&url)
        .header("Api-Key", api_key_ref)
        .header("Content-Type", "application/json")
        .body(json_body)?
        .send()
        .await?;
    let response_text = response
        .text()
        .await
        .unwrap_or_else(|_| "Failed to get response text".to_string());
    if response.ok() {
        match serde_json::from_str::<YoutubeChannelResponse>(&response_text) {
            Ok(parsed_response) => Ok(parsed_response.success),
            Err(_parse_error) => Err(anyhow::Error::msg("Failed to parse response")),
        }
    } else {
        Err(anyhow::Error::msg(format!(
            "Error removing channel: {}",
            response.status_text()
        )))
    }
}

#[derive(Default, Deserialize, Debug)]
pub struct CheckYouTubeChannelResponse {
    pub exists: bool,
}

pub async fn call_check_youtube_channel(
    server: &str,
    api_key: &str,
    user_id: i32,
    channel_name: &str,
    channel_url: &str,
) -> Result<CheckYouTubeChannelResponse, Error> {
    let encoded_name = utf8_percent_encode(channel_name, NON_ALPHANUMERIC).to_string();
    let encoded_url = utf8_percent_encode(channel_url, NON_ALPHANUMERIC).to_string();
    let endpoint = format!(
        "{}/api/data/check_youtube_channel?user_id={}&channel_name={}&channel_url={}",
        server, user_id, encoded_name, encoded_url
    );
    let resp = Request::get(&endpoint)
        .header("Api-Key", api_key)
        .send()
        .await
        .context("Network Request Error")?;
    if resp.ok() {
        resp.json::<CheckYouTubeChannelResponse>()
            .await
            .context("Response Parsing Error")
    } else {
        Err(anyhow::anyhow!(
            "Error checking YouTube channel. Server Response: {}",
            resp.status_text()
        ))
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct HomePodcast {
    pub podcastid: i32,
    pub podcastname: String,
    pub podcastindexid: Option<i64>,
    pub artworkurl: Option<String>,
    pub author: Option<String>,
    pub categories: Option<HashMap<String, String>>,
    pub description: Option<String>,
    pub episodecount: Option<i32>,
    pub feedurl: Option<String>,
    pub websiteurl: Option<String>,
    pub explicit: Option<bool>,
    pub is_youtube: bool, // This maps to isyoutubechannel in the DB
    pub play_count: i32,
    pub total_listen_time: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct HomeEpisode {
    pub episodeid: i32,
    pub podcastid: i32,
    pub episodetitle: String,
    pub episodedescription: String,
    pub episodeurl: String,
    pub episodeartwork: String,
    pub episodepubdate: String,
    pub episodeduration: i32,
    pub completed: bool,
    pub podcastname: String,
    pub is_youtube: bool,
    #[serde(default)]
    pub listenduration: Option<i32>,
    #[serde(default)]
    pub saved: bool,
    #[serde(default)]
    pub queued: bool,
    #[serde(default)]
    pub downloaded: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct HomeOverview {
    pub recent_episodes: Vec<HomeEpisode>,
    pub in_progress_episodes: Vec<HomeEpisode>,
    pub top_podcasts: Vec<HomePodcast>,
    pub saved_count: i32,
    pub downloaded_count: i32,
    pub queue_count: i32,
}

pub async fn call_get_home_overview(
    server: &str,
    api_key: &str,
    user_id: i32,
) -> Result<HomeOverview, Error> {
    let endpoint = format!("{}/api/data/home_overview?user_id={}", server, user_id);
    let response = Request::get(&endpoint)
        .header("Api-Key", api_key)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Network request failed: {}", e))?;

    if response.ok() {
        let text = response.text().await?;

        // Validation steps
        if text.is_empty() {
            return Err(anyhow::anyhow!("Empty response from server"));
        }

        // Check if response is valid JSON
        if !text.starts_with('{') || !text.ends_with('}') {
            return Err(anyhow::anyhow!(
                "Invalid JSON response - does not start with {{ and end with }}. \
                Response starts with: {:?} \
                Response ends with: {:?}",
                &text.chars().take(50).collect::<String>(),
                &text.chars().rev().take(50).collect::<String>()
            ));
        }

        // Try parsing with detailed error
        match serde_json::from_str::<HomeOverview>(&text) {
            Ok(parsed) => Ok(parsed),
            Err(e) => {
                let context_start = e.column().saturating_sub(100);
                let context_end = (e.column() + 100).min(text.len());
                let context = &text[context_start..context_end];

                Err(anyhow::anyhow!(
                    "JSON parse error: {} \nContext around position {}: '...{}...'",
                    e,
                    e.column(),
                    context
                ))
            }
        }
    } else {
        Err(anyhow::anyhow!(
            "Server returned error: {}",
            response.status_text()
        ))
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PlaylistEpisode {
    pub title: String,
    pub artwork: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Playlist {
    pub playlist_id: i32,
    pub user_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub is_system_playlist: bool,
    pub podcast_ids: Option<Vec<i32>>,
    pub include_unplayed: bool,
    pub include_partially_played: bool,
    pub include_played: bool,
    pub min_duration: Option<i32>,
    pub max_duration: Option<i32>,
    pub sort_order: String,
    pub group_by_podcast: bool,
    pub max_episodes: Option<i32>,
    pub last_updated: String,
    pub created: String,
    pub episode_count: Option<i32>,
    pub preview_episodes: Vec<PlaylistEpisode>,
    pub icon_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlaylistResponse {
    pub playlists: Vec<Playlist>,
}

pub async fn call_get_playlists(
    server: &str,
    api_key: &str,
    user_id: i32,
) -> Result<PlaylistResponse, Error> {
    let endpoint = format!("{}/api/data/get_playlists?user_id={}", server, user_id);
    let response = Request::get(&endpoint)
        .header("Api-Key", api_key)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Network request failed: {}", e))?;

    if response.ok() {
        let text = response.text().await?;
        if text.is_empty() {
            return Err(anyhow::anyhow!("Empty response from server"));
        }

        match serde_json::from_str::<PlaylistResponse>(&text) {
            Ok(parsed) => Ok(parsed),
            Err(e) => {
                let context_start = e.column().saturating_sub(100);
                let context_end = (e.column() + 100).min(text.len());
                let context = &text[context_start..context_end];
                Err(anyhow::anyhow!(
                    "JSON parse error: {} \nContext around position {}: '...{}...'",
                    e,
                    e.column(),
                    context
                ))
            }
        }
    } else {
        Err(anyhow::anyhow!(
            "Server returned error: {}",
            response.status()
        ))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CreatePlaylistRequest {
    pub user_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub podcast_ids: Option<Vec<i32>>,
    pub include_unplayed: bool,
    pub include_partially_played: bool,
    pub include_played: bool,
    pub min_duration: Option<i32>,
    pub max_duration: Option<i32>,
    pub sort_order: String,
    pub group_by_podcast: bool,
    pub max_episodes: Option<i32>,
    pub icon_name: String,
    pub play_progress_min: Option<f32>,
    pub play_progress_max: Option<f32>,
    pub time_filter_hours: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct CreatePlaylistResponse {
    pub detail: String,
    pub playlist_id: i32,
}

pub async fn call_create_playlist(
    server: &str,
    api_key: &str,
    playlist_data: CreatePlaylistRequest,
) -> Result<CreatePlaylistResponse, Error> {
    let endpoint = format!("{}/api/data/create_playlist", server);
    let response = Request::post(&endpoint)
        .header("Api-Key", api_key)
        .json(&playlist_data)?
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Network request failed: {}", e))?;

    if response.ok() {
        let text = response.text().await?;
        if text.is_empty() {
            return Err(anyhow::anyhow!("Empty response from server"));
        }

        match serde_json::from_str::<CreatePlaylistResponse>(&text) {
            Ok(parsed) => Ok(parsed),
            Err(e) => {
                let context_start = e.column().saturating_sub(100);
                let context_end = (e.column() + 100).min(text.len());
                let context = &text[context_start..context_end];
                Err(anyhow::anyhow!(
                    "JSON parse error: {} \nContext around position {}: '...{}...'",
                    e,
                    e.column(),
                    context
                ))
            }
        }
    } else {
        Err(anyhow::anyhow!(
            "Server returned error: {}",
            response.status()
        ))
    }
}

pub async fn call_delete_playlist(
    server: &str,
    api_key: &str,
    user_id: i32,
    playlist_id: i32,
) -> Result<(), Error> {
    let endpoint = format!("{}/api/data/delete_playlist", server);
    let data = serde_json::json!({
        "user_id": user_id,
        "playlist_id": playlist_id
    });

    let response = Request::delete(&endpoint)
        .header("Api-Key", api_key)
        .json(&data)?
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Network request failed: {}", e))?;

    if response.ok() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Server returned error: {}",
            response.status()
        ))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlaylistEpisodesResponse {
    pub episodes: Vec<Episode>,
    pub playlist_info: PlaylistInfo,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PlaylistInfo {
    pub name: String,
    pub description: Option<String>,
    pub episode_count: Option<i32>, // Changed from i32 to Option<i32>
    pub icon_name: Option<String>,  // Changed from String to Option<String>
}

pub async fn call_get_playlist_episodes(
    server: &str,
    api_key: &str,
    user_id: &i32,
    playlist_id: i32,
) -> Result<PlaylistEpisodesResponse, Error> {
    let endpoint = format!(
        "{}/api/data/get_playlist_episodes?user_id={}&playlist_id={}",
        server, user_id, playlist_id
    );

    let response = Request::get(&endpoint)
        .header("Api-Key", api_key)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Network request failed: {}", e))?;

    if response.ok() {
        let text = response.text().await?;
        if text.is_empty() {
            return Ok(PlaylistEpisodesResponse {
                playlist_info: PlaylistInfo {
                    name: "Unknown".to_string(),
                    description: None,
                    episode_count: None,
                    icon_name: None,
                },
                episodes: vec![],
            });
        }

        match serde_json::from_str::<PlaylistEpisodesResponse>(&text) {
            Ok(parsed) => Ok(parsed),
            Err(_) => {
                // If parse fails, try parsing as a more basic structure
                #[derive(Deserialize)]
                struct BasicResponse {
                    playlist_info: PlaylistInfo,
                    episodes: Vec<Episode>,
                }

                let basic = serde_json::from_str::<BasicResponse>(&text)?;
                Ok(PlaylistEpisodesResponse {
                    playlist_info: basic.playlist_info,
                    episodes: basic.episodes,
                })
            }
        }
    } else {
        Err(anyhow::anyhow!(
            "Server returned error: {}",
            response.status()
        ))
    }
}

#[derive(Deserialize, Debug)]
pub struct RssKeyResponse {
    pub rss_key: String,
}

pub async fn call_get_rss_key(
    server_name: &str,
    api_key: &Option<String>,
    user_id: i32,
) -> Result<String, anyhow::Error> {
    let url = format!("{}/api/data/get_rss_key?user_id={}", server_name, user_id);
    
    let api_key_ref = api_key
        .as_deref()
        .ok_or_else(|| anyhow::Error::msg("API key is missing"))?;

    let response = Request::get(&url)
        .header("Api-Key", api_key_ref)
        .send()
        .await?;

    if response.ok() {
        let rss_key_response: RssKeyResponse = response
            .json()
            .await
            .map_err(|e| anyhow::Error::new(e))?;
        Ok(rss_key_response.rss_key)
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("Failed to read error message"));
        Err(anyhow::Error::msg(format!(
            "Failed to get RSS key: {} - {}",
            response.status_text(),
            error_text
        )))
    }
}
