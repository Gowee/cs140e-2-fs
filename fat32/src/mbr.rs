use std::{fmt, io, mem};

use traits::BlockDevice;

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct CHS {
    head: u8, // head
    sector_and_cylinder: u16, // sector (Bits 6-7 are the upper two bits for the Starting Cylinder field.) and Cylinder
}

impl fmt::Debug for CHS {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("CHS")
            .field("head", &self.head)
            .field("sector", &(self.sector_and_cylinder & (0b11111 << 6)))
            .field("cylinder", &(self.sector_and_cylinder >> 6))
            .finish()
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct PartitionEntry {
    /// Boot indicator bit flag: 0 = no, 0x80 = bootable (or "active")
    pub boot_indicator: u8,
    starting_chs: CHS,
    /// Partition Type (0xB or 0xC for FAT32).
    pub partition_type: u8,
    ending_chs: CHS,
    /// Relative Sector (offset, in sectors, from start of disk to start of the partition)
    pub relative_sector: u32,
    /// Total Sectors in partition
    pub total_sectors: u32,
}

/// The master boot record (MBR).
#[repr(C, packed)]
pub struct MasterBootRecord {
    bootstrap: [u8; 436], //MBR Bootstrap (flat binary executable code)
    /// Optional "unique" disk ID
    pub disk_id: [u8; 10],
    /// MBR Partition Table
    pub partition_table: [PartitionEntry; 4],
    signature: [u8; 2], // (0x55, 0xAA) "Valid bootsector" signature byte
}

#[derive(Debug)]
pub enum Error {
    /// There was an I/O error while reading the MBR.
    Io(io::Error),
    /// Partiion `.0` (0-indexed) contains an invalid or unknown boot indicator.
    UnknownBootIndicator(u8),
    /// The MBR magic signature was invalid.
    BadSignature,
}

impl MasterBootRecord {
    /// Reads and returns the master boot record (MBR) from `device`.
    ///
    /// # Errors
    ///
    /// Returns `BadSignature` if the MBR contains an invalid magic signature.
    /// Returns `UnknownBootIndicator(n)` if partition `n` contains an invalid
    /// boot indicator. Returns `Io(err)` if the I/O error `err` occured while
    /// reading the MBR.
    pub fn from<T: BlockDevice>(mut device: T) -> Result<MasterBootRecord, Error> {
        let mut buf = [0u8; 512];
        if device.read_sector(0, &mut buf).map_err(|e| Error::Io(e))? != 512 {
            return Err(Error::Io(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Got less than 512 bytes when reading MBR.",
            )));
        }
        let mbr = unsafe { mem::transmute::<[u8; 512], MasterBootRecord>(buf) };
        if mbr.signature != [0x55, 0xAA] {
            return Err(Error::BadSignature);
        }
        for (index, partition_entry) in mbr.partition_table.iter().enumerate() {
            match partition_entry.boot_indicator {
                0x0 | 0x80 => (),
                _ => return Err(Error::UnknownBootIndicator(index as u8)),
            }
        }
        Ok(mbr)
    }

    pub fn first_fat32_partition(&self) -> Option<&PartitionEntry> {
        self.first_partition_of(&[0xB, 0xC])
    }

    pub fn first_partition_of(&self, partition_type: &[u8]) -> Option<&PartitionEntry> {
        for entry in self.partition_table.iter() {
            if partition_type.contains(&entry.partition_type) {
                return Some(entry);
            }
        }
        return None;
    }
}

impl fmt::Debug for MasterBootRecord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("MasterBootRecord")
            .field("bootstrap", &String::from("<ellipsis>"))
            .field("disk_id", &self.disk_id)
            .field("partition_table", &self.partition_table)
            .field("signature", &self.signature)
            .finish()
    }
}
