use std::{io, fmt, mem, str};

use traits::BlockDevice;
use vfat::Error;

#[repr(C, packed)]
pub struct BiosParameterBlock {
    first_three: [u8; 3],
    pub oem_identifier: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub number_of_reserved_sectors: u16,
    pub number_of_fats: u8,
    pub max_no_of_director_entries: u16,
    pub total_logical_sectors: u16,
    pub fat_id: u8,
    pub number_of_sectors_per_fat: u16,
    pub number_of_sectors_per_track: u16,
    pub number_of_heads_or_sides: u16,
    pub number_of_hidden_sectors: u32,
    pub total_logical_sectors_: u32,

    pub sectors_per_fat: u32,
    pub flags: u16,
    pub fat_version_number: u16,
    pub cluster_no_of_root_directory: u32,
    pub sector_no_of_fsinfo_structure: u16,
    pub sector_no_of_backup_boot_sector: u16,
    __r0: [u8; 12],
    pub drive_number: u8,
    pub flags_winnt: u8,
    pub signature: u8,
    pub volume_id_serial_no: u32,
    pub volume_label_string: [u8; 11],
    pub system_identifier_string: [u8; 8],
    boot_code: [u8; 420],
    bootable_partition_signature: u16,
}

impl BiosParameterBlock {
    /// Reads the FAT32 extended BIOS parameter block from sector `sector` of
    /// device `device`.
    ///
    /// # Errors
    ///
    /// If the EBPB signature is invalid, returns an error of `BadSignature`.
    pub fn from<T: BlockDevice>(mut device: T, sector: u64) -> Result<BiosParameterBlock, Error> {
        let mut buf = [0u8; 512];
        if device.read_sector(sector, &mut buf)? != 512 {
            return Err(Error::Io(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Got less than 512 bytes when reading MBR.",
            )));
        }
        let bpb = unsafe { mem::transmute::<[u8; 512], BiosParameterBlock>(buf) };
        /* if (bpb.signature >> 1) != (0x28 >> 1) {
            return Err(Error::BadSignature);
        }*/
        if bpb.bootable_partition_signature != 0xAA55 {
            return Err(Error::BadSignature);
        }
        Ok(bpb)
    }
}

impl fmt::Debug for BiosParameterBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BiosParameterBlock")
            .field("first_three", &self.first_three)
            .field("oem_identifier", &str::from_utf8(&self.oem_identifier).unwrap_or("INVALID_TEXT"))
            .field("bytes_per_sector", &self.bytes_per_sector)
            .field("sectors_per_cluster", &self.sectors_per_cluster)
            .field("number_of_reserved_sectors", &self.number_of_reserved_sectors)
            .field("number_of_fats", &self.number_of_fats)
            .field("max_no_of_director_entries", &self.max_no_of_director_entries)
            .field("total_logical_sectors", &self.total_logical_sectors)
            .field("fat_id", &self.fat_id)
            .field("number_of_sectors_per_fat", &self.number_of_sectors_per_fat)
            .field("number_of_sectors_per_track", &self.number_of_sectors_per_track)
            .field("number_of_heads_or_sides", &self.number_of_heads_or_sides)
            .field("number_of_hidden_sectors", &self.number_of_hidden_sectors)
            .field("total_logical_sectors_", &self.total_logical_sectors_)
            .field("sectors_per_fat", &self.sectors_per_fat)
            .field("flags", &self.flags)
            .field("fat_version_number", &self.fat_version_number)
            .field("cluster_no_of_root_directory", &self.cluster_no_of_root_directory)
            .field("sector_no_of_fsinfo_structure", &self.sector_no_of_fsinfo_structure)
            .field("sector_no_of_backup_boot_sector", &self.sector_no_of_backup_boot_sector)
            // .field("__r0", &self.__r0)
            .field("drive_number", &self.drive_number)
            .field("flags_winnt", &self.flags_winnt)
            .field("signature", &self.signature)
            .field("volume_id_serial_no", &self.volume_id_serial_no)
            .field("volume_label_string", &str::from_utf8(&self.volume_label_string).unwrap_or("INVALID_TEXT"))
            .field("system_identifier_string", &str::from_utf8(&self.system_identifier_string).unwrap_or("INVALID_TEXT"))
            // .field("boot_code", &self.boot_code)
            .field("bootable_partition_signature", &self.bootable_partition_signature)
            .finish()
    }
}
