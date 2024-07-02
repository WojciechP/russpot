use rspotify::model::{FullTrack, PlayContextId, SimplifiedPlaylist};
use rspotify::prelude::*;

/// Either a single Spotify item (track) or a conceptual
/// collection of items (playlist, album).
#[derive(Clone)]
pub enum SpotItem {
    Track(FullTrack),
    Playlist(SimplifiedPlaylist),
    UserPlaylists,
}

impl SpotItem {
    /// Primary display name for the item.
    pub fn name(&self) -> &str {
        match self {
            SpotItem::Track(ft) => &ft.name,
            SpotItem::Playlist(sp) => &sp.name,
            SpotItem::UserPlaylists => "Saved playlists",
        }
    }

    /// Returns a Spotify URI, like "spotify:album:XXX".
    pub fn uri(&self) -> Option<String> {
        match self {
            SpotItem::Track(ft) => ft.id.as_ref().map(|id| id.uri()),
            SpotItem::Playlist(sp) => Some(sp.id.uri()),
            SpotItem::UserPlaylists => None,
        }
    }

    /// Returns an URL for the item or collection.
    pub fn href(&self) -> Option<&str> {
        match self {
            SpotItem::Track(ft) => ft.href.as_deref(),
            SpotItem::Playlist(sp) => Some(&sp.href),
            SpotItem::UserPlaylists => None,
        }
    }

    /// Returns an URL to load the image from.
    pub fn img_url(&self) -> Option<&str> {
        match self {
            SpotItem::Track(ft) => &ft.album.images,
            SpotItem::Playlist(sp) => &sp.images,
            SpotItem::UserPlaylists => return None,
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
            SpotItem::Playlist(sp) => Some(PlayContextId::Playlist(sp.id.clone())),
            SpotItem::UserPlaylists => None,
        }
    }
}

impl std::fmt::Debug for SpotItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.href().unwrap_or("unlinkable item"))
    }
}
