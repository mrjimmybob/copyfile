use ansi_term::Colour;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{BufReader, BufWriter, Read, Seek, Write};
use std::path::Path;
// use std::path::PathBuf;
use std::env;
use std::process;
use std::time::Instant;
use std::vec::Vec;

const BLOCK_SIZE: usize = 4096; // 4KB block size

#[derive(Clone)]
struct Options {
    ascii: bool,
    verbose: bool,
    progress: bool,
    force: bool,
    move_file: bool,
    resume: bool,
    source: String,
    destination: String,
}

fn concatenate_path(path: &str, filename: &str) -> String {
    Path::new(path).join(filename).to_str().unwrap().to_string()
}

fn is_directory(path: &str) -> bool {
    Path::new(path).is_dir()
}

fn file_exists(path: &str) -> bool {
    Path::new(path).exists()
}

fn get_file_size(file_path: &str) -> io::Result<u64> {
    let metadata = fs::metadata(file_path)?;
    Ok(metadata.len())
}

fn get_file_name(path: &str) -> Result<String, std::io::Error> {
    let file_name = Path::new(path)
        .file_name()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid path"))?;
    let file_name = file_name.to_str().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid file name")
    })?;
    Ok(file_name.to_string())
}

fn remove_source(path: &str) -> Result<(), std::io::Error> {
    fs::remove_file(path)
}

fn print_file_size(file_path: &str) -> io::Result<()> {
    let size = get_file_size(file_path)?;
    let unit;
    let size_f64;

    if size < 1024 {
        unit = "bytes";
        size_f64 = size as f64;
    } else if size < 1024 * 1024 {
        unit = "Kb";
        size_f64 = size as f64 / 1024.0;
    } else if size < 1024 * 1024 * 1024 {
        unit = "Mb";
        size_f64 = size as f64 / (1024.0 * 1024.0);
    } else if size < 1024 * 1024 * 1024 * 1024 {
        unit = "Gb";
        size_f64 = size as f64 / (1024.0 * 1024.0 * 1024.0);
    } else {
        unit = "Tb";
        size_f64 = size as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0);
    }

    let size = format!("{:.2} {}", size_f64, unit);
    println!(
        "        {} {}",
        Colour::Yellow.paint("File size:"),
        Colour::Cyan.paint(size)
    );
    Ok(())
}

fn print_help() {
    let mayor_version: u32 = 1;
    let minor_version: u32 = 0;
    let revision: u32 = 0;

    println!(
        "{}",
        Colour::Cyan.paint("Copy a file from source to destination.")
    );
    println!("");
    println!("{}", "Usage:");
    println!(
        "\t{} [{}] [{}] [{}] [{}] [{}] {} {}",
        Colour::Yellow.paint("copyfile"),
        Colour::Blue.paint("-v"),
        Colour::Blue.paint("-a"),
        Colour::Blue.paint("-f"),
        Colour::Blue.paint("-p"),
        Colour::Blue.paint("-r"),
        Colour::Blue.paint("source"),
        Colour::Blue.paint("destination")
    );
    println!(
        "\t{} {}",
        Colour::Yellow.paint("copyfile"),
        Colour::Blue.paint("-h")
    );
    println!("{}", Colour::Cyan.paint("Options:"));
    println!(
        "\t{}\t\t{}",
        Colour::Cyan.paint("-v | --verbose"),
        Colour::Cyan.paint("Be verbose.")
    );
    println!(
        "\t{}\t\t{}",
        Colour::Cyan.paint("-p | --progress"),
        Colour::Cyan.paint("Show a progress bar.")
    );
    println!(
        "\t{}\t\t{}",
        Colour::Cyan.paint("-a | --ascii"),
        Colour::Cyan.paint("Use ascii characters instead of UTF-8 for the progress bar.")
    );
    println!(
        "\t{}\t\t{}",
        Colour::Cyan.paint("-f | --force"),
        Colour::Cyan.paint("Force overwrite if file exists at destination.")
    );
    println!(
        "\t{}\t\t{}",
        Colour::Cyan.paint("-r | --resume"),
        Colour::Cyan.paint("Resume copy if destination file exists.")
    );
    println!(
        "\t{}\t\t{}",
        Colour::Cyan.paint("-h | --help"),
        Colour::Cyan.paint("Show this help.")
    );
    println!(
        "{} {}{}",
        Colour::Yellow.paint("Third"),
        Colour::Green.paint("3"),
        Colour::Yellow.paint("ye Software Inc. © 2024")
    );
    println!("Version: {}.{}.{}.", mayor_version, minor_version, revision);
}

// fn print_success(options: &Options) {
//     if options.verbose {
//         let msg = format!("File {} successfully!", if options.move_file { "moved" } else { "copied" } );
//         println!("");
//         println!("{}", Colour::Green.paint(msg));
//     }
// }

fn print_failure(options: &Options, error: String) {
    if options.verbose {
        println!("");
        println!(
            "{}{}",
            Colour::Red.paint("Error copying file: "),
            Colour::Red.paint(error)
        );
    }
}

fn print_error(options: &Options, e: std::io::Error) {
    if options.verbose {
        let msg = format!(
            "Error {} file: ",
            if options.move_file {
                "moving"
            } else {
                "copying"
            }
        );
        println!("");
        println!(
            "{}{}",
            Colour::Red.paint(msg),
            Colour::Red.paint(e.to_string())
        );
    }
}

fn print_header(options: &Options) {
    if options.verbose {
        let msg = format!(
            "{} file from:",
            if options.move_file {
                " Moving"
            } else {
                "Copying"
            }
        );
        println!(
            "{} {}",
            Colour::Yellow.paint(msg),
            Colour::Cyan.paint(options.source.as_str())
        );
        println!(
            "{} {}",
            Colour::Yellow.paint("               to:"),
            Colour::Cyan.paint(options.destination.as_str())
        );
        match print_file_size(options.source.as_str()) {
            Ok(_) => (),
            Err(e) => println!(
                "{} {}",
                Colour::Red.paint("Error:"),
                Colour::Red.paint(e.to_string())
            ),
        }
        if options.force {
            println!(
                "{} {}",
                Colour::Yellow.paint("  Force overwrite:"),
                Colour::Cyan.paint("true")
            );
        }
        if options.resume {
            println!(
                "{} {}",
                Colour::Yellow.paint("           Resume:"),
                Colour::Cyan.paint("true")
            );
        }
        println!("");
    }
}

fn progress_header(options: &Options) {
    if options.progress {
        println!("{}", Colour::Yellow.paint("0        10        20        30        40        50        60        70        80        90       100"));
        if options.ascii {
            println!("{}", Colour::Yellow.paint("|--------|---------|---------|---------|---------|---------|---------|---------|---------|---------|"));
        } else {
            println!("{}", Colour::Yellow.paint("┣┯┯┯┯┯┯┯┯╋┯┯┯┯┯┯┯┯┯╋┯┯┯┯┯┯┯┯┯╋┯┯┯┯┯┯┯┯┯╋┯┯┯┯┯┯┯┯┯╋┯┯┯┯┯┯┯┯┯╋┯┯┯┯┯┯┯┯┯╇┯┯┯┯┯┯┯┯┯╋┯┯┯┯┯┯┯┯┯╋┯┯┯┯┯┯┯┯┯┫"));
        }
    }
}

fn copy_file(options: &Options) -> std::io::Result<()> {
    print_header(&options);
    progress_header(&options);

    let start = Instant::now();
    let file_size = get_file_size(options.source.as_str())?;
    let mut src_file = File::open(options.source.as_str())?;
    let mut dst_opts = OpenOptions::new();
    dst_opts.write(true).create(true);
    let already_copied: u64;
    if options.resume && file_exists(options.destination.as_str()) {
        already_copied = get_file_size(options.destination.as_str())?;
        if already_copied > file_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Destination file is larger than source",
            ));
        }
        src_file.seek(io::SeekFrom::Start(already_copied))?;
        dst_opts.append(true);
    } else {
        already_copied = 0;
        dst_opts.truncate(true);
    }
    let dst_file = dst_opts.open(options.destination.as_str())?;
    let mut reader = BufReader::with_capacity(BLOCK_SIZE, src_file);
    let mut writer = BufWriter::with_capacity(BLOCK_SIZE, dst_file);
    let mut buffer = [0; BLOCK_SIZE];
    let mut col = 0;
    let mut acum = already_copied as usize;

    let acrac = if options.ascii { "#" } else { "▒" }; // ░▒▓█
    if options.progress && already_copied > 0 {
        let start_col: u64 = (already_copied * 100) / file_size;
        while (col as u64) < start_col {
            print!("{}", Colour::Cyan.dimmed().paint(acrac));
            io::stdout().flush().unwrap();
            col = col + 1;
        }
    }

    while let Ok(n) = reader.read(&mut buffer) {
        if options.progress {
            let val: u64 = (acum as u64 * 100) / file_size;
            if val > col {
                print!("{}", Colour::Cyan.paint(acrac));
                io::stdout().flush().unwrap();
                col = col + 1;
            }
        }

        if n == 0 {
            break;
        }
        writer.write_all(&buffer[..n])?;
        acum = acum + n;
    }
    if options.progress && col < 100 {
        while col < 100 {
            let acrac = if options.ascii { "#" } else { "▒" }; // ░▒▓█
            print!("{}", Colour::Cyan.paint(acrac));
            io::stdout().flush().unwrap();
            col = col + 1;
        }
    }
    writer.flush()?;

    if options.verbose {
        let end = Instant::now();
        let duration = end - start;
        let time = format!("{:.2?}", duration);
        println!("");
        println!("");
        println!(
            "{}",
            Colour::Green.paint(format!(
                "{} file succesful in {}",
                if options.move_file { "Move" } else { "Copy" },
                time
            ))
        );
    }

    Ok(())
}

fn main() {
    let mut options = Options {
        ascii: false,
        verbose: false,
        progress: false,
        force: false,
        move_file: false,
        resume: false,
        source: String::from(""),
        destination: String::from(""),
    };

    let mut error: String = String::from("No error specified");
    let mut got_src = false;
    let mut got_dst = false;

    let args: Vec<String> = env::args().collect();
    if args.len() >= 2 {
        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match &arg[..] {
                "-h" | "--help" => {
                    print_help();
                    process::exit(0x0100);
                }
                "-v" | "--verbose" => {
                    options.verbose = true;
                }
                "-p" | "--progress" => {
                    options.progress = true;
                }
                "-a" | "--ascii" => {
                    options.ascii = true;
                }
                "-f" | "--force" => {
                    options.force = true;
                }
                "-m" | "--move" => {
                    options.move_file = true;
                }
                "-r" | "--resume" => {
                    options.resume = true;
                }
                x => {
                    if false == got_src {
                        options.source = String::from(x);
                        got_src = true;
                    } else {
                        options.destination = String::from(x);
                        got_dst = true;
                    }
                }
            }
        }
    } else {
        error = String::from("You need to give me at least a file and a destination, RTFM!");
    }

    if got_src && got_dst {
        let mut proceed = false;
        if !file_exists(options.source.as_str()) {
            error = String::from("Source file not found. I can't copy what I can't find.");
        } else if is_directory(options.destination.as_str()) {
            match get_file_name(options.source.as_str()) {
                Ok(file_name) => {
                    options.destination =
                        concatenate_path(options.destination.as_str(), file_name.as_str());
                    proceed = true
                }
                Err(e) => print_error(&options, e),
            }
        } else {
            proceed = true
        }
        if true == proceed {
            if file_exists(options.destination.as_str()) && !options.force {
                print_failure(&options, String::from("The destination already exists."));
            } else {
                match copy_file(&options) {
                    Ok(_) => {
                        if options.move_file {
                            match remove_source(&options.source.as_str()) {
                                Ok(()) => (),
                                Err(e) => print_error(&options, e),
                            }
                        }
                    }
                    Err(e) => print_error(&options, e),
                }
            }
        } else {
            print_failure(&options, error);
        }
    } else {
        if !got_src {
            error = String::from("No source file given. What do you want to copy?.");
        } else if !got_dst {
            error = String::from("No destination given. Where do you want to copy your file ?.");
        }

        print_failure(&options, error);
    }
}
