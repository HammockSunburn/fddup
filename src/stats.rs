// Copyright (c) 2021 Hammock Sunburn
//
// Permission is hereby granted, free of charge, to any person obtaining
// a copy of this software and associated documentation files (the
// "Software"), to deal in the Software without restriction, including
// without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to
// permit persons to whom the Software is furnished to do so, subject to
// the following conditions:
//
// The above copyright notice and this permission notice shall be
// included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
// LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
// WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use crate::possdupe::PossDupe;

use anyhow::{anyhow, Result};
use humansize::{file_size_opts, FileSize};
use num_traits::cast::ToPrimitive;

// Return human-readable string representing a number of bytes.
fn to_human_readable<T: FileSize>(size: T) -> Result<String> {
    size.file_size(file_size_opts::BINARY)
        .map_err(|e| anyhow!(e))
}

// Return human-readable string representing a percentage.
fn to_percentage<T: ToPrimitive>(numerator: T, denominator: T) -> String {
    let n = ToPrimitive::to_f64(&numerator).unwrap();
    let d = ToPrimitive::to_f64(&denominator).unwrap();

    format!("{:.1}%", (n / d) * 100.0)
}

pub struct Stats {
    // Size of all files we might read
    total_bytes_considered: u64,

    // Number of bytes actually read from the file set
    total_bytes_read: u64,

    // Number of bytes we chose to not read because the files were unique in size or
    // because digests differed in the files before being completely read
    total_bytes_skipped: u64,

    // Number of duplicate files; note that if two files are identical, this is
    // counted here as two
    num_duplicate_files: usize,

    // Number of unique files; this includes files which had a unique size or files
    // which had a non-unique size, but differing hash
    num_unique_files: usize,

    // Number of files which were read, but only partially so
    num_files_partially_read: usize,

    // Number of files which were read in their entirety
    num_files_fully_read: usize,

    // Number of files which had 0 bytes read because they had a unique size
    num_files_not_read: usize,
}

impl Stats {
    pub fn new() -> Stats {
        Stats {
            total_bytes_considered: 0,
            total_bytes_read: 0,
            total_bytes_skipped: 0,
            num_duplicate_files: 0,
            num_unique_files: 0,
            num_files_partially_read: 0,
            num_files_fully_read: 0,
            num_files_not_read: 0,
        }
    }

    pub fn unique(&mut self, pd: &PossDupe) {
        self.num_unique_files += 1;
        self.track(pd);
    }

    pub fn duplicate(&mut self, pd: &PossDupe) {
        self.num_duplicate_files += 1;
        self.track(pd);
    }

    fn track(&mut self, pd: &PossDupe) {
        self.total_bytes_considered += pd.file_len;
        self.total_bytes_read += pd.bytes_read;
        self.total_bytes_skipped += pd.bytes_remaining();

        if pd.bytes_read > 0 {
            if pd.bytes_remaining() == 0 {
                self.num_files_fully_read += 1;
            } else {
                self.num_files_partially_read += 1;
            }
        } else {
            self.num_files_not_read += 1;
        }
    }

    pub fn display(&self) -> Result<()> {
        let total_files = self.num_duplicate_files + self.num_unique_files;

        eprintln!(
            "{} files: {} duplicate ({}), {} unique ({})",
            self.num_duplicate_files + self.num_unique_files,
            self.num_duplicate_files,
            to_percentage(self.num_duplicate_files, total_files),
            self.num_unique_files,
            to_percentage(self.num_unique_files, total_files)
        );

        eprintln!(
            "{} bytes: {} read ({}), {} skipped ({})",
            to_human_readable(self.total_bytes_considered)?,
            to_human_readable(self.total_bytes_read)?,
            to_percentage(self.total_bytes_read, self.total_bytes_considered),
            to_human_readable(self.total_bytes_skipped)?,
            to_percentage(self.total_bytes_skipped, self.total_bytes_considered)
        );

        eprintln!(
            "{} files partially read ({}), {} files fully read ({}), {} files skipped ({})",
            self.num_files_partially_read,
            to_percentage(self.num_files_partially_read, total_files),
            self.num_files_fully_read,
            to_percentage(self.num_files_fully_read, total_files),
            self.num_files_not_read,
            to_percentage(self.num_files_not_read, total_files)
        );

        Ok(())
    }
}
