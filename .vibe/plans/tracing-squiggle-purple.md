# Plan: Tracing Implementation for Redbook

## Summary

Implement **tracing** in the **redbook** crate (CDDA CD digital audio library, IEC 60908:1999) using the `tracing` ecosystem.

**Goal**: Enable flexible, multi-level observability for library consumers and CLI users without hardware/network access.

**Requirements**:

- **Library**: Emit spans/events only (add `tracing` dep, never init subscribers)
- **Tests first (TDD)**: Create tests for non-hardware/network cases (hex parsing, Disc creation, track lookups, Frame/Msf) using existing `tests/assets` fixtures
- **Levels**: `read_chunk` = trace, rip start/done = info with track name, ignored errors (HTTP 500) = warn with URI/status, album spans include album name
- **CLI flags**: `-v` (debug→stdout), `-vv` (trace→stdout), `-q` (no info→stdout), `-qq` (no info→stdout, no warn→stderr), `--debug LOGFILE`, `--trace LOGFILE`, `--json`
- **Style**: Prefer `#[tracing::instrument]` or explicit `Span::enter()`/`exit()` over closures; include track numbers where relevant

**Technical constraints**: Multi-threaded CLI (not async), existing `dbg!` calls to be replaced, `clap` for argument parsing, stdout/stderr/file routing via `tracing-subscriber` layers.

---

## Steps

AFTER EACH STEP COMMIT YOUR WORK before continuing. Run `git add . && git commit -m <meaningful commit message>` BEFORE moving on to the next step.

### Phase 0: Dependencies & Test Infrastructure

**0.1 Add tracing to library**

- Add `tracing = "0.1"` to `crates/redbook/Cargo.toml` [dependencies]
- Library only — **no** `tracing-subscriber` in library dependencies

**0.2 Add test dependencies**

- Add `tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }` as **dev-dependency**
- Add `test-log = { version = "0.2", features = ["tracing-subscriber"] }` or implement custom test helper

**0.3 Create test helper module**

- New file: `crates/redbook/tests/tracing.rs`
- Implements `TestTracing::new()` → returns `(Guard, Vec<u8>)` capturing all output
- Supports both text and JSON format verification
- Uses `tracing-subscriber::fmt::TestWriter` pattern

---

### Phase 1: Tests First (Non-Hardware/Network Cases)

**1.1 Hex module tests** (new file: `crates/redbook/tests/tracing_hex.rs`)

- Test `hex_to_bytes()` emits `trace` span with input length
- Test `hex_dump()` emits `trace` span with byte count
- Test `parse_toc()` emits `debug` span with TOC entry count
- Use existing fixtures: `TOC.hex`, `CDROM_TOC.hex`

**1.2 Disc module tests** (new file: `crates/redbook/tests/tracing_disc.rs`)

- Test `Disc::new()` emits `info` span with track count, album name (if available)
- Test `Disc::track(n)` emits `debug` span with `track_number`
- Test `Disc::tracks()` emits `debug` span with iterator count
- Test `Disc::set_release(index)` emits `debug` span with selected release index
- Use fixture: `9822581d-.../TOC.hex` + `musicbrainz_disc.json`

**1.3 Frame/Msf type tests** (extend `crates/redbook/src/lib.rs` tests)

- Test `Frame::from(Msf)` emits `trace` span with values
- Test `Msf::from(Frame)` emits `trace` span with values
- Test conversion round-trips

**1.4 Tagging tests** (new file: `crates/redbook/tests/tracing_tagging.rs`)

- Test `Disc::tag_for(n)` emits `debug` span with `track_number`, `title`
- Pre-load MusicBrainz data from `musicbrainz_disc.json` fixture

**Verification**: All tests compile but fail (no instrumentation yet). This is expected TDD state.

---

### Phase 2: Library Instrumentation (Pass Tests)

**2.1 Hex module** (`crates/redbook/src/hex.rs`)

- `#[tracing::instrument(level = "trace")]` on `hex_to_bytes()`, `hex_dump()`
- `#[tracing::instrument(level = "debug")]` on `parse_toc()`
- Add field: `len = hex.len()` or `bytes.len()`

**2.2 Disc module** (`crates/redbook/src/disc.rs`)

- `Disc::new()`: `#[tracing::instrument(level = "info", skip(toc, tracks))]` + `tracing::Span::record("track_count", tracks.len())`
- `Disc::track()`: `#[tracing::instrument(level = "debug", skip(self))]` + record `track_number`
- `Disc::tracks()`: `#[tracing::instrument(level = "debug", skip(self))]`
- `Disc::set_release()`: `#[tracing::instrument(level = "debug", skip(self))]` + record `index`
- `Disc::tag_for()`: `#[tracing::instrument(level = "debug", skip(self))]` + record `track_number`, `title` from result

**2.3 Frame/Msf types** (`crates/redbook/src/lib.rs`)

- Add `#[tracing::instrument(level = "trace")]` to `From<Msf> for Frame`, `From<Frame> for Msf`, etc.
- Or use explicit spans: `let _span = tracing::trace_span!("frame_conversion", from = ?msf, to = ?frames).entered();`

**2.4 Tagging module** (`crates/redbook/src/tagging.rs`)

- Instrument public functions with `debug` spans

**Verification**: All Phase 1 tests pass.

---
---

### Phase 3: CLI Integration (bin/rip)

**3.1 Add CLI verbosity flags** (`crates/redbook/src/bin/rip/cli.rs`)

```rust
#[derive(Clap)]
struct TracingArgs {
    #[arg(short = 'v', action = clap::ArgAction::Count)]
    verbose: u8,  // 0, 1, or 2

    #[arg(short = 'q', action = clap::ArgAction::Count)]
    quiet: u8,    // 0, 1, or 2

    #[arg(long, value_name = "LOGFILE")]
    debug: Option<PathBuf>,

    #[arg(long, value_name = "LOGFILE")]
    trace: Option<PathBuf>,

    #[arg(long)]
    json: bool,
}
```

**3.2 Implement stdout/stderr/file layering** (`crates/redbook/src/bin/rip/main.rs`)

- Initialize subscriber **before any other code** (including `exit_safely` if used)
- Logic matrix:

| Flags | stdout | stderr | file |
| ------- | -------- | -------- | ------ |
| (default) | INFO | WARN | none |
| `-v` | DEBUG | WARN | none |
| `-vv` | TRACE | WARN | none |
| `-q` | NONE | WARN | none |
| `-qq` | NONE | NONE | none |
| `--debug f` | per -v/-q | per -v/-q | DEBUG |
| `--trace f` | per -v/-q | per -v/-q | TRACE |
| `--json` | per -v/-q | per -v/-q | JSON format |

- Use `tracing_subscriber::fmt::Layer` with custom `MakeWriter` for stdout/stderr
- Use separate `fmt::Layer` or `json::Layer` for file output
- Combine with `tracing_subscriber::Registry`

**3.3 Example setup code**:

```rust
fn init_tracing(args: &Args) {
    use tracing_subscriber::{fmt, prelude::*, filter::LevelFilter, EnvFilter};

    let stdout_filter = match (args.verbose, args.quiet) {
        (0, 0) => LevelFilter::INFO,
        (1, _) => LevelFilter::DEBUG,
        (2.., _) => LevelFilter::TRACE,
        (_, 1..) => LevelFilter::OFF,   // -q / -qq
    };

    let stderr_filter = match args.quiet {
        0 => LevelFilter::WARN,
        _ => LevelFilter::OFF,        // -q or -qq
    };

    let (file_layer, file_filter) = if let Some(path) = args.debug.as_ref().or(args.trace.as_ref()) {
        let level = if args.debug.is_some() { LevelFilter::DEBUG } else { LevelFilter::TRACE };
        let writer = std::fs::File::create(path).expect("create log file");
        let layer = if args.json {
            fmt::layer().json().with_writer(writer)
        } else {
            fmt::layer().with_writer(writer)
        };
        (Some(layer), level)
    } else {
        (None, LevelFilter::OFF)
    };

    let registry = tracing_subscriber::registry()
        .with(fmt::layer().with_filter(stdout_filter))
        .with(fmt::layer().with_filter(stderr_filter).with_writer(std::io::stderr))
        .with(file_layer.unwrap_or_else(|| fmt::layer().with_filter(LevelFilter::OFF)));

    if let Some(path) = args.debug.as_ref().or(args.trace.as_ref()) {
        registry.with(file_layer.unwrap()).init();
    } else {
        registry.init();
    }
}
```

---
---

### Phase 4: Complete Library Instrumentation

**4.1 Windows CD access** (`crates/redbook/src/win.rs`)

- `CdDrive::open()`: `#[tracing::instrument(level = "info")]` + record `path`
- `CdDrive::read_chunk()`: `#[tracing::instrument(level = "trace", skip(self, buf))]` + record `track.toc_entry.track`, `frame_offset`, `frames_to_read`
- `CdDrive::toc()`: `#[tracing::instrument(level = "debug")]`
- `AudioCd::read_track()`: `#[tracing::instrument(level = "info", skip(self))]` + record `track_number`, `track.title()`
- `AudioCd::rip()`: `#[tracing::instrument(level = "info", skip(self))]` + record `track_number`, `track.title()`

**4.2 Rip operations** (`crates/redbook/src/bin/rip/main.rs` and `cli.rs`)

- Album rip: `info_span!("rip_album", album = %album_title)`
- Track rip start: `tracing::info!("rip_track_start", track = n, name = %track_name)`
- Track rip done: `tracing::info!("rip_track_done", track = n, name = %track_name, duration_secs = ?)`
- Encode start: `tracing::debug!("encode_start", track = n, format = "flac")`
- Encode done: `tracing::debug!("encode_done", track = n, bytes = data.len())`

**4.3 MusicBrainz** (`crates/redbook/src/disc.rs`)

- `Disc::update_musicbrainz()`: `#[tracing::instrument(level = "info", skip(self))]` + record `discid`
- Success: `tracing::info!("musicbrainz_retrieved", releases = mb.releases.len())`
- Error: `tracing::error!("musicbrainz_failed", error = ?err)`

**4.4 Cover art** (`crates/redbook/src/disc.rs`)

- `Disc::update_cover_art()`: `#[tracing::instrument(level = "info", skip(self))]`
- Success: `tracing::info!("coverart_retrieved", size_bytes = data.len())`
- HTTP error (500, 404, etc.): `tracing::warn!("coverart_failed", url = %url, status = code, reason = ?response.text().ok())`

**4.5 Errors** (library-wide)

- Convert existing `dbg!` calls to appropriate `tracing::debug!` or `tracing::error!`
- Preserve all existing error returns; tracing supplements but doesn't replace

---
---

### Phase 5: Validation & Cleanup

**5.1 Run full test suite**

- `cargo test` in redbook crate (it is NOT POSSIBLE to run `cargo test --features all` as binaries will only compile for target windows, you are coding on linunx)
- Verify no regressions in existing tests

**5.2 Manual CLI testing**

Is NOT possible as the CLI will only run on windows. SKIP THIS STEP.

**5.3 Documentation**

- Add `tracing` usage example to `redbook` lib docs
- Document expected spans/events in module docs

**5.4 Remove dead code**

- Remove redundant `dbg!` calls replaced by tracing
- Remove `println!`/`eprintln!` from library code (keep in binaries for user-facing output)

---
---

## Summary Checklist

| # | Task | File | Est. |
| --- | ------ | ------ | ------ |
| 0.1 | Add tracing dep to library | Cargo.toml | 5m |
| 0.2 | Add test deps | Cargo.toml | 5m |
| 0.3 | Test helper module | tests/tracing.rs | 30m |
| 1.1 | Hex tracing tests | tests/tracing_hex.rs | 30m |
| 1.2 | Disc tracing tests | tests/tracing_disc.rs | 60m |
| 1.3 | Frame/Msf tracing tests | lib.rs tests | 20m |
| 1.4 | Tagging tracing tests | tests/tracing_tagging.rs | 30m |
| 2.1 | Hex instrumentation | src/hex.rs | 20m |
| 2.2 | Disc instrumentation | src/disc.rs | 40m |
| 2.3 | Frame/Msf instrumentation | src/lib.rs | 20m |
| 2.4 | Tagging instrumentation | src/tagging.rs | 20m |
| 3.1 | CLI flags | src/bin/rip/cli.rs | 20m |
| 3.2 | Subscriber setup | src/bin/rip/main.rs | 60m |
| 4.1 | Windows instrumentation | src/win.rs | 40m |
| 4.2 | Rip instrumentation | src/bin/rip/*.rs | 40m |
| 4.3 | MusicBrainz instrumentation | src/disc.rs | 20m |
| 4.4 | Cover art instrumentation | src/disc.rs | 20m |
| 5.1 | Full test suite | - | 30m |
| 5.2 | Manual testing | - | 0m (SKIPPED) |
| 5.3 | Documentation | - | 30m |
| 5.4 | Cleanup | - | 30m |
| | **Total** | | **~11 hrs** |

**Remember** AFTER EACH STEP COMMIT YOUR WORK before continuing. Run `git add . && git commit -m <meaningful commit message>` BEFORE moving on to the next step.

---

## Supporting tools

- codebase analysis with graphify skill
- `cargo stage` will run formatting, linting and all tests before calling git add on success
- dependencies (code inclding inline documentation) at /opt/cargo/registry/src/
- stdlib (code inclding inline documentation) at /opt/rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/
- other tools installed can be identified by looking at /workspaces/splurt/.devcontainer/Dockerfile and the base image referenced there

---

## Assumptions

1. `#[tracing::instrument]` macro available (Rust 1.95+ has proc macro support)
2. `tracing = "0.1"` compatible with project's Rust version (1.95.0)
3. Existing test fixtures sufficient — no new fixtures required

If any assumptions appear to be incorrect STOP WORK and ASK FOR CLARIFICATION.

---

## Clarifications

1. The `tracing` dependency is **not optional** (behind a feature flag like `tracing`). It adds little to the overall size of the library.
2. Trace levels. For example, for `Disc::tag_for()`, trace:
| level | details |
|-------|---------|
| TRACE | track_number, full `{VorbisComment:?}` |
| DEBUG | track_number, `{VorbisComment.comments.len()}` |
| INFO | track_number, `"Tagged"`|
