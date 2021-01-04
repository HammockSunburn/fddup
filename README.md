# fddup

Pull requests welcome! I've been writing a lot of solo Rust code for a while now (and reading
the code of seasoned Rustaceans), but would welcome code review and pull requests for changes
to make the code more idiomatic.

## Overview

`fddup` aims to be a very fast multi-threaded cross-platform duplicate file finder.

`fddup` will only consider two files identical if their size is identical and their hashes are
identical. But, rather than simply hashing the entire file, `fddup` will hash it piecemeal in
chunks and, if any hashed chunk differs between files, then no further hashing of the file will
occur and they will be considered different.

This method can significantly reduce the number of operations where most files are of different
lengths or for those that are of the same length, it's they often differ early on in the file.

`fddup` provides no functionality to modify data, it merely reports on what it finds. `fddup`
will exit on any kind of failure (e.g., read permission issue) with a hopefully useful error
message and non-zero exit status to ensure that failures are obvious.

## Warning

I've used this program extensively to deduplicate a large collection of digital video and
imagery and have found it to be reliable and to produce correct results. But, there's no warranty
here, if you use the output of `fddup` to, for example, make choices about files to delete, I'd
suggest you perform your own additional testing to ensure it's working as it should.

## Usage

`fddup` accepts a list of files, one per line, from standard input or by specifying a file
containing a list of filenames with the `-f` or `--files` command line option. Rather than having
its own logic to walk directory trees, specify globs, and find files, `fddup` allows you to use
your favorite tool for doing so. I'd recommend using [fd](https://github.com/sharkdp/fd) which is
available in Linux distributions and simpler to use than the traditional UNIX `find` command.

```text
fddup 1.0
Hammock Sunburn <hammocksunburn@gmail.com>
Find duplicate files

USAGE:
    fddup [FLAGS] [OPTIONS]

FLAGS:
    -h, --help          Prints help information
    -z, --show-size     show size of duplicate files in addition to hash and filename
    -e, --skip-empty    skip empty (0-length) files
    -V, --version       Prints version information
    -v, --verbose       show extra information (#of files, bytes read, etc.)

OPTIONS:
    -f, --files <FILENAME>     List of files to be checked for duplicates; if not specified, filenames are read from
                               STDIN
    -o, --output <FILE>        Output duplicate filenames and hashes to this file; if not specified, outputs to STDOUT
    -s, --read-size <BYTES>    Size of file read operations, in bytes [default: 4096]
    -j, --threads <NUM>        Number of threads to use for performing work [default: 64]
```

An example usage, using `fd`, could look like this:

```shell
fd '.jpg' /mnt/my_data | fddup -v -e
```

### Output

The output from `fddup` will consist of one line on standard output for each file which is a
duplicate of another file along with the SHA256 digest of the file. For example:

```text
..
..
c236c5dcedd77ba32042d49b7c20b730a7aa9b4bd7f24916683c6b4403ad6b05  /path/file/file/filename1.txt
c236c5dcedd77ba32042d49b7c20b730a7aa9b4bd7f24916683c6b4403ad6b05  /path/to/other/file/filename2.txt
c236c5dcedd77ba32042d49b7c20b730a7aa9b4bd7f24916683c6b4403ad6b05  /path/to/another/file/filename3.txt
a6deef70588f823996f05aa813f9c228feaadc99fb275fd228a0647f61660c4a  /more/files/filename4.txt
a6deef70588f823996f05aa813f9c228feaadc99fb275fd228a0647f61660c4a  /more/files/filename5.txt
..
..
```

In the excerpt above, we can see that `filename1.txt`, `filename2.txt`, and `filename3.txt` are
files with the same size and same SHA256 digest. Also, `filename4.txt` and `filename5.txt` are
duplicates of one another.

Command line option `-z` (`--show-size`) will provide a second column which shows the size of each
duplicate file:

```text
..
..
c236c5dcedd77ba32042d49b7c20b730a7aa9b4bd7f24916683c6b4403ad6b05  302551  /path/file/file/filename1.txt
c236c5dcedd77ba32042d49b7c20b730a7aa9b4bd7f24916683c6b4403ad6b05  302551  /path/to/other/file/filename2.txt
c236c5dcedd77ba32042d49b7c20b730a7aa9b4bd7f24916683c6b4403ad6b05  302551  /path/to/another/file/filename3.txt
a6deef70588f823996f05aa813f9c228feaadc99fb275fd228a0647f61660c4a  185221  /more/files/filename4.txt
a6deef70588f823996f05aa813f9c228feaadc99fb275fd228a0647f61660c4a  185221  /more/files/filename5.txt
..
..
```

You may use `-o` (`--output`) to write the output from the command to a file instead of standard
output.

You may choose to skip zero-length (empty) files from being considered by `fddup` with the
`-e` (`--skip-empty`) option.

To obtain extra statistics about the operations performed by `fddup`, you may use the `-v`
(`--verbose`) option. Extra information will be written to standard error. For example:

```text
136761 files: 110798 duplicate (81.0%), 25963 unique (19.0%)
1.73 GiB bytes: 1.26 GiB read (73.2%), 473.87 MiB skipped (26.8%)
10008 files partially read (7.3%), 119318 files fully read (87.2%), 7435 files skipped (5.4%)
```

The first line shows the number of files considered by `fddup` (136761) and the number which were
determined to be duplicates (110798) and the number which were unique (25963). Note that if two
files were found to be duplicates of one another, this counts as two in the duplicate count, not
one.

On the second line, the total size of the files considered is shown (1.73 GiB), the number of
bytes which were read by `fddup` (1.26 GiB) and the number of bytes which were skipped (473.87 MiB)
because `fddup` didn't need to read them. For example, if there is only a single file of a given
size, the entire file can be skipped. If two files have the same size, but their digests differ
early in the file, the remainders of those files may be skipped.

Finally, the third line shows the number of files which were read in some form by `fddup`. Some
files may be partially read (10008) because they had the same size, but their digests differed
early in the file. Some files were fully read (119318) because they ended up being identical or
didn't differ until the last chunk which was read. Finally, some files which are unique in size
may be skipped (7435).

## Optimizations

`fddup` has two options to tune how it runs. The defaults should be suitable for most situations,
but if you have an odd architecture or storage backend (`sshfs` over a slow link, for example),
you might consider tuning these.

### Number of Threads

The number of threads which will be used to dispatch read requests and perform the digest
calculations can be modified using `-j` (`--threads`). This defaults to the number of logical
cores (including AMD SMT/Intel Hyper-Threading). For most kinds of storage, this is reasonable.
Many SSDs will be able to cope with a deep queue of read requests quite well. Even disk arrays of
mechanical drives will likely be able to perform well with many threads.

In my testing, more threads is pretty much always better than less threads. For systems with low
core counts, you may wish to run more threads than you have logical cores, especially if you have
an I/O system than can cope well with high queue depths.

### Read Size

The second parameter, `-s` (`--read-size`), controls the size of each read request and is the chunk
size by which file contents will be hashed and compared with one another. For fast I/O systems,
this should be a large value and defaults to 512 KiB which is the maximum allowed by `fddup` as it
preallocates a thread local buffer used for reading. If you have an I/O system which is reasonably
low latency but slow, you may see a benefit to decreasing this value.
