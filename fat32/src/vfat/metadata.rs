use std::fmt;

use traits;

/// A date as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Date(u16);

/// Time as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Time(u16);

/// File attributes as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Attributes(u8);

/// A structure containing a date and time.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct Timestamp {
    pub date: Date,
    pub time: Time,
}

/// Metadata for a directory entry.
#[derive(Default, Debug, Clone)]
pub struct Metadata {
    pub attributes: Attributes,
    pub created_time: Timestamp,
    pub accessed_time: Timestamp,
    pub modified_time: Timestamp,
}

pub(super) const ROOTMETADATA: Metadata = Metadata {
    attributes: Attributes(0x10),
    created_time: Timestamp {
        date: Date(0),
        time: Time(0),
    },
    accessed_time: Timestamp {
        date: Date(0),
        time: Time(0),
    },
    modified_time: Timestamp {
        date: Date(0),
        time: Time(0),
    },
};

impl From<u16> for Date {
    fn from(raw: u16) -> Date {
        Date(raw)
    }
}

impl From<u16> for Time {
    fn from(raw: u16) -> Time {
        Time(raw)
    }
}

impl From<(Date, Time)> for Timestamp {
    fn from(date_time: (Date, Time)) -> Timestamp {
        Timestamp {
            date: date_time.0,
            time: date_time.1,
        }
    }
}

impl From<u8> for Attributes {
    fn from(raw: u8) -> Attributes {
        Attributes(raw)
    }
}

impl traits::Timestamp for Timestamp {
    /// The calendar year.
    ///
    /// The year is not offset. 2009 is 2009.
    fn year(&self) -> usize {
        (self.date.0 >> 9) as usize + 1980 // Is the endianness right?
    }

    /// The calendar month, starting at 1 for January. Always in range [1, 12].
    ///
    /// January is 1, Feburary is 2, ..., December is 12.
    fn month(&self) -> u8 {
        ((self.date.0 & (0xF << 5)) >> 5) as u8
    }

    /// The calendar day, starting at 1. Always in range [1, 31].
    fn day(&self) -> u8 {
        (self.date.0 & 0b11111) as u8
    }

    /// The 24-hour hour. Always in range [0, 24).
    fn hour(&self) -> u8 {
        (self.time.0 >> 11) as u8
    }

    /// The minute. Always in range [0, 60).
    fn minute(&self) -> u8 {
        ((self.time.0 & (0b111111 << 5)) >> 5) as u8
    }

    /// The second. Always in range [0, 60).
    fn second(&self) -> u8 {
        ((self.time.0 & 0b11111) * 2) as u8
    }
}

impl Attributes {
    const READ_ONLY: u8 = 0x01;
    const HIDDEN: u8 = 0x02;
    const SYSTEM: u8 = 0x04;
    const VOLUME_ID: u8 = 0x08;
    const DIRECTORY: u8 = 0x10;
    const ARCHIVE: u8 = 0x20;
    const LFN: u8 = Self::READ_ONLY | Self::HIDDEN | Self::SYSTEM | Self::VOLUME_ID;

    // `val & mask == mask` is necessary!
    // barely `!= 0` does not work because there is mask like 0x10 which has two or more bits set
    pub fn read_only(&self) -> bool {
        self.0 & Self::READ_ONLY == Self::READ_ONLY
    }

    pub fn hidden(&self) -> bool {
        self.0 & Self::HIDDEN == Self::HIDDEN
    }

    pub fn system(&self) -> bool {
        self.0 & Self::SYSTEM == Self::SYSTEM
    }

    pub fn volume_id(&self) -> bool {
        self.0 & Self::VOLUME_ID == Self::VOLUME_ID
    }

    pub fn directory(&self) -> bool {
        self.0 & Self::DIRECTORY == Self::DIRECTORY
    }

    pub fn archive(&self) -> bool {
        self.0 & Self::ARCHIVE == Self::ARCHIVE
    }

    pub fn lfn(&self) -> bool {
        self.0 & Self::LFN == Self::LFN
    }
}

impl traits::Metadata for Metadata {
    type Timestamp = Timestamp;

    /// Whether the associated entry is read only.
    fn read_only(&self) -> bool {
        self.attributes.read_only()
    }

    /// Whether the entry should be "hidden" from directory traversals.
    fn hidden(&self) -> bool {
        self.attributes.hidden()
    }

    /// The timestamp when the entry was created.
    fn created(&self) -> Self::Timestamp {
        self.created_time
    }

    /// The timestamp for the entry's last access.
    fn accessed(&self) -> Self::Timestamp {
        self.accessed_time
    }

    /// The timestamp for the entry's last modification.
    fn modified(&self) -> Self::Timestamp {
        self.modified_time
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use traits::Timestamp;
        write!(
            f,
            "{}-{}-{} {}:{}:{}",
            self.year(),
            self.month(),
            self.day(),
            self.hour(),
            self.minute(),
            self.second()
        )
    }
}

// FIXME: Implement `fmt::Display` (to your liking) for `Metadata`.
impl fmt::Display for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use traits::Metadata;
        write!(f, "Metadata:\n")?;
        if self.attributes.lfn() {
            write!(f, "LFN")?;
        } else {
            if self.read_only() {
                write!(f, "R/O")?;
            }
            if self.hidden() {
                write!(f, " HIDDEN")?;
            }
        }
        write!(
            f,
            "\nctime: {}\natime: {}\nmtime: {}",
            self.created_time, self.accessed_time, self.modified_time
        )
    }
}
