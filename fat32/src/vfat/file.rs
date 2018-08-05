use std::cmp::{max, min};
use std::io::{self, Seek, SeekFrom};

use traits;
use vfat::{Cluster, Metadata, Shared, VFat};

#[derive(Debug)]
pub struct File {
    pub name: String,
    pub metadata: Metadata,
    pub size: u32,
    first_cluster: Cluster,
    vfat: Shared<VFat>,
    offset: u32,
}

impl File {
    pub fn new(
        name: String,
        metadata: Metadata,
        size: u32,
        first_cluster: Cluster,
        vfat: Shared<VFat>,
    ) -> File {
        File {
            name,
            metadata,
            size,
            first_cluster,
            vfat,
            offset: 0,
        }
    }
}

impl io::Seek for File {
    /// Seek to offset `pos` in the file.
    ///
    /// A seek to the end of the file is allowed. A seek _beyond_ the end of the
    /// file returns an `InvalidInput` error.
    ///
    /// If the seek operation completes successfully, this method returns the
    /// new position from the start of the stream. That position can be used
    /// later with SeekFrom::Start.
    ///
    /// # Errors
    ///
    /// Seeking before the start of a file or beyond the end of the file results
    /// in an `InvalidInput` error.
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let offset = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::End(offset) => {
                let offset = self.size as i64 + offset;
                if offset < 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Should not seek before 0.",
                    ));
                }
                offset as u64
            }
            SeekFrom::Current(offset) => {
                let offset = self.offset as i64 + offset;
                if offset < 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Should not seek before 0.",
                    ));
                }
                offset as u64
            }
        };
        if offset > self.size as u64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Should not seek beyond end.",
            ));
        }
        self.offset = offset as u32; // Works rely on the fact that maximum file size is 2**32 bits.
        Ok(offset)
    }
}

impl io::Write for File {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unimplemented!("Read-only!")
    }

    fn flush(&mut self) -> io::Result<()> {
        unimplemented!("Read-only")
    }
}

impl io::Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // io::Read does not need all octets are returned at once
        let read_bytes = {
            let mut vfat = self.vfat.borrow_mut();
            let cluster = self.offset / vfat.cluster_size() as u32;
            let offset_in_cluster = self.offset as usize % vfat.cluster_size();
            let available_bytes = (self.size - self.offset) as usize;
            let len = min(available_bytes, buf.len());
            vfat.read_cluster(
                cluster.into(),
                offset_in_cluster,
                &mut buf[..len],
            )?
        };
        self.seek(SeekFrom::Current(read_bytes as i64))?;
        Ok(read_bytes)
    }
}

impl traits::File for File {
    /// Writes any buffered data to disk.
    fn sync(&mut self) -> io::Result<()> {
        unimplemented!("Read-only!");
    }

    /// Returns the size of the file in bytes.
    fn size(&self) -> u64 {
        self.size as u64
    }
}
