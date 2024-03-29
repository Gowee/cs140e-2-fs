use std::fmt;
use vfat::*;

#[derive(Debug, PartialEq)]
pub enum Status {
    /// The FAT entry corresponds to an unused (free) cluster.
    Free,
    /// The FAT entry/cluster is reserved.
    Reserved,
    /// The FAT entry corresponds to a valid data cluster. The next cluster in
    /// the chain is `Cluster`.
    Data(Cluster),
    /// The FAT entry corresponds to a bad (disk failed) cluster.
    Bad,
    /// The FAT entry corresponds to a valid data cluster. The corresponding
    /// cluster is the last in its chain.
    Eoc(u32),
}

#[repr(C, packed)]
pub struct FatEntry(pub u32);

impl FatEntry {
    /// Returns the `Status` of the FAT entry `self`.
    pub fn status(&self) -> Status {
        use self::Status::*;
        match self.0 & !(0xF << 28) { // ignore the upper 4 digits
            0x0000000 => Free,
            0x0000001 => Reserved,
            v @ 0x0000002...0xFFFFFEF => Data(v.into()),
            0xFFFFFF0...0xFFFFFF6 => Reserved,
            0xFFFFFF7 => Bad,
            v @ 0xFFFFFF8...0xFFFFFFF => Eoc(v),
            _ => unreachable!(),
        }
    }
}

impl fmt::Debug for FatEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("FatEntry")
            .field("value", &self.0)
            .field("status", &self.status())
            .finish()
    }
}
