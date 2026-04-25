#![cfg_attr(all(test, unstable_assert_matches), feature(assert_matches))]
#![cfg_attr(unstable_bool_to_result, feature(bool_to_result))]
#![cfg_attr(unstable_let_chains, feature(let_chains))]

pub mod message;
pub mod search;
pub mod udp;
