use std::io;
use std::path::Path;
use std::mem::size_of;
use std::cmp::min;

use util::SliceExt;
use mbr::MasterBootRecord;
use vfat::{Shared, Cluster, File, Dir, Entry, FatEntry, Error, Status};
use vfat::{BiosParameterBlock, CachedDevice, Partition};
use traits::{FileSystem, BlockDevice};

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
        let fat32 = mbr.first_fat32_partition().expect(
            "FAT32 partition is found.",
        );
        let bpb = BiosParameterBlock::from(&mut device, fat32.relative_sector as u64)?;

        let bps = bpb.bytes_per_sector;
        let spc = bpb.sectors_per_cluster;
        let spf = bpb.sectors_per_fat;
        let fss = fat32.relative_sector as u64 /* start of partition */ + 1 /* BPB */ + bpb.number_of_reserved_sectors as u64;
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
            data_start_sector: fss as u64 +
                bpb.number_of_fats as u64 * bpb.number_of_sectors_per_fat as u64,
            root_dir_cluster: rdc,
        };
        println!("{:#?}", vfat);
        Ok(Shared::new(vfat))
    }

    // TODO: The following methods may be useful here:
    //
    //  * A method to read from an offset of a cluster into a buffer.
    //
    //    fn read_cluster(
    //        &mut self,
    //        cluster: Cluster,
    //        offset: usize,
    //        buf: &mut [u8]
    //    ) -> io::Result<usize>;
    //
    //  * A method to read all of the clusters chained from a starting cluster
    //    into a vector.
    //
    //    fn read_chain(
    //        &mut self,
    //        start: Cluster,
    //        buf: &mut Vec<u8>
    //    ) -> io::Result<usize>;
    //
    //  * A method to return a reference to a `FatEntry` for a cluster where the
    //    reference points directly into a cached sector.
    //
    //    fn fat_entry(&mut self, cluster: Cluster) -> io::Result<&FatEntry>;
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
