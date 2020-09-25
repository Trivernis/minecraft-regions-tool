use crate::region_file::RegionFile;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::path::PathBuf;

pub struct WorldFolder {
    path: PathBuf,
}

impl WorldFolder {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn count_chunks(&self) -> io::Result<u64> {
        let mut count = 0u64;
        let region_file_path = self.path.join(PathBuf::from("region"));

        for file in fs::read_dir(region_file_path)? {
            let file_path = file?.path();
            if file_path.extension() == Some(OsStr::new("mca")) {
                let f = File::open(file_path)?;
                let region_file = RegionFile::new(Box::new(BufReader::new(f)))?;
                count += region_file.count_chunks() as u64;
            }
        }

        Ok(count)
    }
}
