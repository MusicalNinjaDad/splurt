//! `NOTIFY *` messages

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Notify {
    Alive,
}
