# futures-udp

Runtime agnostic, non-blocking, non-exclusive async UDP networking.

`futures-udp` provides two key structs:

- `UdpStream` for reading data from a UDP Socket
- `UdpSink` for sending data via a UDP Socket

These structs implement the `futures-rs` traits `Stream` & `Sink` respectively but are tested
and known to work with both `tokio` & `futures`-rs runtimes. (tokio tests performed in a
downstream crate, I'll add them here soon so to make sure this never breaks)

## Why?

- I usually don't want to be forced to bring `tokio` into my dependency tree unless I want
  to use it as my runtime. I think the runtime choice should be left to the final binary.
- `futures-rs` is a lot lighter weight and provided by rust-lang, so I chose that for the base
  traits. They are cross-compatible with `tokio`.
- Working with a bare UdpSocket is "a bit hard", doing it async is "a bit more hard".
  Adding `Stream` & `Sink` semantics makes it "nice".
- Despite the docs `futures_net::UdpSocket` creates a blocking socket, which is locked
  for exclusive use. (Opening a ticket TBD)

## Where this started

I wanted to create a simple SSDP-repeater ([splurt](https://github.com/MusicalNinjaDad/splurt)) to allow me to run DLNA services in docker containers. I looked at the crates available and couldn't find anything that gave me a usable API for UDP where I felt confident in the code and it did what I needed. So I ended up building it myself and decided to split it out into a separate crate.

## Stability & MSRV

I've chosen to rely on two experimental features, while this crate is in v0.x.y, as I feel they
add significant value to the API. I also believe in supporting language development and
generating feedback to features as they near stabilisation.

This crate will not move to v1.x.y until both features are stabilised, or I decide to stop using
them. Realistically, however, they will be stable while I allow this API to go through a
"settling-in" phase before fixing it at v1.0.0

> 🔬 **Experimental Features**
>
> This crate makes use of the following experimental features:
>
> - [`#![feature(never_type)]`](https://github.com/rust-lang/rust/issues/35121) [final stages of stabilisation]
> - [`#![feature(bool_to_result)]`](https://github.com/rust-lang/rust/issues/142748) [in FCP as of 2026-04-25]
>
> This list includes any unstable features used by direct & transitive dependencies (currently, none).
>
> Both are so close to being part of stable rust that I chose to use them here.

You do not need to enable these in your own code, the list is for information only.

### Stability guarantees

We run automated tests **every month** to ensure no fundamental changes affect this crate and
test every PR against the current nightly, as well as the current equivalent beta & stable.
If you find an issue before we do, please
[raise an issue on github](https://github.com/MusicalNinjaDad/splurt/issues).

### MSRV

For those of you working with a pinned nightly (etc.) this crate supports the equivalent of
1.90.0 onwards. We use [autocfg](https://crates.io/crates/autocfg/) to seamlessly handle
features which have been stabilised since then.

### Dependencies

We deliberately keep the dependency list short and pay attention to any transitive dependencies
we bring in.

- `futures-rs` (for the Stream & Sink traits)
- `futures-net` (for the underlying UdpSocket)
- `socket2` (to set the socket to non-blocking, non-exclusive)
