use anyhow::{anyhow, Context, Result};
use clap::Parser;
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};

/// Analyzes the occurrences of entry types in a log file
///
/// Each line in the input file should contain a complete json object containing a `type`
/// field. The entries in the file will be grouped by this type and for each unique type
/// the following statistics will be printed:
///
/// The number of entries with this type. The space used (in bytes, excluding the line terminator)
/// by all entries with this type.
// NOTE: The previous paragraph should be a markdown list but I couldn't figure out how to make
// clap not merge adjacent lines.
#[derive(Parser, Debug)]
#[clap(max_term_width = 90)]
struct Args {
    /// The file to analyze
    file: OsString,
}

#[derive(Default)]
struct TypeData {
    /// The number of entries with this type.
    num: u64,
    /// The number of bytes used by all entries with this type.
    bytes: u64,
}

// NOTE: serde_json ignores unknown fields by default.
#[derive(Deserialize)]
struct JsonObject {
    // NOTE: The exercise description does not specify the type of the "type" field. So it would not
    // be incorrect for the type to be a number or an array. This program would error out on such type
    // fields. Therefore, the type of this field should really by `serde_json::Value` which can hold
    // any kind of json value. Unfortunately, `serde_json::Value` does not implement `Hash` and can
    // therefore not be used easily as the key in a HashMap. Therefore we would have to implement a
    // small wrapper type around `serde_json::Value` that implements `Hash`, `PartialEq`, and
    // `Deserialize`.
    //
    // However, since the example always used string types, I've decided to go with this field type for
    // the exercise. In a real project, I would ask for the requirements to be clarified first.
    #[serde(rename = "type")]
    ty: String,
}

fn main() {
    let args = Args::parse();

    let result = match process_file(&args.file) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Could not process file {:?}: {:?}", args.file, e);
            std::process::exit(1);
        }
    };

    // Sort the result by type name to make the output reproducible.
    let mut result: Vec<_> = result.into_iter().collect();
    result.sort_by(|(l, _), (r, _)| l.cmp(r));

    for (ty, stats) in result {
        println!(
            "Type {:?}: Number of Objects: {}; Total Bytes: {}",
            ty, stats.num, stats.bytes
        );
    }
}

fn process_file(file: &OsString) -> Result<HashMap<String, TypeData>> {
    let mut result = HashMap::new();
    let file = File::open(file).context("Could not open the file")?;
    let file = BufReader::new(file);
    for (n, line) in file.lines().enumerate() {
        process_line(&mut result, line)
            .with_context(|| format!("Could not process line number {}", n + 1))?;
    }
    Ok(result)
}

fn process_line(stats: &mut HashMap<String, TypeData>, line: io::Result<String>) -> Result<()> {
    let line = line.context("Could not read from the file")?;
    let obj: JsonObject =
        serde_json::from_str(&line).with_context(|| format!("Could not parse `{}`", line))?;
    let data = stats.entry(obj.ty).or_default();
    // NOTE: These fields cannot realistically overflow. Even if each byte took only 1ns to process,
    // it would still take more than 300 years before data.bytes overflows. I assume that serde_json
    // is much slower than that. Furthermore, the only way for us to process so many bytes is if
    // the input file refers to a pipe (or some weird FUSE file system). I'm using `checked_add` only
    // because this is an exercise and to demonstrate that I'm aware of such issues.
    data.num += 1;
    data.bytes = data
        .bytes
        .checked_add(line.len() as u64)
        .ok_or_else(|| anyhow!("Total number of bytes processed exceeded 2^64"))?;
    Ok(())
}
