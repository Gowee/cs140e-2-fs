use std::cmp::min;
use std::io;
use std::mem::size_of;
use std::path::Path;

use mbr::MasterBootRecord;
use traits::{BlockDevice, FileSystem};
use util::SliceExt;
use vfat::{BiosParameterBlock, CachedDevice, Partition};
use vfat::{Cluster, Dir, Entry, Error, FatEntry, File, Shared, Status};

#[derive(Debug)]
pub struct VFat {
    device: CachedDevice,
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    sectors_per_fat: u32,
    fat_start_sector: u64,
    data_start_sector: u64,
    root_dir_cluster: Cluster,
}

impl VFat {
    pub fn from<T>(mut device: T) -> Result<Shared<VFat>, Error>
    where
        T: BlockDevice + 'static,
    {
        let mbr = MasterBootRecord::from(&mut device)?;
        let fat32 = mbr.first_fat32_partition().ok_or(Error::NotFound)?;
        let bpb = BiosParameterBlock::from(&mut device, fat32.relative_sector as u64)?;

        let bps = bpb.bytes_per_sector;
        let spc = bpb.sectors_per_cluster;
        let spf = bpb.sectors_per_fat;
        let fss = fat32.relative_sector as u64 /* start of partition */ /*+ 1  BPB */ + bpb.number_of_reserved_sectors as u64;
        let rdc: Cluster = bpb.cluster_no_of_root_directory.into(); // TODO: NOTIMPLEMTNED YET!
        let cached_device = CachedDevice::new(
            device,
            Partition {
                start: fat32.relative_sector as u64,
                sector_size: bpb.bytes_per_sector as u64,
            },
        );
        let vfat = VFat {
            device: cached_device,
            bytes_per_sector: bps,
            sectors_per_cluster: spc,
            sectors_per_fat: spf,
            fat_start_sector: fss,
            data_start_sector: fss as u64
                + bpb.number_of_fats as u64 * bpb.number_of_sectors_per_fat as u64,
            root_dir_cluster: rdc,
        };
        // println!("{:#?}", vfat);
        Ok(Shared::new(vfat))
    }

    #[inline(always)]
    fn cluster_size(&self) -> usize {
        self.sectors_per_cluster as usize * self.bytes_per_sector as usize
    }

    // TODO: The following methods may be useful here:
    //
    ///  * A method to read from an offset of a cluster into a buffer.

    fn read_cluster(
        &mut self,
        cluster: Cluster,
        offset: usize,
        buf: &mut [u8],
    ) -> io::Result<usize> {
        if self.fat_entry(cluster)?.status() == Status::Bad {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Cluster is bad."));
        }
        let mut nsector = self.data_start_sector
            + (cluster.inner() as u64 - 2) * self.sectors_per_cluster as u64
            + offset as u64 / self.bytes_per_sector as u64;
        let mut index = {
            let sector = self.device.get(nsector)?;
            let offset_in_sector = offset % self.bytes_per_sector as usize;
            let until = min(buf.len() + offset_in_sector, self.bytes_per_sector as usize);
            &mut buf[..until - offset_in_sector].copy_from_slice(&sector[offset_in_sector..until]);
            nsector += 1;
            until - offset_in_sector
        };
        let total = min(
            self.sectors_per_cluster as usize * self.bytes_per_sector as usize - offset,
            buf.len(),
        );

        while index < total {
            self.device.read_sector(nsector, &mut buf[index..])?;
        }
        Ok(total)
    }

    ///  * A method to read all of the clusters chained from a starting cluster
    ///    into a vector.
    ///
    fn read_chain(&mut self, start: Cluster, buf: &mut Vec<u8>) -> io::Result<usize> {
        let mut cluster = Some(start);
        let mut index = 0;
        while cluster.is_some() {
            let next = match self.fat_entry(cluster.unwrap())?.status() {
                Status::Data(n) => Some(n),
                Status::Eoc(_) => None,
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "FAT entry other than Data and Eoc encountered.",
                    ))
                }
            };

            index += self.read_cluster(cluster.unwrap(), 0, &mut buf[index..])?;
            cluster = next;
        }
        Ok(index)
    }

    ///  * A method to return a reference to a `FatEntry` for a cluster where the
    ///    reference points directly into a cached sector.
    fn fat_entry(&mut self, cluster: Cluster) -> io::Result<&FatEntry> {
        let offset_by_byte = cluster.inner() * 4;
        let offset_by_sector = offset_by_byte / self.bytes_per_sector as u32;
        if offset_by_sector >= self.sectors_per_fat {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Cluster does not exist.",
            ));
        }
        let nsector = offset_by_sector as u64 + self.fat_start_sector;
        let sector = self.device.get(nsector)?;
        let offset_in_sector = offset_by_byte as usize % self.bytes_per_sector as usize;
        Ok(unsafe {
            &*(sector[offset_in_sector..offset_in_sector + 4].as_ptr() as *const FatEntry)
        })
    }
}

impl<'a> FileSystem for &'a Shared<VFat> {
    type File = ::traits::Dummy;
    type Dir = ::traits::Dummy;
    type Entry = ::traits::Dummy;

    fn open<P: AsRef<Path>>(self, path: P) -> io::Result<Self::Entry> {
        unimplemented!("FileSystem::open()")
    }

    fn create_file<P: AsRef<Path>>(self, _path: P) -> io::Result<Self::File> {
        unimplemented!("read only file system")
    }

    fn create_dir<P>(self, _path: P, _parents: bool) -> io::Result<Self::Dir>
    where
        P: AsRef<Path>,
    {
        unimplemented!("read only file system")
    }

    fn rename<P, Q>(self, _from: P, _to: Q) -> io::Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        unimplemented!("read only file system")
    }

    fn remove<P: AsRef<Path>>(self, _path: P, _children: bool) -> io::Result<()> {
        unimplemented!("read only file system")
    }
}
