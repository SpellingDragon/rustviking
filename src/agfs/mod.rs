//! AGFS Virtual FileSystem
//!
//! POSIX-style virtual filesystem with plugin routing.

pub mod filesystem;
pub mod mountable;
pub mod viking_uri;
pub mod metadata;

pub use filesystem::{FileSystem, FileInfo, WriteFlag};
pub use mountable::{MountableFS, MountPoint};
pub use viking_uri::VikingUri;
pub use metadata::Metadata;
