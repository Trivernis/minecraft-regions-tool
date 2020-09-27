use crate::region_file::RegionFile;
use crate::scan::ScanOptions;
use crate::scan::ScanStatistics;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use log::LevelFilter;
use rayon::prelude::*;
use std::fs;
use std::io;
use std::ops::Add;
use std::path::PathBuf;
use std::sync::Arc;

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
            let region_file = RegionFile::new(&file)?;
            count += region_file.count_chunks() as u64;
        }

        Ok(count)
    }

    /// Scans all region files for potential errors
    pub fn scan_files(&self, options: ScanOptions) -> io::Result<ScanStatistics> {
        let paths = self.region_file_paths();
        let bar = ProgressBar::new(paths.len() as u64);
        let options = Arc::new(options);
        bar.set_style(
            ProgressStyle::default_bar().template("\r[{eta_precise}] {wide_bar} {pos}/{len} "),
        );
        if log::max_level() == LevelFilter::Debug {
            bar.set_draw_target(ProgressDrawTarget::hidden())
        }
        bar.enable_steady_tick(1000);

        let statistic: ScanStatistics = paths
            .par_iter()
            .filter_map(|path| {
                log::debug!("Opening and scanning region file {:?}", path);
                let mut region_file = RegionFile::new(path)
                    .map_err(|e| {
                        log::error!("Failed to open region file {:?}: {}\n", path, e);
                        if options.fix_delete {
                            if let Err(e) = fs::remove_file(path) {
                                return e;
                            }
                        }

                        e
                    })
                    .ok()?;

                let result = region_file.scan_chunks(&options).ok()?;
                bar.inc(1);
                log::debug!("Statistics for {:?}:\n{}", path, result);

                Some(result)
            })
            .reduce(|| ScanStatistics::new(), |a, b| a.add(b));

        bar.finish_and_clear();

        Ok(statistic)
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
