use gloo_net::http::Request;
use serde::{Deserialize, Deserializer, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::fmt;

fn null_as_zero<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    // Option<i32> will deserialize null as None, or a number as Some(value)
    let opt = Option::<i32>::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

#[derive(Deserialize, Debug, PartialEq, Clone, Serialize, Default)]
#[serde(default)]
#[allow(non_snake_case)]
pub struct Episode {
    pub podcastid: i32,
    pub podcastname: String,
    #[serde(alias = "Episodetitle")]
    pub episodetitle: String,
    pub description: String,
    pub artworkurl: String,
    pub author: String,
    pub categories: Option<HashMap<String, String>>,
    #[serde(alias = "Episodedescription")]
    pub episodedescription: String,
    pub episodecount: Option<i32>,
    pub feedurl: String,
    pub websiteurl: String,
    pub explicit: i32,
    pub userid: i32,
    #[serde(alias = "Episodeid")]
    pub episodeid: i32,
    #[serde(alias = "Episodeurl")]
    pub episodeurl: String,
    #[serde(alias = "Episodeartwork")]
    pub episodeartwork: String,
    #[serde(alias = "Episodepubdate")]
    pub episodepubdate: String,
    #[serde(alias = "Episodeduration")]
    pub episodeduration: i32,
    #[serde(alias = "Listenduration", deserialize_with = "null_as_zero")]
    pub listenduration: i32,
    #[serde(alias = "Completed")]
    pub completed: bool,
    pub saved: bool,
    pub queued: bool,
    pub downloaded: bool,
    pub is_youtube: bool,
    pub guid: String,
    pub queueposition: Option<i32>,
    pub downloadedlocation: Option<String>,
}

impl Episode {
    pub fn get_episode_artwork(&self) -> String {
        self.episodeartwork.clone()
    }

    pub fn get_episode_title(&self) -> String {
        self.episodetitle.clone()
    }

    pub fn get_is_youtube(&self) -> bool {
        self.is_youtube
    }

    pub fn get_episode_id(&self, _fallback_id: Option<i32>) -> i32 {
        self.episodeid.clone()
    }

    pub fn as_any(&self) -> &dyn Any {
        self
    }

    /// Parse json Value to populate an Episode struct
    /// Any field not included in the json will be filled with its default value
    ///
    /// Surely there is a better way? Just converts the Value back to string, then deserializes
    /// that string into an Espiode.
    pub fn from_json(json: &serde_json::value::Value) -> Result<Self, serde_json::Error> {
        let j = serde_json::to_string(json)?;
        serde_json::from_str::<Self>(&j)
    }
}
