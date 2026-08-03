# Skills Reference

## `cargo msrv`

**Purpose**: Find and verify Minimum Supported Rust Version (MSRV) for Rust crates.

**Important**: Must be run against a specific crate (not workspace). Use `--path` or `--manifest-path` to target a specific crate.

### Commands

#### `cargo msrv verify`

Verify whether the MSRV specified in Cargo.toml is satisfiable.

**Options**:

- `--path <Crate Directory>`: Path to project root directory
- `--manifest-path <Cargo Manifest>`: Path to cargo manifest file  
- `--min <VERSION>`: Least recent version to check
- `--max <VERSION>`: Most recent version to check
- `--ignore-lockfile`: Ignore the lockfile for MSRV search
- `--no-check-feedback`: Don't print compatibility check results

**Usage**:

```bash
cargo msrv verify --path crates/upnp2
cargo msrv verify --manifest-path crates/upnp2/Cargo.toml
```

#### `cargo msrv find`

Find the MSRV for a crate.

**Options**:

- `--path <Crate Directory>`: Path to project root directory
- `--manifest-path <Cargo Manifest>`: Path to cargo manifest file
- `--bisect`: Use binary search (default, faster)
- `--linear`: Use linear search
- `--write-toolchain-file`: Pin the MSRV by writing to rust-toolchain file
- `--ignore-lockfile`: Temporarily remove lockfile

**Usage**:

```bash
cargo msrv find --path crates/upnp2
cargo msrv find --manifest-path crates/upnp2/Cargo.toml
```

#### `cargo msrv show`

Show the MSRV of a crate as specified in its Cargo.toml.

**Usage**:

```bash
cargo msrv show --path crates/upnp2
```

#### `cargo msrv set`

Set the MSRV of the current crate.

**Usage**:

```bash
cargo msrv set 1.94.0 --path crates/upnp2
```

### Workflow

1. **Verify current MSRV**:

   ```bash
   cargo msrv verify --path crates/upnp2
   ```

2. **If verification fails**, find the actual MSRV:

   ```bash
   cargo msrv find --path crates/upnp2
   ```

3. **Update Cargo.toml** with the found MSRV in the appropriate field:
   - `package.rust-version` for the crate
   - Or workspace-level `rust-version` if applicable
