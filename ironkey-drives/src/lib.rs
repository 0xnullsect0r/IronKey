//! ironkey-drives — Drive enumeration, partition management, and SMART diagnostics.
//!
//! Parses `/sys/block/` to enumerate all block devices and their partitions,
//! reads SMART health data via ATA ioctl, and provides disk operations
//! (format, clone, wipe).

pub mod enumerate;
pub mod ops;
pub mod smart;

pub use enumerate::{
    enumerate_drives, format_size, DriveInfo, DriveStatus, PartitionInfo,
    PartitionStatus,
};
pub use ops::{CloneProgress, FormatOpts, WipeMode};
pub use smart::SmartData;
