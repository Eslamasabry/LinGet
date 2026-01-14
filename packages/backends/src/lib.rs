pub mod backends;
pub mod pkexec;
pub mod streaming;
pub mod traits;

pub use pkexec::{run_pkexec, run_pkexec_with_logs, Suggest};
pub use streaming::{StreamLine, StreamType};
pub use traits::{LockStatus, PackageBackend};

pub mod prelude {
    pub use super::traits::PackageBackend;
    pub use super::LockStatus;
}
