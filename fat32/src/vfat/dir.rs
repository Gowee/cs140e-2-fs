use std::borrow::Cow;
use std::char::decode_utf16;
use std::ffi::OsStr;
use std::io;
use std::iter;
use std::vec;

use traits;
use util::VecExt;
use vfat::{Attributes, Date, Metadata, Time, Timestamp};
use vfat::{Cluster, Entry, File, Shared, VFat};

#[derive(Debug)]
pub struct Dir {
    pub name: String,
    pub metadata: Metadata,
    first_cluster: Cluster,
    vfat: Shared<VFat>,
}

impl Dir {
    fn new(name: String, metadata: Metadata, first_cluster: Cluster, vfat: Shared<VFat>) -> Dir {
        Dir {
            name,
            metadata,
            first_cluster,
            vfat,
        }
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatRegularDirEntry {
    /// File name: 8 ASCII characters.
    /// A file name may be terminated early using 0x00 or 0x20 characters.
    /// If the file name starts with 0x00, the previous entry was the last entry.
    /// If the file name starts with 0xE5, this is a deleted/unused entry
    name: [u8; 8],
    /// File extension: 3 ASCII characters.
    /// A file extension may be terminated early using 0x00 or 0x20 characters.
    extension: [u8; 3],
    /// Attributes of the file. The possible attributes are:
    /// READ_ONLY=0x01 HIDDEN=0x02 SYSTEM=0x04 VOLUME_ID=0x08
    /// DIRECTORY=0x10 ARCHIVE=0x20
    /// LFN=READ_ONLY|HIDDEN|SYSTEM|VOLUME_ID
    /// (LFN means that this entry is a long file name entry)
    attributes: Attributes,
    /// Reserved for use by Windows NT.
    __r0: u8,
    /// Creation time in tenths of a second. Range 0-199 inclusive. Ubuntu uses 0-100.
    _creation_time: u8,
    /// The time that the file was created. Multiply Seconds by 2.
    /// Bits 15 - 11: hours. Bits 10 -5: minutes. Bits 4 - 0: seconds/2
    ctime: Time,
    /// The date on which the file was created.
    /// Bits 15 - 9: Year (0 = 1980). Bits 8 - 5: Month. Bits 4 - 0: Day.
    cdate: Date,
    /// Last accessed date. Same format as the creation date
    adate: Date,
    /// The high 16 bits of this entry's first cluster number.
    /// For FAT 12 and FAT 16 this is always zero.
    first_cluster_higher_bits: u16,
    /// Last modification time. Same format as the creation time.
    mtime: Time,
    /// Last modification date. Same format as the creation date.
    mdate: Date,
    /// The low 16 bits of this entry's first cluster number.
    first_cluster_lower_bits: u16,
    /// The size of the file in bytes.
    size: u32,
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatLfnDirEntry {
    /// Sequence Number
    ///
    /// Bit 6 set: last logical LFN entry.
    /// Bit 5 clear: first physical LFN entry
    /// Bits 4-0: from 0x01 .. 0x14 ( 0x1F ) : position of entry
    /// If the sequence number is 0x00, the previous entry was the last entry.
    /// If the sequence number is 0xE5, this is a deleted/unused entry.
    seq_num: u8,
    /// Name characters (five UCS-2 (subset of UTF-16) characters)
    /// A file name may be terminated early using 0x00 or 0xFF characters.
    name_characters_1: [u8; 10],
    /// Attributes (always 0x0F). Used to determine if a directory entry is an LFN entry.
    attributes: u8,
    /// Type
    /// (always 0x00 for VFAT LFN, other values reserved for future use;
    /// for special usage of bits 4 and 3 in SFNs see further up)
    type_: u8,
    /// Checksum of DOS file name.
    checksum: u8,
    /// Second set of name characters (six UCS-2 characters).
    /// Same early termination conditions apply.
    name_characters_2: [u8; 12],
    /// Always 0x0000 for an LFN.
    __r0: u16,
    /// Third set of name characters (two UCS-2 characters).
    /// Same early termination conditions apply.
    name_characters_3: [u8; 4],
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatUnknownDirEntry {
    seq_num: u8,
    __r0: [u8; 10],
    attributes: u8,
    __r1: [u8; 20],
}

pub union VFatDirEntry {
    unknown: VFatUnknownDirEntry,
    regular: VFatRegularDirEntry,
    long_filename: VFatLfnDirEntry,
}

impl Dir {
    /// Finds the entry named `name` in `self` and returns it. Comparison is
    /// case-insensitive.
    ///
    /// # Errors
    ///
    /// If no entry with name `name` exists in `self`, an error of `NotFound` is
    /// returned.
    ///
    /// If `name` contains invalid UTF-8 characters, an error of `InvalidInput`
    /// is returned.
    pub fn find<P: AsRef<OsStr>>(&self, name: P) -> io::Result<Entry> {
        use traits::{Dir, Entry};
        match name.as_ref().to_str() {
            None => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "File name contains non unicode charaters.",
            )),
            Some(name) => {
                for entry in self.entries()? {
                    if entry.name().eq_ignore_ascii_case(name) {
                        return Ok(entry);
                    }
                }
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "File is not found.",
                ));
            }
        }
    }
}

impl traits::Dir for Dir {
    /// The type of entry stored in this directory.
    type Entry = Entry;

    /// An type that is an iterator over the entries in this directory.
    type Iter = EntryIter;

    /// Returns an interator over the entries in this directory.
    fn entries(&self) -> io::Result<Self::Iter> {
        let mut buf = Vec::new();
        self.vfat
            .borrow_mut()
            .read_chain(self.first_cluster, &mut buf)?;
        let raw_entries: Vec<VFatDirEntry> = unsafe { buf.cast() }; // TODO: works or not?
        Ok(EntryIter::new(raw_entries.into_iter(), self.vfat.clone()))
    }
}

pub struct EntryIter {
    raw_entries: vec::IntoIter<VFatDirEntry>,
    vfat: Shared<VFat>,
    lfn: Option<[[u8; 26]; 0x1F]>,
}

impl EntryIter {
    fn new(raw_entries: vec::IntoIter<VFatDirEntry>, vfat: Shared<VFat>) -> EntryIter {
        EntryIter {
            raw_entries,
            vfat,
            lfn: None,
        }
    }
}

impl iter::Iterator for EntryIter {
    type Item = Entry;

    fn next(&mut self) -> Option<Self::Item> {
        self.raw_entries.next().and_then(|raw_entry: VFatDirEntry| {
            let entry = unsafe { raw_entry.unknown };
            match entry.seq_num {
                0x00 => None,        // the previous entry was the last entry
                0xE5 => self.next(), // this is a deleted/unused entry; TODO: should lfn be cleared?
                seq_num => {
                    if entry.attributes == 0x0F {
                        // VFatLfnDirEntry
                        if !(seq_num >= 0x01 && seq_num <= 0x1F) {
                            // invalid seq_num
                            panic!("Unexpected sequence number: {}.", seq_num);
                        }
                        let entry = unsafe { raw_entry.long_filename };
                        let mut lfn = self.lfn.get_or_insert([[0x00; 26]; 0x1F])[seq_num as usize];
                        lfn[0..10].copy_from_slice(&entry.name_characters_1);
                        lfn[10..22].copy_from_slice(&entry.name_characters_2);
                        lfn[22..26].copy_from_slice(&entry.name_characters_3);
                        self.next()
                    } else {
                        let entry = unsafe { raw_entry.regular };
                        let mut file_name = match self.lfn {
                            None => String::from(""),
                            Some(ref lfn) => {
                                let raw_lfn: Vec<u8> = lfn
                                    .into_iter()
                                    .flatten()
                                    .map(|c| *c)
                                    .take_while(|&c| c != 0x00 && c != 0xFF)
                                    .collect();
                                let raw_lfn: Vec<u16> = unsafe { raw_lfn.cast() }; // TODO:
                                String::from_utf16_lossy(raw_lfn.as_slice())
                            }
                        };
                        self.lfn = None; // clear lfn

                        let name: Vec<u8> = entry
                            .name
                            .iter()
                            .map(|c| *c)
                            .take_while(|&c| c != 0x00 && c != 0x20)
                            .collect();
                        file_name.push_str(&String::from_utf8_lossy(&name));
                        let extension: Vec<u8> = entry
                            .extension
                            .iter()
                            .map(|c| *c)
                            .take_while(|&c| c != 0x00 && c != 0x20)
                            .collect();
                        if !extension.is_empty() {
                            file_name.push_str(".");
                        }
                        file_name.push_str({ &String::from_utf8_lossy(&extension) });

                        let metadata = Metadata {
                            attributes: entry.attributes,
                            created_time: (entry.cdate, entry.ctime).into(),
                            accessed_time: (entry.adate, 0.into()).into(),
                            modified_time: (entry.mdate, entry.mtime).into(),
                        };

                        let first_cluster = (((entry.first_cluster_higher_bits as u32) << 16)
                            | entry.first_cluster_lower_bits as u32)
                            .into();
                        Some(if metadata.attributes.directory() {
                            Entry::Dir(Dir::new(
                                file_name,
                                metadata,
                                first_cluster,
                                self.vfat.clone(),
                            ))
                        } else {
                            Entry::File(File::new(
                                file_name,
                                metadata,
                                entry.size,
                                first_cluster,
                                self.vfat.clone(),
                            ))
                        })
                    }
                }
            }
        })
    }
}
