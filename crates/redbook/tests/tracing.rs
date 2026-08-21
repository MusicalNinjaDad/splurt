//! Test utilities for tracing
//!
//! This module provides utilities for testing tracing output in the redbook library.

use tracing::subscriber::DefaultGuard;
use tracing_subscriber::{
    fmt, layer::{Layer, SubscriberExt},
    util::SubscriberInitExt,
    EnvFilter, Registry,
};

/// Guard that holds the subscriber
pub struct TestTracingGuard {
    _subscriber: DefaultGuard,
}

impl TestTracingGuard {
    /// Create a new test tracing guard with the specified filter
    pub fn new(filter: &str) -> Self {
        let env_filter = EnvFilter::try_new(filter).expect("invalid filter");
        
        let subscriber = Registry::default()
            .with(
                fmt::layer()
                    .with_test_writer()
                    .with_ansi(false)
                    .with_target(false)
                    .with_filter(env_filter),
            );
        
        let guard = subscriber.set_default();
        
        Self {
            _subscriber: guard,
        }
    }
}

impl Default for TestTracingGuard {
    fn default() -> Self {
        Self::new("trace")
    }
}

/// Initialize test tracing with a TRACE level filter
pub fn init_trace_tracing() -> TestTracingGuard {
    TestTracingGuard::new("trace")
}

/// Initialize test tracing with a DEBUG level filter
pub fn init_debug_tracing() -> TestTracingGuard {
    TestTracingGuard::new("debug")
}

/// Initialize test tracing with an INFO level filter
pub fn init_info_tracing() -> TestTracingGuard {
    TestTracingGuard::new("info")
}

/// Initialize test tracing with a custom filter string
pub fn init_capturing_tracing(filter: &str) -> TestTracingGuard {
    TestTracingGuard::new(filter)
}
