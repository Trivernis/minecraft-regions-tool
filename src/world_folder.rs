use crate::region_file::RegionFile;
use crate::scan::ScanStatistics;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{BufReader, BufWriter};
use std::ops::Add;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct WorldFolder {
    path: PathBuf,
}

impl WorldFolder {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Counts all chunks of a world
    pub fn count_chunks(&self) -> io::Result<u64> {
        let mut count = 0u64;

        for file in self.region_file_paths() {
            let f = File::open(file)?;
            let region_file = RegionFile::new(BufReader::new(f))?;
            count += region_file.count_chunks() as u64;
        }

        Ok(count)
    }

    pub fn scan_files(&self, fix: bool) -> io::Result<()> {
        let paths = self.region_file_paths();
        let bar = Arc::new(Mutex::new(ProgressBar::new(paths.len() as u64)));
        bar.lock().unwrap().set_style(
            ProgressStyle::default_bar().template("[{eta_precise}] {wide_bar} {pos}/{len} "),
        );

        let statistic: ScanStatistics = paths
            .par_iter()
            .filter_map(|file| {
                let f = OpenOptions::new().read(true).open(file).ok()?;
                let mut region_file = RegionFile::new(BufReader::new(f)).ok()?;

                let result = region_file.scan_chunks().ok()?;
                if fix {
                    let f = OpenOptions::new().write(true).open(file).ok()?;
                    let mut writer = BufWriter::new(f);
                    region_file.write(&mut writer).ok()?;
                }
                bar.lock().unwrap().inc(1);

                Some(result)
            })
            .reduce(|| ScanStatistics::new(), |a, b| a.add(b));

        bar.lock().unwrap().finish_and_clear();

        println!("{}", statistic);

        Ok(())
    }

    /// Returns a list of region file paths for the world folder
    fn region_file_paths(&self) -> Vec<PathBuf> {
        let region_file_path = self.path.join(PathBuf::from("region"));

        fs::read_dir(region_file_path)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .collect()
    }
}
