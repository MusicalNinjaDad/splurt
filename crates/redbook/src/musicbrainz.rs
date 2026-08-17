use serde::Deserialize;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Deserialize)]
pub struct DiscId {
    pub offsets: Vec<u64>,
    pub releases: Vec<Release>,
    #[serde(rename = "offset-count")]
    pub offset_count: Option<u32>,
    pub sectors: Option<u64>,
    pub id: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Deserialize)]
pub struct Release {
    #[serde(rename = "text-representation")]
    pub text_representation: Option<TextRepresentation>,
    #[serde(rename = "release-events")]
    pub release_events: Option<Vec<ReleaseEvent>>,
    #[serde(rename = "status-id")]
    pub status_id: Option<String>,
    pub quality: Option<String>,
    pub media: Option<Vec<Media>>,
    pub status: Option<String>,
    pub country: Option<String>,
    pub date: Option<String>,
    pub packaging: Option<String>,
    pub disambiguation: Option<String>,
    #[serde(rename = "cover-art-archive")]
    pub cover_art_archive: Option<CoverArtArchive>,
    pub barcode: Option<String>,
    #[serde(rename = "packaging-id")]
    pub packaging_id: Option<String>,
    pub asin: Option<String>,
    pub title: String,
    pub id: String,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Option<Vec<ArtistCredit>>,
    #[serde(rename = "release-group")]
    pub release_group: Option<ReleaseGroup>,
    #[serde(rename = "label-info-list")]
    pub label_info_list: Option<Vec<LabelInfo>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Deserialize)]
pub struct TextRepresentation {
    pub language: Option<String>,
    pub script: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Deserialize)]
pub struct ReleaseEvent {
    pub date: Option<String>,
    pub area: Option<Area>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Deserialize)]
pub struct Area {
    #[serde(rename = "sort-name")]
    pub sort_name: Option<String>,
    #[serde(rename = "type-id")]
    pub type_id: Option<String>,
    pub disambiguation: Option<String>,
    pub name: String,
    #[serde(rename = "iso-3166-1-codes")]
    pub iso_3166_1_codes: Option<Vec<String>>,
    pub r#type: Option<String>,
    pub id: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Deserialize)]
pub struct CoverArtArchive {
    pub front: Option<bool>,
    pub count: Option<u32>,
    pub darkened: Option<bool>,
    pub back: Option<bool>,
    pub artwork: Option<bool>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Deserialize)]
pub struct Media {
    #[serde(rename = "format-id")]
    pub format_id: Option<String>,
    pub tracks: Option<Vec<Track>>,
    pub discs: Option<Vec<Disc>>,
    pub id: Option<String>,
    #[serde(rename = "track-count")]
    pub track_count: Option<u32>,
    pub title: Option<String>,
    pub format: Option<String>,
    pub position: Option<u32>,
    #[serde(rename = "track-offset")]
    pub track_offset: Option<u32>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Deserialize)]
pub struct Track {
    pub position: Option<u32>,
    pub recording: Option<Recording>,
    pub number: Option<String>,
    pub title: Option<String>,
    pub id: Option<String>,
    pub length: Option<u64>,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Option<Vec<ArtistCredit>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Deserialize)]
pub struct Recording {
    pub title: String,
    pub id: String,
    #[serde(rename = "first-release-date")]
    pub first_release_date: Option<String>,
    pub length: Option<u64>,
    pub video: Option<bool>,
    pub disambiguation: Option<String>,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Option<Vec<ArtistCredit>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Deserialize)]
pub struct Disc {
    pub offsets: Option<Vec<u64>>,
    pub sectors: Option<u64>,
    #[serde(rename = "offset-count")]
    pub offset_count: Option<u32>,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Deserialize)]
pub struct ArtistCredit {
    pub name: String,
    pub artist: Option<Artist>,
    #[serde(rename = "joinphrase")]
    pub join_phrase: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Deserialize)]
pub struct Artist {
    pub name: String,
    pub id: Option<String>,
    #[serde(rename = "sort-name")]
    pub sort_name: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Deserialize)]
pub struct ReleaseGroup {
    pub id: Option<String>,
    #[serde(rename = "type-id")]
    pub type_id: Option<String>,
    pub r#type: Option<String>,
    #[serde(rename = "primary-type-id")]
    pub primary_type_id: Option<String>,
    pub primary_type: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Deserialize)]
pub struct LabelInfo {
    #[serde(rename = "catalog-number")]
    pub catalog_number: Option<String>,
    pub label: Option<Label>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Deserialize)]
pub struct Label {
    pub name: String,
    pub id: Option<String>,
}

pub trait ArtistCreditsExt {
    fn names(&self) -> impl Iterator<Item = String>;
}

impl ArtistCreditsExt for Vec<ArtistCredit> {
    fn names(&self) -> impl Iterator<Item = String> {
        self.iter().map(|credit| credit.name.clone())
    }
}

impl ArtistCreditsExt for Option<Vec<ArtistCredit>> {
    fn names(&self) -> impl Iterator<Item = String> {
        self.as_ref()
            .map(|credits| credits.names().collect::<Vec<String>>())
            .unwrap_or_default()
            .into_iter()
    }
}
