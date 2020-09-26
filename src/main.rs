use minecraft_regions_tool::world_folder::WorldFolder;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Opt {
    /// Path to the world folder
    #[structopt(parse(from_os_str))]
    input: PathBuf,

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
    let world = WorldFolder::new(opt.input);
    match opt.sub_command {
        SubCommand::Count => println!("Chunk Count: {}", world.count_chunks().unwrap()),
        SubCommand::Scan(opt) => world.scan_files(opt.fix).unwrap(),
    }
}
