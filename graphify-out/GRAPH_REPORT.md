# Graph Report - .  (2026-08-21)

## Corpus Check
- Corpus is ~42,699 words - fits in a single context window. You may not need a graph.

## Summary
- 780 nodes · 1646 edges · 43 communities (36 shown, 7 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 40 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- Snoop UI
- Disc Metadata
- UPnP Device Map
- UPnP Devices
- Audio Frame Processing
- UPnP Notification
- Hex Dumping
- UPnP Search
- URI Parsing
- Error Handling
- MusicBrainz Integration
- String Parsing
- SSDP Headers
- UPnP Listener
- UPnP MSearch
- CDA File Handling
- CD Drive
- UPnP Types
- Windows CDROM TOC
- UUID Handling
- UPnP Response
- HTTP Headers
- Audio CD
- UPnP Headers
- Snoop Main
- Track Reading
- Error Types
- Lenient Parsing
- CLI Errors
- Redbook Main
- Error Conversion
- Redbook CLI
- Duration Handling
- Port Configuration
- Rip CLI
- Redbook Build
- Tag CLI
- Snoop Build
- Command Parsing
- TOC Build
- Crate Metadata
- Redbook Library

## God Nodes (most connected - your core abstractions)
1. `Disc` - 31 edges
2. `Frame` - 29 edges
3. `validate_root_device()` - 25 edges
4. `ST` - 25 edges
5. `UpnpHeader` - 24 edges
6. `NT` - 23 edges
7. `CdDrive` - 21 edges
8. `RootDevice` - 21 edges
9. `ParseError` - 20 edges
10. `Header` - 20 edges

## Surprising Connections (you probably didn't know these)
- `create_toc()` --calls--> `hex_to_bytes()`  [INFERRED]
  crates/redbook/src/disc.rs → crates/redbook/src/hex.rs
- `create_toc()` --calls--> `parse_toc()`  [INFERRED]
  crates/redbook/src/disc.rs → crates/redbook/src/hex.rs
- `load_toc()` --calls--> `hex_to_bytes()`  [INFERRED]
  crates/redbook/src/win.rs → crates/redbook/src/hex.rs
- `Disc` --references--> `Frame`  [EXTRACTED]
  crates/redbook/src/disc.rs → crates/redbook/src/lib.rs
- `AudioCd` --references--> `Disc`  [EXTRACTED]
  crates/redbook/src/win.rs → crates/redbook/src/disc.rs

## Import Cycles
- 1-file cycle: `crates/redbook/src/musicbrainz.rs -> crates/redbook/src/musicbrainz.rs`
- 1-file cycle: `crates/upnp2/src/message/header.rs -> crates/upnp2/src/message/header.rs`

## Communities (43 total, 7 thin omitted)

### Community 0 - "Snoop UI"
Cohesion: 0.05
Nodes (66): Buffer, CompletedFrame, DeviceLines<'d>, DeviceListing, ErrorListing, FocusHolder, HandleEvent, hide_embedded_device_if_not_expanded() (+58 more)

### Community 1 - "Disc Metadata"
Cohesion: 0.06
Nodes (42): create_disc(), create_toc(), create_tracks(), Disc, DiscError, new(), Display, Error (+34 more)

### Community 2 - "UPnP Device Map"
Cohesion: 0.07
Nodes (34): DeviceLines, Uuid, ControlPoint, Option, UserAgent, Uuid, Vec, DeviceMap (+26 more)

### Community 3 - "UPnP Devices"
Cohesion: 0.06
Nodes (27): Device, DeviceDetails, Display, Formatter, P, Result, Self, String (+19 more)

### Community 4 - "Audio Frame Processing"
Cohesion: 0.09
Nodes (23): Add, Duration, Frame, leadin_compensation(), Msf, RippedTrack, Duration, From (+15 more)

### Community 5 - "UPnP Notification"
Cohesion: 0.12
Nodes (22): ConfigId, HashMap, UpnpHeader, Alive, ByeBye, Notify, NT, NTS (+14 more)

### Community 6 - "Hex Dumping"
Cohesion: 0.08
Nodes (22): hex_dump(), hex_dump_roundtrip(), hex_to_bytes(), hex_to_bytes_bad_char(), hex_to_bytes_bad_length(), HexErrorKind, parse_toc(), ParseHexError (+14 more)

### Community 7 - "UPnP Search"
Cohesion: 0.17
Nodes (13): custom_build(), new_searcher(), Duration, Ipv4Addr, Option, Result, Self, String (+5 more)

### Community 8 - "URI Parsing"
Cohesion: 0.10
Nodes (15): Box, Display, Err, Formatter, FromStr, Option, P, Result (+7 more)

### Community 9 - "Error Handling"
Cohesion: 0.13
Nodes (16): AddrParseError, Error, From, Option, Self, ErrorKind, ParseError, Box (+8 more)

### Community 10 - "MusicBrainz Integration"
Cohesion: 0.17
Nodes (14): ArtistCreditsExt, Option<T>, Release, ReleaseExt, ReleaseScript, ReleaseStatus, Item, Iterator (+6 more)

### Community 11 - "String Parsing"
Cohesion: 0.23
Nodes (4): Err, Error, Result, Self

### Community 12 - "SSDP Headers"
Cohesion: 0.14
Nodes (11): FriendlyName, Host, ProductTokens, ProductTokens<_FLD>, Default, Display, Formatter, FromStr (+3 more)

### Community 13 - "UPnP Listener"
Cohesion: 0.13
Nodes (12): Context, Listener, Ipv4Addr, Item, Option, Result, Self, MAX_MSG_SIZE (+4 more)

### Community 14 - "UPnP MSearch"
Cohesion: 0.24
Nodes (11): MSearch, MulticastSearch, Display, Error, Formatter, Option, Result, Self (+3 more)

### Community 15 - "CDA File Handling"
Cohesion: 0.19
Nodes (11): audio(), CdaFile, Error, From, P, Self, TryFrom, Vec (+3 more)

### Community 16 - "CD Drive"
Cohesion: 0.16
Nodes (10): CdDrive, Drop, Formatter, PartialEq, PathBuf, String, Debug, Eq (+2 more)

### Community 17 - "UPnP Types"
Cohesion: 0.24
Nodes (10): BootId, Header, Lenient<H>, NextBootId, ProductTokens<FIELD_NAME>, PartialEq, u32, UpnpV2 (+2 more)

### Community 18 - "Windows CDROM TOC"
Cohesion: 0.19
Nodes (9): CDROM_TOC, CdromTocExt, leadout(), load_toc(), Item, Iterator, Toc, TocEntry (+1 more)

### Community 19 - "UUID Handling"
Cohesion: 0.19
Nodes (8): ControlPointUuid, Man, Mx, TryFrom, Uri, Uuid, ST, Uri

### Community 20 - "UPnP Response"
Cohesion: 0.16
Nodes (10): Response, DateTime, Display, Error, Formatter, Option, Result, Self (+2 more)

### Community 21 - "HTTP Headers"
Cohesion: 0.24
Nodes (7): get_option_invalid(), get_option_none(), get_option_some(), HeaderEntry, secure_location(), secure_location_invalid_scheme(), secure_location_no_port()

### Community 22 - "Audio CD"
Cohesion: 0.29
Nodes (6): AudioCdExt, AudioCdExtMut, AudioCd, ReadOnlyAudioCd, Arc, Send

### Community 23 - "UPnP Headers"
Cohesion: 0.22
Nodes (5): H, HeaderExt, Option<H>, String, UpnpV2Ext

### Community 24 - "Snoop Main"
Cohesion: 0.25
Nodes (6): Exit, main(), Exit, String, T, SelectedTrack

### Community 25 - "Track Reading"
Cohesion: 0.39
Nodes (3): Result, Track, Sector

### Community 26 - "Error Types"
Cohesion: 0.39
Nodes (7): Exit<T>, Error, From, Self, RecvError, SendError, SpawnError

### Community 27 - "Lenient Parsing"
Cohesion: 0.32
Nodes (4): Lenient<T>, T, UpnpHeader<'h>, FromIterator

### Community 28 - "CLI Errors"
Cohesion: 0.43
Nodes (6): ClapError, Exit<T>, Error, From, Infallible, Self

### Community 29 - "Redbook Main"
Cohesion: 0.33
Nodes (5): Exit, main(), Exit, String, T

### Community 30 - "Error Conversion"
Cohesion: 0.47
Nodes (5): Exit<T>, Error, From, Infallible, Self

### Community 31 - "Redbook CLI"
Cohesion: 0.33
Nodes (5): Exit, main(), Exit, String, T

### Community 32 - "Duration Handling"
Cohesion: 0.53
Nodes (3): Duration, MaxAge, Duration

### Community 33 - "Port Configuration"
Cohesion: 0.53
Nodes (4): From, Option, u16, UpnpPort

### Community 34 - "Rip CLI"
Cohesion: 0.50
Nodes (3): Rip, Option, String

## Knowledge Gaps
- **10 isolated node(s):** `redbook`, `SelectedTrack`, `TocEntry`, `snoop`, `Ui<CrosstermBackend<io::Stdout>>` (+5 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **7 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Disc` connect `Disc Metadata` to `Audio Frame Processing`, `Audio CD`?**
  _High betweenness centrality (0.054) - this node is a cross-community bridge._
- **Why does `Lenient` connect `UPnP Device Map` to `Snoop UI`, `UPnP Notification`, `URI Parsing`, `SSDP Headers`, `UUID Handling`, `HTTP Headers`, `UPnP Headers`, `Lenient Parsing`?**
  _High betweenness centrality (0.051) - this node is a cross-community bridge._
- **Why does `Mx` connect `UUID Handling` to `UPnP Devices`, `UPnP Search`, `String Parsing`, `SSDP Headers`, `UPnP MSearch`, `UPnP Types`, `HTTP Headers`?**
  _High betweenness centrality (0.047) - this node is a cross-community bridge._
- **What connects `redbook`, `SelectedTrack`, `TocEntry` to the rest of the system?**
  _10 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Snoop UI` be split into smaller, more focused modules?**
  _Cohesion score 0.05209274314965372 - nodes in this community are weakly interconnected._
- **Should `Disc Metadata` be split into smaller, more focused modules?**
  _Cohesion score 0.06201923076923077 - nodes in this community are weakly interconnected._
- **Should `UPnP Device Map` be split into smaller, more focused modules?**
  _Cohesion score 0.07294117647058823 - nodes in this community are weakly interconnected._