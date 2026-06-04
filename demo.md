# Demo for Rust Nürnberg

I would be interested in joining the rust meetup next week and even have a project to show (presentation English, Q&A/discussions German/English). It's my first meetup so I thought I would ask first, if this is the sort of topic that would be interesting.

A couple of months ago I decided to create a UPnP message repeater to allow me to run DLNA services in containers without full host-network access. This led to me going down many rabbit-holes which I'd like to open for discussion:

- **async, networking & custom futures**: a non-blocking, non-exclusive `async UDPSocket` (not available in futures-net) which also implements `Stream` & `Sink` [MusicalNinjaDad/futures-udp]
- **nightly: experimental `trait Try` & `!` (never type)**: Implementing Try, what the new `never_type` will bring us (due to hit stable very soon...), nicer compile errors from custom proc-macros [MusicalNinjaDad/try_v2] & [MusicalNinjaDad/proc_macro2_diagnostic]
- **async & `try bikeshed`**: using `?` inside loops in anonymous async blocks (yes, the syntax really is `try bikeshed`!)
- **staying stable while using `nightly`**: Handling the stabilisation lifecycle as experimental features move to `stable` (was anyone using `assert_matches` before 2026-04-10 and got hit by the changes to beta?) [MusicalNinjaDad/rust - ninja-build_rs]
- **implementing `Termination`**: yes you can have custom error codes, use `?` in your `main()` and run `Drop` properly (or do currently you rely on RAII without realising the destructors probably won't run on error?!) [MusicalNinjaDad/exit_safely]
- **lenient parsing**: UPnP is horribly implemented by many devices - how to leverage type-safety to ensure a strictly compliant implementation for outgoing messages while also accepting such things as "RANDOMTEXT_1235545646545_MS" as a `Uuid` from other devices.

The original project is at [MusicalNinjaDad/splurt] and still very WIP.

My thought would be to show the trigger point for each topic in the code and then spend 3-5 minutes giving an overview, aiming to have most of the time spent Q&A style on the topic that interest attendees.

What do you think?
