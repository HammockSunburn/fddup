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

use crate::algo;
use crate::cli::Options;
use crate::possdupe::PossDupe;

use anyhow::{Context, Result};
use std::cell::RefCell;
use std::cmp::min;
use std::fs::symlink_metadata;
use std::io::{BufRead, Read, Write};

// Read filenames, one per line, from the given `BufRead`. Find some relevant
// data about the file, such as whether it's a symlink or directory, and the
// file's size.
fn stat_files(reader: Box<dyn BufRead>) -> Result<Vec<PossDupe>> {
    let mut result = Vec::new();

    for line in reader.lines() {
        let filename = line.with_context(|| "an input line isn't a valid unicode string")?;
        let attr = symlink_metadata(&filename)
            .with_context(|| format!("couldn't open file to read attributes: {}", filename))?;
        if attr.is_dir() || attr.file_type().is_symlink() {
            continue;
        }

        result.push(PossDupe::new(&filename, attr.len()));
    }

    Ok(result)
}

// Remove any duplicate paths which may have been specified as input.
fn remove_duplicate_paths(poss_dupes: &mut Vec<PossDupe>) {
    poss_dupes.sort_by(|a, b| a.path.cmp(&b.path));
    poss_dupes.dedup_by(|a, b| a.path.eq(&b.path));
}

// Sort our possible duplicates by length and digest snapshot.
fn sort_poss_dupes(poss_dupes: &mut Vec<PossDupe>) {
    poss_dupes.sort_by(|a, b| {
        a.key
            .len
            .cmp(&b.key.len)
            .then_with(|| a.key.digest_snapshot.cmp(&b.key.digest_snapshot))
    });
}

pub struct Fddup {
    options: Options,
    poss_dupes: Vec<PossDupe>,
}

impl Fddup {
    pub fn new(options: Options) -> Fddup {
        Fddup {
            options,
            poss_dupes: Vec::new(),
        }
    }

    pub async fn run(&mut self) {
        match self.run_impl().await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("{}", e)
            }
        }
    }

    async fn run_impl(&mut self) -> Result<()> {
        let reader = crate::cli::input_stream(&self.options)?;
        let mut writer = crate::cli::output_writer(&self.options)?;
        let mut stats = crate::stats::Stats::new();

        self.poss_dupes = stat_files(reader)?;
        remove_duplicate_paths(&mut self.poss_dupes);
        sort_poss_dupes(&mut self.poss_dupes);

        // Keep going as long as we have some possibly duplicate files.
        while !self.poss_dupes.is_empty() {
            // Obtain a group of work equal to the number of configured threads,
            // but we may obtain more files than the number of threads to ensure
            // that all files of the same length are handled by the inner loop.
            let mut w = algo::find_work(&mut self.poss_dupes, self.options.num_threads);

            // Keep going with this group of work as long as there are possible
            // duplicates or confirmed duplicates.
            while !w.work.is_empty() || !w.duplicates.is_empty() || !w.uniques.is_empty() {
                for unique in w.uniques.into_iter() {
                    stats.unique(&unique);
                }

                // Display digest and filenames of any duplicates.
                for duplicate in w.duplicates.into_iter() {
                    stats.duplicate(&duplicate);

                    if self.options.show_size {
                        writer.write_fmt(format_args!(
                            "{}  {}  {}\n",
                            hex::encode(duplicate.key.digest_snapshot),
                            duplicate.file_len,
                            duplicate.path.to_str().unwrap()
                        ))?;
                    } else {
                        writer.write_fmt(format_args!(
                            "{}  {}\n",
                            hex::encode(duplicate.key.digest_snapshot),
                            duplicate.path.to_str().unwrap()
                        ))?;
                    }
                }

                // Create tasks, one per possible duplicate. Each task is spawned
                // and will open the file (if it's not already open), perform a
                // single read of the configured size, and update the digest.
                let mut tasks = Vec::new();

                for pd in w.work.into_iter() {
                    let task = tokio::spawn(read_poss_dupe(pd, self.options.read_size));
                    tasks.push(task);
                }

                let mut results = vec![];

                // Join up with the tasks, tracking the results for each.
                for t in tasks {
                    let pd = tokio::join!(t).0.unwrap().unwrap();
                    results.push(pd);
                }

                sort_poss_dupes(&mut results);

                // Find work again, but only on the subset of work for this loop. Note that we use
                // usize::MAX here rather than the configured number of threads. This is because
                // it's possible that in the earlier call to `find_work` may have obtained more
                // files than the configured number of threads if the number of files for a single
                // size spanned the remaining number of threads.
                w = algo::find_work(&mut results, usize::MAX);
            }
        }

        if self.options.verbose {
            stats.display()?;
        }

        Ok(())
    }
}

thread_local! {
    // Re-use the same buffer for reading in each thread.
    pub static BUFFER: RefCell<[u8; crate::cli::MAX_READ_BUFFER_SIZE]> =
        RefCell::new([0; crate::cli::MAX_READ_BUFFER_SIZE]);
}

// In the thread pool, asynchronously open the file if needed, perform a read operation,
// and hash the data.
async fn read_poss_dupe(mut poss_dupe: PossDupe, read_size: usize) -> Result<PossDupe> {
    poss_dupe.open()?;

    BUFFER.with(|b| {
        let mut buffer = *b.borrow_mut();
        let to_read = min(read_size as u64, poss_dupe.bytes_remaining()) as usize;

        if let Some(file) = &mut poss_dupe.file {
            let bytes_read = file.read(&mut buffer[0..to_read])?;
            assert!(bytes_read == to_read);
            poss_dupe.bytes_read += bytes_read as u64;
            poss_dupe.update_digest(&buffer[0..bytes_read]);
        }

        Ok(poss_dupe)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_pd(path: &str, len: u64) -> PossDupe {
        PossDupe::new(path, len)
    }

    impl PartialEq for PossDupe {
        fn eq(&self, other: &Self) -> bool {
            self.path == other.path
                && self.key.len == other.key.len
                && self.bytes_read == other.bytes_read
        }
    }

    #[test]
    fn remove_duplicate_paths() {
        let mut pd = Vec::new();
        pd.push(mk_pd("a", 10));
        pd.push(mk_pd("b", 10));
        pd.push(mk_pd("a", 10));

        // Shouldn't be possible to have the same path with different lengths
        // unless the length were to change during iteration, but if we do,
        // treat it as a duplicate.
        pd.push(mk_pd("b", 11));

        crate::fddup::remove_duplicate_paths(&mut pd);

        assert_eq!(pd, vec![mk_pd("a", 10), mk_pd("b", 10)]);
    }

    #[test]
    fn sort_poss_dupes() {
        let mut pd = Vec::new();

        // "c" and "d" have been read some
        let mut d = mk_pd("d", 300);
        d.update_digest(&['a' as u8]);
        pd.push(d);

        let mut c = mk_pd("c", 300);
        c.update_digest(&['b' as u8]);
        pd.push(c);

        // "a" and "b" haven't yet been read
        pd.push(mk_pd("a", 100));
        pd.push(mk_pd("b", 200));

        crate::fddup::sort_poss_dupes(&mut pd);

        assert_eq!(
            pd,
            vec![
                mk_pd("a", 100),
                mk_pd("b", 200),
                mk_pd("c", 300),
                mk_pd("d", 300)
            ]
        );
    }
}
