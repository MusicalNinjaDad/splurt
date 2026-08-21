use metaflac::block::VorbisComment;
use musicbrainz_rs::entity::release::{ReleaseScript, ReleaseStatus};
pub use musicbrainz_rs::entity::{
    artist_credit::ArtistCredit,
    discid::Discid,
    release::{Release, Track},
};

pub trait VorbisTagExt {
    const KEY: &'static str;
    fn extend_vorbis(&self, vorbis: &mut VorbisComment);
}

impl VorbisTagExt for ReleaseStatus {
    const KEY: &'static str = "RELEASESTATUS";

    fn extend_vorbis(&self, vorbis: &mut VorbisComment) {
        let value = match self {
            ReleaseStatus::Official => "Official",
            ReleaseStatus::Promotion => "Promotion",
            ReleaseStatus::Bootleg => "Bootleg",
            ReleaseStatus::PseudoRelease => "Pseudo-Release",
            ReleaseStatus::UnrecognizedReleaseStatus => "Other",
            _ => "Other",
        };
        vorbis.set(Self::KEY, vec![value]);
    }
}

impl VorbisTagExt for ReleaseScript {
    const KEY: &'static str = "SCRIPT";

    fn extend_vorbis(&self, vorbis: &mut VorbisComment) {
        vorbis.set(Self::KEY, vec![self.code()])
    }
}

pub trait ArtistCreditsExt {
    fn names(&self) -> impl Iterator<Item = String>;
    fn artist_ids(&self) -> impl Iterator<Item = String>;
}

impl ArtistCreditsExt for Track {
    fn names(&self) -> impl Iterator<Item = String> {
        self.artist_credit.names()
    }

    fn artist_ids(&self) -> impl Iterator<Item = String> {
        self.artist_credit.artist_ids()
    }
}

impl ArtistCreditsExt for Release {
    fn names(&self) -> impl Iterator<Item = String> {
        self.artist_credit.names()
    }

    fn artist_ids(&self) -> impl Iterator<Item = String> {
        self.artist_credit.artist_ids()
    }
}

impl ArtistCreditsExt for Vec<ArtistCredit> {
    fn names(&self) -> impl Iterator<Item = String> {
        self.iter().map(|credit| credit.name.clone())
    }

    fn artist_ids(&self) -> impl Iterator<Item = String> {
        self.iter().map(|credit| credit.artist.id.clone())
    }
}

impl ArtistCreditsExt for &Vec<ArtistCredit> {
    fn names(&self) -> impl Iterator<Item = String> {
        self.iter().map(|credit| credit.name.clone())
    }

    fn artist_ids(&self) -> impl Iterator<Item = String> {
        self.iter().map(|credit| credit.artist.id.clone())
    }
}

impl<T: ArtistCreditsExt> ArtistCreditsExt for Option<T> {
    fn names(&self) -> impl Iterator<Item = String> {
        self.as_ref()
            .map(|credits| credits.names().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
    }

    fn artist_ids(&self) -> impl Iterator<Item = String> {
        self.as_ref()
            .map(|credits| credits.artist_ids().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
    }
}

pub trait ReleaseExt {
    fn track(&self, track_number: usize) -> Option<&Track>;
}

impl ReleaseExt for Release {
    fn track(&self, track_number: usize) -> Option<&Track> {
        self.media
            .as_ref()
            .and_then(|all_media| all_media.first())
            .and_then(|media| media.tracks.as_ref())
            .and_then(|tracks| {
                tracks
                    .iter()
                    .find(|trk| trk.number.parse() == Ok(track_number))
            })
    }
}
