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

use crate::algo::GetKey;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::path::PathBuf;

// Key used for sorting possible duplicate files consisting of the file's length
// and its hash (digest) of data read thus far.
#[derive(Debug, Clone)]
pub struct Key {
    /// Length (in bytes) of this file
    pub len: u64,

    /// Digest computed thus far; starts out as all 0s
    pub digest_snapshot: [u8; 32],
}

impl Key {
    pub fn new(len: u64) -> Key {
        Key {
            len,
            digest_snapshot: [0; 32],
        }
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len && self.digest_snapshot == other.digest_snapshot
    }
}

// A single file which may or may not be a duplicate of another file.
#[derive(Debug)]
pub struct PossDupe {
    pub path: PathBuf,
    pub key: Key,
    pub file_len: u64,
    pub bytes_read: u64,

    // File will be lazily opened if and when we need to read from it
    pub file: Option<File>,

    digest: Sha256,
}

impl GetKey<Key> for PossDupe {
    fn key(&self) -> Key {
        self.key.clone()
    }

    fn bytes_remaining(&self) -> u64 {
        self.bytes_remaining()
    }
}

impl PossDupe {
    pub fn new(path: &str, file_len: u64) -> PossDupe {
        PossDupe {
            path: PathBuf::from(path),
            key: Key::new(file_len),
            file_len,
            bytes_read: 0,
            file: None,
            digest: Sha256::new(),
        }
    }

    pub fn open(&mut self) -> Result<()> {
        if self.file.is_none() {
            self.file = Some(File::open(&self.path).with_context(|| {
                format!("couldn't open {} for reading", self.path.to_str().unwrap())
            })?);
        }

        Ok(())
    }

    pub fn bytes_remaining(&self) -> u64 {
        self.key.len.saturating_sub(self.bytes_read)
    }

    pub fn update_digest(&mut self, buffer: &[u8]) {
        self.digest.update(&buffer);
        self.key
            .digest_snapshot
            .clone_from_slice(self.digest.clone().finalize().as_slice())
    }
}
