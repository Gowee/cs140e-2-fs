use std::{io, fmt, mem};

use traits::BlockDevice;
use vfat::Error;

#[repr(C, packed)]
pub struct BiosParameterBlock {
    first_three: [u8; 3],
    oem_identifier: [u8; 8],
    bytes_per_sector: [u8; 2],
    sectors_per_cluster: u8,
    number_of_reserved_sectors: [u8; 2],
    number_of_fats: u8,
    max_no_of_director_entries: [u8; 2],
    total_logical_sectors: [u8; 2],
    fat_id: u8,
    number_of_sectors_per_fat: [u8; 2],
    number_of_sectors_per_track: [u8; 2],
    number_of_heads_or_sides: [u8; 2],
    number_of_hidden_sectors: [u8; 4],
    total_logical_sectors_: [u8; 4],

    sectors_per_fat: [u8; 4],
    flags: [u8; 2],
    fat_version_number: [u8; 2],
    cluster_no_of_root_directory: [u8; 4],
    sector_no_of_fsinfo_structure: [u8; 2],
    sector_no_of_backup_boot_sector: [u8; 2],
    __r0: [u8; 12],
    drive_number: u8,
    flags_winnt: u8,
    signature: u8,
    volume_id_serial_no: [u8; 4],
    volume_label_string: [u8; 11],
    system_identifier_string: [u8; 8],
    boot_code: [u8; 420],
    bootable_partition_signature: [u8; 2],
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
        if bpb.bootable_partition_signature != [0x55, 0xAA] {
            return Err(Error::BadSignature);
        }
        Ok(bpb)
    }
}

impl fmt::Debug for BiosParameterBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unimplemented!("BiosParameterBlock::debug()")
    }
}
