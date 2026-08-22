//! Test fixtures for album data

use std::fmt::Display;
use std::path::PathBuf;

use crate::{Frame, Msf, TocEntry, win::CdromTocExt};
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
}
