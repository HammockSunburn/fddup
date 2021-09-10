// Copyright (c) 2021 Hammock Sunburn
//
// Permission is hereby granted, free of charge, to any person obtaining
// a copy of this software and associated documentation files (the
// "Software"), to deal in the Software without restriction, including
// without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to
// permit persons to whom the Software is furnished to do so, subject to
// the following conditions:

// The above copyright notice and this permission notice shall be
// included in all copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
// LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
// WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use anyhow::{Context, Result};
use clap::{App, Arg};
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::Path;

pub const MAX_READ_BUFFER_SIZE: usize = 512 * 1024;

pub struct Options {
    pub files: Option<String>,
    pub output: Option<String>,
    pub skip_empty: bool,
    pub verbose: bool,
    pub show_size: bool,
    pub read_size: usize,
    pub num_threads: usize,
}

const OPTION_FILES: &str = "files";
const OPTION_OUTPUT: &str = "output";
const OPTION_SKIP_EMPTY: &str = "skip-empty";
const OPTION_VERBOSE: &str = "verbose";
const OPTION_SHOW_SIZE: &str = "show-size";
const OPTION_READ_SIZE: &str = "read-size";
const OPTION_THREADS: &str = "threads";

impl Options {
    pub fn parse() -> Options {
        let default_threads = num_cpus::get().to_string();
        let default_read_size = MAX_READ_BUFFER_SIZE.to_string();

        let matches = App::new("fddup")
            .version("1.0.3")
            .author("Hammock Sunburn <hammocksunburn@gmail.com>")
            .about("Find duplicate files")
            .arg(
                Arg::with_name(OPTION_FILES)
                    .short("f")
                    .long("files")
                    .value_name("FILENAME")
                    .help("List of files to be checked for duplicates; if not specified, filenames are read from STDIN")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name(OPTION_OUTPUT)
                .short("o")
                .long("output")
                .value_name("FILE")
                .help("Output duplicate filenames and hashes to this file; if not specified, outputs to STDOUT")
                .takes_value(true)
            )
            .arg(
                Arg::with_name(OPTION_SKIP_EMPTY)
                .short("e")
                .long("skip-empty")
                .help("skip empty (0-length) files")
            )
            .arg(
                Arg::with_name(OPTION_VERBOSE)
                .short("v")
                .long("verbose")
                .help("show extra information (#of files, bytes read, etc.)")
            )
            .arg(
                Arg::with_name(OPTION_SHOW_SIZE)
                .short("z")
                .long("show-size")
                .help("show size of duplicate files in addition to hash and filename")
            )
            .arg(
                Arg::with_name(OPTION_READ_SIZE)
                .short("s")
                .long("read-size")
                .value_name("BYTES")
                .help("Size of file read operations, in bytes")
                .default_value(&default_read_size)
                .takes_value(true)
            )
            .arg(
                Arg::with_name(OPTION_THREADS)
                .short("j")
                .long("threads")
                .value_name("NUM")
                .help("Number of threads to use for performing work")
                .default_value(default_threads.as_str())
                .takes_value(true)
            )
            .get_matches();

        let files = matches.value_of(OPTION_FILES).map(String::from);
        let output = matches.value_of(OPTION_OUTPUT).map(String::from);
        let skip_empty = matches.is_present(OPTION_SKIP_EMPTY);
        let verbose = matches.is_present(OPTION_VERBOSE);
        let show_size = matches.is_present(OPTION_SHOW_SIZE);

        let read_size = matches
            .value_of(OPTION_READ_SIZE)
            .unwrap()
            .parse::<usize>()
            .unwrap();

        if read_size > MAX_READ_BUFFER_SIZE {}

        let num_threads = matches
            .value_of(OPTION_THREADS)
            .unwrap()
            .parse::<usize>()
            .unwrap();

        Options {
            files,
            output,
            skip_empty,
            verbose,
            show_size,
            read_size,
            num_threads,
        }
    }
}

// Return an input stream from a file or from stdin, depending on the specified command
// line arguments.
pub fn input_stream(options: &Options) -> Result<Box<dyn BufRead>> {
    let reader: Box<dyn BufRead> = match &options.files {
        None => Box::new(BufReader::new(io::stdin())),
        Some(filename) => {
            Box::new(BufReader::new(File::open(filename).with_context(|| {
                format!("failed to read input file {}", filename)
            })?))
        }
    };

    Ok(reader)
}

// Return a buffered output writer to a file or to stdout, depending on the specified
// command line arguments.
pub fn output_writer(options: &Options) -> Result<BufWriter<Box<dyn Write>>> {
    let write: Box<dyn Write> = match options.output {
        Some(ref output) => Box::new(
            File::create(&Path::new(output))
                .with_context(|| format!("couldn't create output file {}", output))?,
        ),
        None => Box::new(io::stdout()),
    };

    Ok(BufWriter::new(write))
}
