use serde::{Deserialize, Serialize};
use anyhow::Result;
use reqwest::blocking::Client as ReqwestClient;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SearchResponse {
    pub data: Vec<SearchResult>,
    pub pagination: Pagination,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SearchResult {
    pub entity_type: String,
    pub entity_id: String,
    pub quality_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Pagination {
    pub has_more: bool,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailResponse<T> {
    pub data: T,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Artist {
    pub artist_id: Option<String>,
    pub name: Option<String>,
    pub sort_name: Option<String>,
    pub area: Option<String>,
    pub begin_year: Option<i32>,
    pub end_year: Option<i32>,
    pub url: Option<String>,
    pub aliases: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub image_url: Option<String>,
    pub updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Release {
    pub release_id: Option<String>,
    pub name: Option<String>,
    pub title: Option<String>,
    pub release_date: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub artist_credit: Option<ArtistCredit>,
    pub tracks: Option<Vec<Track>>,
    pub updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Recording {
    pub recording_id: Option<String>,
    pub name: Option<String>,
    pub title: Option<String>,
    pub duration_secs: Option<i32>,
    pub image_url: Option<String>,
    pub artist_credit: Option<ArtistCredit>,
    pub releases: Option<Vec<ReleaseReference>>,
    pub sources: Option<Vec<Source>>,
    pub updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Feed {
    pub feed_guid: Option<String>,
    pub title: Option<String>,
    pub name: Option<String>,
    pub feed_url: Option<String>,
    pub release_artist: Option<String>,
    pub release_artist_sort: Option<String>,
    pub raw_medium: Option<String>,
    pub release_kind: Option<String>,
    pub release_date: Option<String>,
    pub publisher_text: Option<String>,
    pub language: Option<String>,
    pub explicit: Option<bool>,
    pub episode_count: Option<i32>,
    pub newest_item_at: Option<i64>,
    pub oldest_item_at: Option<i64>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Track {
    pub track_guid: Option<String>,
    pub feed_guid: Option<String>,
    pub title: Option<String>,
    pub name: Option<String>,
    pub duration_secs: Option<i32>,
    pub pub_date: Option<i64>,
    pub track_number: Option<i32>,
    pub explicit: Option<bool>,
    pub description: Option<String>,
    pub enclosure_url: Option<String>,
    pub image_url: Option<String>,
    pub artist_credit: Option<ArtistCredit>,
    pub updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ArtistCredit {
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ReleaseReference {
    pub position: Option<i32>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Source {
    pub title: Option<String>,
    pub primary_enclosure_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityDetail {
    Artist(Artist),
    Release(Release),
    Recording(Recording),
    Feed(Feed),
    Track(Track),
}

pub struct Client {
    client: ReqwestClient,
    base_url: String,
}

impl Client {
    pub fn new() -> Self {
        Client {
            client: ReqwestClient::new(),
            base_url: "https://musicindex.org".to_string(),
        }
    }

    pub fn new_with_base_url(base_url: String) -> Self {
        Client {
            client: ReqwestClient::new(),
            base_url,
        }
    }

    pub fn search(
        &self,
        query: &str,
        entity_type: Option<&str>,
        limit: Option<i32>,
        cursor: Option<&str>,
    ) -> Result<SearchResponse> {
        let mut url = format!("{}/v1/search?q={}", self.base_url, query);

        if let Some(etype) = entity_type {
            url.push_str(&format!("&type={}", etype));
        }

        let limit_val = limit.unwrap_or(20);
        url.push_str(&format!("&limit={}", limit_val));

        if let Some(c) = cursor {
            url.push_str(&format!("&cursor={}", c));
        }

        let response = self.client.get(&url).send()?;
        let search_response: SearchResponse = response.json()?;

        Ok(search_response)
    }

    pub fn fetch_detail(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<EntityDetail> {
        match entity_type {
            "artist" => {
                let url = format!("{}/v1/artists/{}", self.base_url, entity_id);
                let response = self.client.get(&url).send()?;
                let detail: DetailResponse<Artist> = response.json()?;
                Ok(EntityDetail::Artist(detail.data))
            }
            "release" => {
                let url = format!("{}/v1/releases/{}?include=tracks", self.base_url, entity_id);
                let response = self.client.get(&url).send()?;
                let detail: DetailResponse<Release> = response.json()?;
                Ok(EntityDetail::Release(detail.data))
            }
            "recording" => {
                let url = format!(
                    "{}/v1/recordings/{}?include=sources,releases",
                    self.base_url, entity_id
                );
                let response = self.client.get(&url).send()?;
                let detail: DetailResponse<Recording> = response.json()?;
                Ok(EntityDetail::Recording(detail.data))
            }
            "feed" => {
                let url = format!("{}/v1/feeds/{}", self.base_url, entity_id);
                let response = self.client.get(&url).send()?;
                let detail: DetailResponse<Feed> = response.json()?;
                Ok(EntityDetail::Feed(detail.data))
            }
            "track" => {
                let url = format!("{}/v1/tracks/{}", self.base_url, entity_id);
                let response = self.client.get(&url).send()?;
                let detail: DetailResponse<Track> = response.json()?;
                Ok(EntityDetail::Track(detail.data))
            }
            _ => Err(anyhow::anyhow!("Unknown entity type: {}", entity_type)),
        }
    }
}

impl Default for Client {
    fn default() -> Self {
        Client::new()
    }
}
