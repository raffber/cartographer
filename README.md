# Cartographer - Produce map files like its 1999

This command line application allows generating map files by parsing executables and their ELF section.


# Building

This project should compile with rust stable as of 03-2020.
For instructions how to build rust projects, refer to the [getting started guide](https://www.rust-lang.org/learn/get-started).


# Feature Set

 * Parse TI-COFF files
 * Parse DWARF sections as generated by the TI C2000 compiler
 * Provides an extensible library to add more binary file types


# Usage


```
$ ./cartographer.exe --help
cartographer - Produce map files like its 1999 0.1
Raphael Bernhard <beraphae@gmail.com>
Extract DWARF information from executables and creates map files

USAGE:
    cartographer.exe [FLAGS] [OPTIONS] --input <INPUT_FILE>

FLAGS:
    -h, --help       Prints help information
    -p, --pretty     Defines whether the resulting json file should be pretty printed.
    -V, --version    Prints version information

OPTIONS:
    -i, --input <INPUT_FILE>      Input file binary file to be processed.
    -o, --output <OUTPUT_FILE>    Output map files to be written.
```


# About

Developed by Raphael Bernhard (raphael.bernhard@psbel.com / beraphae@gmail.com)