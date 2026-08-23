//! Test fixtures for album data

use std::fmt::Display;
use std::path::PathBuf;

use crate::{Frame, Msf, TocEntry, Track, win::CdromTocExt};
use windows_sys::Win32::Devices::Cdrom::CDROM_TOC;

/// Test album identifier for parameterized tests
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestAlbum {
    DefinitelyMaybe,
    TheWallDisc1,
    TheWallDisc2,
}

impl Display for TestAlbum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestAlbum::DefinitelyMaybe => write!(f, "DefinitelyMaybe"),
            TestAlbum::TheWallDisc1 => write!(f, "TheWallDisc1"),
            TestAlbum::TheWallDisc2 => write!(f, "TheWallDisc2"),
        }
    }
}

impl TestAlbum {
    /// Path to the CDROM_TOC.hex file for this album
    pub fn cdrom_toc_path(&self) -> PathBuf {
        match self {
            TestAlbum::DefinitelyMaybe => {
                PathBuf::from("tests/assets/definitely_maybe/CDROM_TOC.hex")
            }
            TestAlbum::TheWallDisc1 => PathBuf::from("tests/assets/the_wall/disc1/CDROM_TOC.hex"),
            TestAlbum::TheWallDisc2 => PathBuf::from("tests/assets/the_wall/disc2/CDROM_TOC.hex"),
        }
    }

    /// Path to the TOC.hex file for this album
    pub fn toc_path(&self) -> PathBuf {
        match self {
            TestAlbum::DefinitelyMaybe => PathBuf::from("tests/assets/definitely_maybe/TOC.hex"),
            TestAlbum::TheWallDisc1 => PathBuf::from("tests/assets/the_wall/disc1/TOC.hex"),
            TestAlbum::TheWallDisc2 => PathBuf::from("tests/assets/the_wall/disc2/TOC.hex"),
        }
    }

    /// Path to the assets directory for this album
    pub fn assets_path(&self) -> PathBuf {
        match self {
            TestAlbum::DefinitelyMaybe => PathBuf::from("tests/assets/definitely_maybe"),
            TestAlbum::TheWallDisc1 => PathBuf::from("tests/assets/the_wall/disc1"),
            TestAlbum::TheWallDisc2 => PathBuf::from("tests/assets/the_wall/disc2"),
        }
    }

    /// Load and parse the TOC.hex file
    pub fn expected_toc(&self) -> cdtoc::Toc {
        let path = self.toc_path();
        let toc_dump = super::load_hex_file(&path);
        let toc_string = crate::hex::parse_toc(toc_dump);
        cdtoc::Toc::from_cdtoc(toc_string).unwrap()
    }

    /// Load and parse the CDROM_TOC.hex file
    pub fn load_cdrom_toc(&self) -> CDROM_TOC {
        let path = self.cdrom_toc_path();
        let toc_dump = super::load_hex_file(&path);
        #[allow(unsafe_code)]
        unsafe {
            CDROM_TOC::from_raw_bytes(toc_dump)
        }
    }

    /// Expected first track number for this album
    pub fn expected_first_track(&self) -> u8 {
        match self {
            TestAlbum::DefinitelyMaybe => 1,
            TestAlbum::TheWallDisc1 => 1,
            TestAlbum::TheWallDisc2 => 1,
        }
    }

    /// Expected last track number for this album
    pub fn expected_last_track(&self) -> u8 {
        match self {
            TestAlbum::DefinitelyMaybe => 11,
            TestAlbum::TheWallDisc1 => 13,
            TestAlbum::TheWallDisc2 => 13,
        }
    }

    /// Expected leadout frame for this album
    pub fn expected_leadout(&self) -> Frame {
        match self {
            TestAlbum::DefinitelyMaybe => Frame::from(Msf::new(0x34, 0x05, 0x1c)),
            TestAlbum::TheWallDisc1 => Frame::from(Msf::new(0x27, 0x0E, 0x0A)),
            TestAlbum::TheWallDisc2 => Frame::from(Msf::new(0x29, 0x3a, 0x19)),
        }
    }

    /// Expected audio tracks for this album with track names
    pub fn expected_audio_tracks(&self) -> Vec<(TocEntry, &'static str)> {
        match self {
            TestAlbum::DefinitelyMaybe => vec![
                (
                    TocEntry {
                        track: 1,
                        start: Frame::from(Msf::new(0x00, 0x02, 0x21)),
                    },
                    "Rock 'n' Roll Star",
                ),
                (
                    TocEntry {
                        track: 2,
                        start: Frame::from(Msf::new(0x05, 0x19, 0x32)),
                    },
                    "Shakermaker",
                ),
                (
                    TocEntry {
                        track: 3,
                        start: Frame::from(Msf::new(0x0A, 0x22, 0x0D)),
                    },
                    "Live Forever",
                ),
                (
                    TocEntry {
                        track: 4,
                        start: Frame::from(Msf::new(0x0F, 0x0B, 0x00)),
                    },
                    "Up in the Sky",
                ),
                (
                    TocEntry {
                        track: 5,
                        start: Frame::from(Msf::new(0x13, 0x27, 0x44)),
                    },
                    "Columbia",
                ),
                (
                    TocEntry {
                        track: 6,
                        start: Frame::from(Msf::new(0x19, 0x38, 0x41)),
                    },
                    "Supersonic",
                ),
                (
                    TocEntry {
                        track: 7,
                        start: Frame::from(Msf::new(0x1E, 0x28, 0x2D)),
                    },
                    "Bring It On Down",
                ),
                (
                    TocEntry {
                        track: 8,
                        start: Frame::from(Msf::new(0x22, 0x3A, 0x21)),
                    },
                    "Cigarettes & Alcohol",
                ),
                (
                    TocEntry {
                        track: 9,
                        start: Frame::from(Msf::new(0x27, 0x2F, 0x3A)),
                    },
                    "Digsy's Dinner",
                ),
                (
                    TocEntry {
                        track: 10,
                        start: Frame::from(Msf::new(0x2A, 0x14, 0x08)),
                    },
                    "Slide Away",
                ),
                (
                    TocEntry {
                        track: 11,
                        start: Frame::from(Msf::new(0x30, 0x34, 0x3F)),
                    },
                    "Married With Children",
                ),
            ],
            TestAlbum::TheWallDisc1 => vec![
                (
                    TocEntry {
                        track: 1,
                        start: Frame::from(Msf::new(0x00, 0x02, 0x00)),
                    },
                    "In the Flesh?",
                ),
                (
                    TocEntry {
                        track: 2,
                        start: Frame::from(Msf::new(0x03, 0x15, 0x2A)),
                    },
                    "The Thin Ice",
                ),
                (
                    TocEntry {
                        track: 3,
                        start: Frame::from(Msf::new(0x05, 0x33, 0x20)),
                    },
                    "Another Brick in the Wall, Part 1",
                ),
                (
                    TocEntry {
                        track: 4,
                        start: Frame::from(Msf::new(0x09, 0x01, 0x1E)),
                    },
                    "The Happiest Days of Our Lives",
                ),
                (
                    TocEntry {
                        track: 5,
                        start: Frame::from(Msf::new(0x0A, 0x33, 0x43)),
                    },
                    "Another Brick in the Wall, Part 2",
                ),
                (
                    TocEntry {
                        track: 6,
                        start: Frame::from(Msf::new(0x0E, 0x33, 0x1B)),
                    },
                    "Mother",
                ),
                (
                    TocEntry {
                        track: 7,
                        start: Frame::from(Msf::new(0x14, 0x19, 0x11)),
                    },
                    "Goodbye Blue Sky",
                ),
                (
                    TocEntry {
                        track: 8,
                        start: Frame::from(Msf::new(0x17, 0x0C, 0x3C)),
                    },
                    "Empty Spaces",
                ),
                (
                    TocEntry {
                        track: 9,
                        start: Frame::from(Msf::new(0x19, 0x15, 0x0A)),
                    },
                    "Young Lust",
                ),
                (
                    TocEntry {
                        track: 10,
                        start: Frame::from(Msf::new(0x1C, 0x34, 0x11)),
                    },
                    "One of My Turns",
                ),
                (
                    TocEntry {
                        track: 11,
                        start: Frame::from(Msf::new(0x20, 0x1C, 0x39)),
                    },
                    "Don't Leave Me Now",
                ),
                (
                    TocEntry {
                        track: 12,
                        start: Frame::from(Msf::new(0x24, 0x2D, 0x0C)),
                    },
                    "Another Brick in the Wall, Part 3",
                ),
                (
                    TocEntry {
                        track: 13,
                        start: Frame::from(Msf::new(0x25, 0x3B, 0x41)),
                    },
                    "Goodbye Cruel World",
                ),
            ],
            TestAlbum::TheWallDisc2 => vec![
                (
                    TocEntry {
                        track: 1,
                        start: Frame::from(Msf::new(0x00, 0x02, 0x00)),
                    },
                    "Hey You",
                ),
                (
                    TocEntry {
                        track: 2,
                        start: Frame::from(Msf::new(0x04, 0x2B, 0x28)),
                    },
                    "Is There Anybody Out There?",
                ),
                (
                    TocEntry {
                        track: 3,
                        start: Frame::from(Msf::new(0x07, 0x17, 0x3C)),
                    },
                    "Nobody Home",
                ),
                (
                    TocEntry {
                        track: 4,
                        start: Frame::from(Msf::new(0x0A, 0x30, 0x19)),
                    },
                    "Vera",
                ),
                (
                    TocEntry {
                        track: 5,
                        start: Frame::from(Msf::new(0x0C, 0x15, 0x1E)),
                    },
                    "Bring the Boys Back Home",
                ),
                (
                    TocEntry {
                        track: 6,
                        start: Frame::from(Msf::new(0x0D, 0x30, 0x2A)),
                    },
                    "Comfortably Numb",
                ),
                (
                    TocEntry {
                        track: 7,
                        start: Frame::from(Msf::new(0x14, 0x0A, 0x14)),
                    },
                    "The Show Must Go On",
                ),
                (
                    TocEntry {
                        track: 8,
                        start: Frame::from(Msf::new(0x15, 0x2E, 0x1E)),
                    },
                    "In the Flesh",
                ),
                (
                    TocEntry {
                        track: 9,
                        start: Frame::from(Msf::new(0x1A, 0x03, 0x0A)),
                    },
                    "Run Like Hell",
                ),
                (
                    TocEntry {
                        track: 10,
                        start: Frame::from(Msf::new(0x1E, 0x1A, 0x41)),
                    },
                    "Waiting for the Worms",
                ),
                (
                    TocEntry {
                        track: 11,
                        start: Frame::from(Msf::new(0x22, 0x19, 0x11)),
                    },
                    "Stop",
                ),
                (
                    TocEntry {
                        track: 12,
                        start: Frame::from(Msf::new(0x22, 0x37, 0x23)),
                    },
                    "The Trial",
                ),
                (
                    TocEntry {
                        track: 13,
                        start: Frame::from(Msf::new(0x28, 0x0F, 0x0F)),
                    },
                    "Outside the Wall",
                ),
            ],
        }
    }

    /// Get just the TocEntry values for comparison with iter_audio()
    pub fn expected_toc_entries(&self) -> Vec<TocEntry> {
        self.expected_audio_tracks()
            .into_iter()
            .map(|(entry, _)| entry)
            .collect()
    }

    /// Expected tracks as Track objects (without metadata)
    pub fn expected_tracks(&self) -> Vec<Track<'static>> {
        match self {
            TestAlbum::DefinitelyMaybe => vec![
                Track {
                    toc_entry: TocEntry {
                        track: 1,
                        start: Frame::from(Msf::new(0x00, 0x02, 0x21)),
                    },
                    duration_frames: Frame::new(24242),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 2,
                        start: Frame::from(Msf::new(0x05, 0x19, 0x32)),
                    },
                    duration_frames: Frame::new(23138),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 3,
                        start: Frame::from(Msf::new(0x0A, 0x22, 0x0D)),
                    },
                    duration_frames: Frame::new(20762),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 4,
                        start: Frame::from(Msf::new(0x0F, 0x0B, 0x00)),
                    },
                    duration_frames: Frame::new(20168),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 5,
                        start: Frame::from(Msf::new(0x13, 0x27, 0x44)),
                    },
                    duration_frames: Frame::new(28272),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 6,
                        start: Frame::from(Msf::new(0x19, 0x38, 0x41)),
                    },
                    duration_frames: Frame::new(21280),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 7,
                        start: Frame::from(Msf::new(0x1E, 0x28, 0x2D)),
                    },
                    duration_frames: Frame::new(19338),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 8,
                        start: Frame::from(Msf::new(0x22, 0x3A, 0x21)),
                    },
                    duration_frames: Frame::new(21700),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 9,
                        start: Frame::from(Msf::new(0x27, 0x2F, 0x3A)),
                    },
                    duration_frames: Frame::new(11425),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 10,
                        start: Frame::from(Msf::new(0x2A, 0x14, 0x08)),
                    },
                    duration_frames: Frame::new(29455),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 11,
                        start: Frame::from(Msf::new(0x30, 0x34, 0x3F)),
                    },
                    duration_frames: Frame::new(14440),
                    ..Default::default()
                },
            ],
            TestAlbum::TheWallDisc1 => vec![
                Track {
                    toc_entry: TocEntry {
                        track: 1,
                        start: Frame::from(Msf::new(0x00, 0x02, 0x00)),
                    },
                    duration_frames: Frame::new(14967),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 2,
                        start: Frame::from(Msf::new(0x03, 0x15, 0x2A)),
                    },
                    duration_frames: Frame::new(11240),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 3,
                        start: Frame::from(Msf::new(0x05, 0x33, 0x20)),
                    },
                    duration_frames: Frame::new(14248),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 4,
                        start: Frame::from(Msf::new(0x09, 0x01, 0x1E)),
                    },
                    duration_frames: Frame::new(8287),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 5,
                        start: Frame::from(Msf::new(0x0A, 0x33, 0x43)),
                    },
                    duration_frames: Frame::new(17960),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 6,
                        start: Frame::from(Msf::new(0x0E, 0x33, 0x1B)),
                    },
                    duration_frames: Frame::new(25040),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 7,
                        start: Frame::from(Msf::new(0x14, 0x19, 0x11)),
                    },
                    duration_frames: Frame::new(12568),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 8,
                        start: Frame::from(Msf::new(0x17, 0x0C, 0x3C)),
                    },
                    duration_frames: Frame::new(9625),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 9,
                        start: Frame::from(Msf::new(0x19, 0x15, 0x0A)),
                    },
                    duration_frames: Frame::new(15832),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 10,
                        start: Frame::from(Msf::new(0x1C, 0x34, 0x11)),
                    },
                    duration_frames: Frame::new(16240),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 11,
                        start: Frame::from(Msf::new(0x20, 0x1C, 0x39)),
                    },
                    duration_frames: Frame::new(19230),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 12,
                        start: Frame::from(Msf::new(0x24, 0x2D, 0x0C)),
                    },
                    duration_frames: Frame::new(5603),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 13,
                        start: Frame::from(Msf::new(0x25, 0x3B, 0x41)),
                    },
                    duration_frames: Frame::new(5570),
                    ..Default::default()
                },
            ],
            TestAlbum::TheWallDisc2 => vec![
                Track {
                    toc_entry: TocEntry {
                        track: 1,
                        start: Frame::from(Msf::new(0x00, 0x02, 0x00)),
                    },
                    duration_frames: Frame::new(21115),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 2,
                        start: Frame::from(Msf::new(0x04, 0x2B, 0x28)),
                    },
                    duration_frames: Frame::new(12020),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 3,
                        start: Frame::from(Msf::new(0x07, 0x17, 0x3C)),
                    },
                    duration_frames: Frame::new(15340),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 4,
                        start: Frame::from(Msf::new(0x0A, 0x30, 0x19)),
                    },
                    duration_frames: Frame::new(6980),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 5,
                        start: Frame::from(Msf::new(0x0C, 0x15, 0x1E)),
                    },
                    duration_frames: Frame::new(6537),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 6,
                        start: Frame::from(Msf::new(0x0D, 0x30, 0x2A)),
                    },
                    duration_frames: Frame::new(28628),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 7,
                        start: Frame::from(Msf::new(0x14, 0x0A, 0x14)),
                    },
                    duration_frames: Frame::new(7210),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 8,
                        start: Frame::from(Msf::new(0x15, 0x2E, 0x1E)),
                    },
                    duration_frames: Frame::new(19255),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 9,
                        start: Frame::from(Msf::new(0x1A, 0x03, 0x0A)),
                    },
                    duration_frames: Frame::new(19780),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 10,
                        start: Frame::from(Msf::new(0x1E, 0x1A, 0x41)),
                    },
                    duration_frames: Frame::new(17877),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 11,
                        start: Frame::from(Msf::new(0x22, 0x19, 0x11)),
                    },
                    duration_frames: Frame::new(2268),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 12,
                        start: Frame::from(Msf::new(0x22, 0x37, 0x23)),
                    },
                    duration_frames: Frame::new(23980),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 13,
                        start: Frame::from(Msf::new(0x28, 0x0F, 0x0F)),
                    },
                    duration_frames: Frame::new(7735),
                    ..Default::default()
                },
            ],
        }
    }

    /// Load musicbrainz data from the musicbrainz.json file for this album
    pub fn expected_musicbrainz(&self) -> crate::musicbrainz::Discid {
        let path = self.assets_path().join("musicbrainz.json");
        let json_content = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("Failed to read musicbrainz.json from {:?}", path));
        serde_json::from_str(&json_content)
            .unwrap_or_else(|e| panic!("Failed to parse musicbrainz.json from {:?}: {}", path, e))
    }

    /// Load the expected release menu from release_selection.txt
    pub fn expected_release_menu(&self) -> Option<String> {
        let path = self.assets_path().join("release_selection.txt");
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                if content.trim().is_empty() {
                    None
                } else {
                    Some(content)
                }
            }
            Err(_) => None,
        }
    }

    /// The correct release number for the album
    pub fn release(&self) -> usize {
        match self {
            TestAlbum::DefinitelyMaybe => 2,
            TestAlbum::TheWallDisc1 => self
                .expected_musicbrainz()
                .releases
                .unwrap()
                .iter()
                .position(|release| release.id == "b13b64f6-85fc-3c1c-8aae-e5adb94d7181")
                .unwrap(),
            TestAlbum::TheWallDisc2 => self
                .expected_musicbrainz()
                .releases
                .unwrap()
                .iter()
                .position(|release| release.id == "b13b64f6-85fc-3c1c-8aae-e5adb94d7181")
                .unwrap(),
        }
    }

    /// The disc_index as it should be automatically identified.
    pub fn expected_disc_index(&self) -> Option<usize> {
        match self {
            TestAlbum::DefinitelyMaybe => Some(0),
            TestAlbum::TheWallDisc1 => self
                .expected_musicbrainz()
                .releases
                .unwrap()
                .get(self.release())
                .map(|release| {
                    release
                        .media
                        .as_ref()
                        .unwrap()
                        .iter()
                        .position(|media| media.position == Some(1))
                })
                .unwrap(),
            TestAlbum::TheWallDisc2 => self
                .expected_musicbrainz()
                .releases
                .unwrap()
                .get(self.release())
                .map(|release| {
                    release
                        .media
                        .as_ref()
                        .unwrap()
                        .iter()
                        .position(|media| media.position == Some(2))
                })
                .unwrap(),
        }
    }
}
