//! AGFS Virtual FileSystem
//!
//! POSIX-style virtual filesystem with plugin routing.

pub mod filesystem;
pub mod metadata;
pub mod mountable;
pub mod setup;
pub mod viking_uri;

pub use filesystem::{FileInfo, FileSystem, WriteFlag};
pub use metadata::Metadata;
pub use mountable::{MountPoint, MountableFS};
pub use setup::setup_agfs;
pub use viking_uri::VikingUri;
