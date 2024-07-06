use rspotify::model::{
    FullTrack, PlayContextId, SearchType, SimplifiedAlbum, SimplifiedPlaylist,
};
use rspotify::prelude::*;

pub fn format_search_type(st: &SearchType) -> &str {
    match st {
        SearchType::Artist => "artists",
        SearchType::Track => "songs",
        SearchType::Album => "albums",
        SearchType::Show => "shows",
        SearchType::Episode => "episodes",
        SearchType::Playlist => "playlists",
    }
}

/// Either a single Spotify item (track) or a conceptual
/// collection of items (playlist, album).
#[derive(Clone)]
pub enum SpotItem {
    Track(FullTrack),
    Album(SimplifiedAlbum),
    Playlist(SimplifiedPlaylist),
    UserPlaylists,
    SearchResults { st: SearchType, query: String },
}

impl SpotItem {
    /// Primary display name for the item.
    pub fn name(&self) -> String {
        match self {
            SpotItem::Track(ft) => ft.name.clone(),
            SpotItem::Album(a) => a.name.clone(),
            SpotItem::Playlist(sp) => sp.name.clone(),
            SpotItem::UserPlaylists => "Saved playlists".to_string(),
            SpotItem::SearchResults { st, ref query } => {
                format!("{}s matching {}", format_search_type(st), query)
            }
        }
    }

    /// Name of the artist (or the owner user for playlists).
    pub fn artist(&self) -> String {
        match self {
            SpotItem::Track(ft) => ft
                .artists
                .iter()
                .map(|a| a.name.clone())
                .collect::<Vec<String>>()
                .join(", "),
            SpotItem::Album(a) => a
                .artists
                .iter()
                .map(|a| a.name.clone())
                .collect::<Vec<String>>()
                .join(", "),
            SpotItem::Playlist(sp) => sp.owner.display_name.clone().unwrap_or("".to_string()),
            SpotItem::UserPlaylists => "You".to_string(),
            SpotItem::SearchResults { .. } => "".to_string(),
        }
    }

    /// Returns a Spotify URI, like "spotify:album:XXX".
    pub fn uri(&self) -> Option<String> {
        match self {
            SpotItem::Track(ft) => ft.id.as_ref().map(|id| id.uri()),
            SpotItem::Album(a) => a.id.as_ref().map(|id| id.uri()),
            SpotItem::Playlist(sp) => Some(sp.id.uri()),
            SpotItem::UserPlaylists => None,
            SpotItem::SearchResults { .. } => None,
        }
    }

    /// Returns an URL for the item or collection.
    pub fn href(&self) -> Option<&str> {
        match self {
            SpotItem::Track(ft) => ft.href.as_deref(),
            SpotItem::Album(a) => a.href.as_deref(),
            SpotItem::Playlist(sp) => Some(&sp.href),
            SpotItem::UserPlaylists => None,
            SpotItem::SearchResults { .. } => None,
        }
    }

    /// Returns an URL to load the image from.
    pub fn img_url(&self) -> Option<&str> {
        match self {
            SpotItem::Track(ft) => &ft.album.images,
            SpotItem::Album(a) => &a.images,
            SpotItem::Playlist(sp) => &sp.images,
            SpotItem::UserPlaylists => return None,
            SpotItem::SearchResults { .. } => return None,
        }
        .first()
        .as_ref()
        .map(|img| img.url.as_str())
    }

    /// Optional PlayContextId for the collection (for playlists and albums).
    /// None if the collection cannot be played directly.
    pub fn context_id(&self) -> Option<PlayContextId<'_>> {
        match self {
            SpotItem::Track(_) => None,
            SpotItem::Album(a) => a.id.clone().map(PlayContextId::Album),
            SpotItem::Playlist(sp) => Some(PlayContextId::Playlist(sp.id.clone())),
            SpotItem::UserPlaylists => None,
            SpotItem::SearchResults { .. } => None,
        }
    }
}

impl std::fmt::Debug for SpotItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.href().unwrap_or("unlinkable item"))
    }
}
