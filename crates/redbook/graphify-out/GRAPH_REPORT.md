# Graph Report - /workspaces/splurt/crates/redbook  (2026-08-21)

## Corpus Check
- Corpus is ~13,079 words - fits in a single context window. You may not need a graph.

## Summary
- 324 nodes · 541 edges · 22 communities (18 shown, 4 thin omitted)
- Extraction: 97% EXTRACTED · 3% INFERRED · 0% AMBIGUOUS · INFERRED: 15 edges (avg confidence: 0.88)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- Windows Audio CD
- Hex Parsing
- Tagging Operations
- Time Duration
- Disc Metadata
- Disc Creation
- MusicBrainz Integration
- CLI Error Handling
- TOC Track Types
- FLAC Encoding
- CLI Main Entry
- Error Types
- Image Picture Types
- Rip CLI
- Build Configuration
- Platform Abstraction
- AudioCd Trait
- External Dependencies
- Tag CLI
- Ripper
- Redbook Project

## God Nodes (most connected - your core abstractions)
1. `Disc` - 30 edges
2. `Frame` - 28 edges
3. `CdDrive` - 19 edges
4. `Msf` - 14 edges
5. `Redbook` - 13 edges
6. `create_disc()` - 12 edges
7. `ParseHexError` - 10 edges
8. `hex_to_bytes()` - 10 edges
9. `CDROM_TOC` - 9 edges
10. `new()` - 8 edges

## Surprising Connections (you probably didn't know these)
- `load_hex_file()` --calls--> `hex_to_bytes()`  [INFERRED]
  tests/parse_toc.rs → src/hex.rs
- `compare_definitely_maybe()` --calls--> `parse_toc()`  [INFERRED]
  tests/parse_toc.rs → src/hex.rs
- `create_toc()` --calls--> `hex_to_bytes()`  [INFERRED]
  src/disc.rs → src/hex.rs
- `create_toc()` --calls--> `parse_toc()`  [INFERRED]
  src/disc.rs → src/hex.rs
- `load_toc()` --calls--> `hex_to_bytes()`  [INFERRED]
  src/win.rs → src/hex.rs

## Import Cycles
- 1-file cycle: `src/lib.rs -> src/lib.rs`
- 1-file cycle: `src/musicbrainz.rs -> src/musicbrainz.rs`

## Hyperedges (group relationships)
- **Time manipulation components** — crates_redbook_todo_frameduration, crates_redbook_todo_track, crates_redbook_todo_starting_frame, crates_redbook_todo_msf, crates_redbook_todo_cdtime, crates_redbook_todo_duration [EXTRACTED 1.00]
- **Platform abstraction** — crates_redbook_todo_mod_linux, crates_redbook_todo_mod_win, crates_redbook_todo_api_surface, crates_redbook_todo_audiocd_trait, crates_redbook_todo_audiocd_win, crates_redbook_todo_audiocd_linux [INFERRED 0.95]
- **TOC and metadata handling** — crates_redbook_todo_toc, crates_redbook_todo_cdtoc, crates_redbook_todo_vec_tracks, crates_redbook_todo_online_directories, crates_redbook_todo_name_file, crates_redbook_todo_artwork [EXTRACTED 1.00]

## Communities (22 total, 4 thin omitted)

### Community 0 - "Windows Audio CD"
Cohesion: 0.06
Nodes (34): Debug, Drop, Eq, HANDLE, Path, PCWSTR, Send, audio() (+26 more)

### Community 1 - "Hex Parsing"
Cohesion: 0.08
Nodes (22): ParseIntError, hex_dump(), hex_dump_roundtrip(), hex_to_bytes(), hex_to_bytes_bad_char(), hex_to_bytes_bad_length(), HexErrorKind, parse_toc() (+14 more)

### Community 2 - "Tagging Operations"
Cohesion: 0.13
Nodes (17): Add, MemSink, N, Output, Rem, Duration, Frame, leadin_compensation() (+9 more)

### Community 3 - "Time Duration"
Cohesion: 0.06
Nodes (33): AI image recognition, artwork, as_relative_position, bytes Bytes, CdTime, cdtoc, clap, coverart description (+25 more)

### Community 4 - "Disc Metadata"
Cohesion: 0.11
Nodes (15): Discid, ExactSizeIterator, Disc, Option, P, PathBuf, Result, String (+7 more)

### Community 5 - "Disc Creation"
Cohesion: 0.11
Nodes (25): F, create_disc(), create_toc(), create_tracks(), DiscError, new(), Display, Error (+17 more)

### Community 6 - "MusicBrainz Integration"
Cohesion: 0.17
Nodes (14): ArtistCreditsExt, Option<T>, Release, ReleaseExt, ReleaseScript, ReleaseStatus, Item, Iterator (+6 more)

### Community 7 - "CLI Error Handling"
Cohesion: 0.18
Nodes (12): ClapError, Exit, Exit<T>, main(), Error, Exit, From, Infallible (+4 more)

### Community 8 - "TOC Track Types"
Cohesion: 0.36
Nodes (5): Option, String, TocEntry, Track, Track<'meta>

### Community 9 - "FLAC Encoding"
Cohesion: 0.25
Nodes (8): flac encoding, flacenc-rs, LANECOUNT, portable-simd, rust-lang/rust#151775, SUPPORTEDLANECOUNT, users.rust-lang.org thread 138089, yotarok/flacenc-rs

### Community 10 - "CLI Main Entry"
Cohesion: 0.33
Nodes (5): Exit, main(), Exit, String, T

### Community 11 - "Error Types"
Cohesion: 0.47
Nodes (5): Exit<T>, Error, From, Infallible, Self

### Community 12 - "Image Picture Types"
Cohesion: 0.40
Nodes (4): B, PictureType, S, Self

### Community 13 - "Rip CLI"
Cohesion: 0.50
Nodes (3): Rip, Option, String

### Community 15 - "Platform Abstraction"
Cohesion: 1.00
Nodes (3): API surface, mod linux, mod win

### Community 16 - "AudioCd Trait"
Cohesion: 0.67
Nodes (3): linux::AudioCd, trait AudioCd, win::AudioCd

### Community 17 - "External Dependencies"
Cohesion: 0.67
Nodes (3): commit b51ab9c31c45f11c2c86e884621d625e1bb786dc, musicbrainz_rs, ring

## Knowledge Gaps
- **36 isolated node(s):** `redbook`, `SelectedTrack`, `TocEntry`, `Ripper`, `starting_frame` (+31 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **4 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Disc` connect `Disc Metadata` to `Windows Audio CD`, `Tagging Operations`, `Disc Creation`?**
  _High betweenness centrality (0.280) - this node is a cross-community bridge._
- **Why does `Frame` connect `Tagging Operations` to `TOC Track Types`, `Windows Audio CD`, `Disc Metadata`, `Disc Creation`?**
  _High betweenness centrality (0.139) - this node is a cross-community bridge._
- **Why does `Release` connect `MusicBrainz Integration` to `Disc Metadata`?**
  _High betweenness centrality (0.102) - this node is a cross-community bridge._
- **What connects `redbook`, `SelectedTrack`, `TocEntry` to the rest of the system?**
  _36 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Windows Audio CD` be split into smaller, more focused modules?**
  _Cohesion score 0.06397306397306397 - nodes in this community are weakly interconnected._
- **Should `Hex Parsing` be split into smaller, more focused modules?**
  _Cohesion score 0.07692307692307693 - nodes in this community are weakly interconnected._
- **Should `Tagging Operations` be split into smaller, more focused modules?**
  _Cohesion score 0.13015873015873017 - nodes in this community are weakly interconnected._