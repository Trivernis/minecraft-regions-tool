use colored::*;
use env_logger::Env;
use log::Level;
use minecraft_regions_tool::world_folder::WorldFolder;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Opt {
    /// Path to the world folder
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    /// Forces verbose output
    #[structopt(short, long)]
    verbose: bool,

    #[structopt(subcommand)]
    sub_command: SubCommand,
}

#[derive(StructOpt, Debug)]
#[structopt()]
enum SubCommand {
    /// Return the total number of chunks in the world
    Count,

    /// Scan for errors in the region files and optionally fix them
    Scan(ScanOptions),
}

#[derive(StructOpt, Debug)]
#[structopt()]
struct ScanOptions {
    #[structopt(short, long)]
    fix: bool,
}

fn main() {
    let opt: Opt = Opt::from_args();
    build_logger(opt.verbose);
    let world = WorldFolder::new(opt.input);
    match opt.sub_command {
        SubCommand::Count => log::info!("Chunk Count: {}", world.count_chunks().unwrap()),
        SubCommand::Scan(opt) => {
            if opt.fix {
                log::info!("Fixing fixable errors.");
            }
            log::info!("Scanning Region files for errors...");
            log::info!("Scan Results:\n{}", world.scan_files(opt.fix).unwrap())
        }
    }
}

fn build_logger(verbose: bool) {
    env_logger::Builder::from_env(Env::default().default_filter_or(if verbose {
        "debug"
    } else {
        "info"
    }))
    .format(|buf, record| {
        use std::io::Write;
        let color = get_level_style(record.level());
        writeln!(
            buf,
            "{}: {}",
            record
                .level()
                .to_string()
                .to_lowercase()
                .as_str()
                .color(color),
            record.args()
        )
    })
    .init();
}

fn get_level_style(level: Level) -> colored::Color {
    match level {
        Level::Trace => colored::Color::Magenta,
        Level::Debug => colored::Color::Blue,
        Level::Info => colored::Color::Green,
        Level::Warn => colored::Color::Yellow,
        Level::Error => colored::Color::Red,
    }
}
