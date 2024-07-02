use rspotify::model::{FullTrack, PlayContextId, SimplifiedPlaylist};

/// A single item to be displayed in a list.
#[derive(Clone)]
pub enum SpotItem {
    Track(FullTrack),
    Playlist(SimplifiedPlaylist),
}

impl SpotItem {
    /// Primary display name for the item.
    pub fn name(&self) -> &str {
        match self {
            SpotItem::Track(ft) => &ft.name,
            SpotItem::Playlist(sp) => &sp.name,
        }
    }

    /// Returns an URL for the image.
    pub fn href(&self) -> Option<&str> {
        match self {
            SpotItem::Track(ft) => ft.href.as_deref(),
            SpotItem::Playlist(sp) => Some(&sp.href),
        }
    }

    pub fn img_url(&self) -> Option<&str> {
        match self {
            SpotItem::Track(ft) => &ft.album.images,
            SpotItem::Playlist(sp) => &sp.images,
        }
        .first()
        .as_ref()
        .map(|img| img.url.as_str())
    }
}

impl std::fmt::Debug for SpotItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.href().unwrap_or("unlinkable item"))
    }
}

/// A logical grouping of Spotify items.
pub enum SpotCollection {
    UserPlaylists,
    Playlist(SimplifiedPlaylist),
}

impl SpotCollection {
    /// The primary display name for the collection.
    pub fn name(&self) -> &str {
        match self {
            SpotCollection::UserPlaylists => "Saved playlists",
            SpotCollection::Playlist(sp) => &sp.name,
        }
    }
    /// Optional PlayContextId for the collection (for playlists and albums).
    /// None if the collection cannot be played directly.
    pub fn context_id(&self) -> Option<PlayContextId<'_>> {
        match self {
            SpotCollection::UserPlaylists => None,
            SpotCollection::Playlist(sp) => Some(PlayContextId::Playlist(sp.id.clone())),
        }
    }
}
