//! Project package layout, manifest, and safe on-disk creation/validation.
//!
//! ```text
//! <display-name>.wcproj/
//!   manifest.json
//!   data/
//!     project.sqlite
//!   assets/
//!   staging/
//! ```
//!
//! The package directory name may be influenced by the working name at
//! creation time, but it is never identity: renaming a Project does not
//! rename its `.wcproj` directory, and opening a package never trusts its
//! directory name for anything but locating `manifest.json`.

pub mod error;
pub mod layout;
pub mod manifest;

pub use error::PackageError;
pub use layout::PackagePaths;
pub use manifest::Manifest;

pub const FORMAT_VERSION: i64 = 1;
