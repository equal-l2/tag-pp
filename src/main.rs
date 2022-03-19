use std::collections::{BTreeMap, HashSet};
use std::io::BufRead;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::prelude::v1::*;

use anyhow::Result;

mod parse;
use parse::*;

#[cfg(feature = "unfair")]
mod unfair;

/// Preprocess tag.csv
///
/// Each lines of the resulting CSV is defined as below:
///
/// <line> ::= <tag-name>,<ids-len>,<ids>
/// <ids> ::= <id>{,<id>}
///
/// Note that <ids-len> is the count of <id> in <ids>, and curly brackets mean optional elements.
///
/// The resulting CSV has a "NO_TAG" entry at the first line, which contains ids without tag.
///
fn sc_tag_pp<T: AsRef<Path>>(tag: T, to: T) -> Result<()> {
    let (tags, no_tags) = {
        let mut tags = BTreeMap::new();
        let mut no_tags = Vec::new();

        let f = std::fs::File::open(tag)?;
        let r = std::io::BufReader::new(f);

        // read all entries and put them into Map
        for s in r.lines() {
            let s = s?;
            let s = s.trim_end();
            if let Some(i) = parse_string_to_tag_id(s) {
                if i.0.is_empty() {
                    no_tags.push(i.1);
                } else {
                    tags.entry(i.0).or_insert_with(Vec::new).push(i.1);
                }
            } else {
                eprintln!("Ignored : {}", s);
            }
        }
        eprintln!("Entries: {}", tags.len());
        eprintln!("NO_TAG len: {}", no_tags.len());

        (tags, no_tags)
    };

    {
        let f = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(to)?;
        let mut w = std::io::BufWriter::new(f);

        // write NO_TAG first
        writeln!(
            w,
            "NO_TAG,{},{}",
            no_tags.len(),
            no_tags
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        )?;

        // write the other normal entries
        for (k, v) in tags {
            writeln!(
                w,
                "{},{},{}",
                k,
                v.len(),
                v.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            )?;
        }
    }

    Ok(())
}

/// Preprocess geotag.csv
///
/// This reads "NO_TAG" from tag_pp.csv, then read geotag.csv, remove any entries contained in
/// "NO_TAG", finally transform URL into 2 elements.
///
/// Each lines of resulting csv is defined as below:
///
/// <line> ::= <id>,<time>,<latitude>,<longitude>,<domain-num>,<url-num1>,<url-num2>
///
/// In data we use, <url-num1> is 1-4 digits, <url-num2> is 8-10 digits, and <url-num3> is exactly
/// 10 digits.
///
/// The original URL is in the form as below:
/// <url> ::= http://farm<domain-num>.static.flickr.com/<url-num1>/<id>_<url-num2>.jpg
///
/// We do not use a String as long as possible since it is space-inefficient provided the value fits
/// 64-bit integer.
/// By this transformation, we can reduce data length by around 30%.
///
fn sc_geotag_pp<T: AsRef<Path>>(tag_pp: T, geotag: T, to: T) -> Result<()> {
    // retrieve NO_TAG
    let no_tags = {
        // read the first line (NO_TAG)
        let f = std::fs::File::open(tag_pp)?;
        let mut buf = String::new();
        std::io::BufReader::new(f).read_line(&mut buf)?;

        let buf = buf.trim_end();
        let mut it = buf.split(',').skip(1);
        if it.next().unwrap().parse::<u64>()? == 0 {
            println!("NO_TAG is empty");
            Default::default()
        } else {
            it.map(|s| s.parse().unwrap()).collect()
        }
    };

    eprintln!("tag read");

    let geotags = {
        let f = std::fs::File::open(geotag)?;
        let r = std::io::BufReader::new(f);

        let mut geotags = Vec::new();
        for s in r.lines() {
            let s = s?;
            let s = s.trim_end();
            // read only the geotag having tags
            match parse_string_to_id_geotag(s, &no_tags) {
                Ok(i) => geotags.push(i),
                Err(e) => match e.downcast_ref() {
                    Some(GeoTagParseError::NoTag(_)) => { /* no-op */ }
                    _ => eprintln!("Ignored : {}", s),
                },
            }
        }
        eprintln!("Entries: {}", geotags.len());

        geotags
    };

    // write the result
    {
        let f = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(to)?;
        let mut w = std::io::BufWriter::new(f);

        for v in geotags {
            writeln!(
                w,
                "{},{},{},{},{},{},{:010x}",
                v.0,
                v.1.time,
                v.1.latitude,
                v.1.longitude,
                v.1.domain_num,
                v.1.url_num1,
                v.1.url_num2
            )?;
        }
    }

    Ok(())
}

// get top n items from geotags, and generate coherent tag_pp and geotag_pp
fn sc_gen_test(tag: String, geotag: String, to_dir: String, num: usize) -> Result<()> {
    // read geotags
    let f = std::fs::File::open(geotag)?;
    let r = std::io::BufReader::new(f);
    let v = Default::default();

    let geotags: Vec<_> = r
        .lines()
        .filter_map(|s| parse_string_to_id_geotag(&s.unwrap(), &v).ok())
        .take(num)
        .collect();

    let nums: HashSet<_> = geotags.iter().map(|g| g.0).collect();
    eprintln!("GeoTag entries: {}", geotags.len());

    // read tags
    let (tags, no_tags) = {
        let f = std::fs::File::open(tag)?;
        let r = std::io::BufReader::new(f);
        let mut tags = BTreeMap::new();
        let mut no_tags = Vec::new();

        for s in r.lines() {
            let s = s?;
            let s = s.trim_end();
            if let Some(i) = parse_string_to_tag_id(s) {
                if nums.contains(&i.1) {
                    if i.0.is_empty() {
                        no_tags.push(i.1);
                    } else {
                        tags.entry(i.0).or_insert_with(Vec::new).push(i.1);
                    }
                }
            } else {
                eprintln!("Ignored : {}", s);
            }
        }

        eprintln!("Tag entries: {}", tags.len());
        eprintln!("NO_TAG len: {}", no_tags.len());
        (tags, no_tags)
    };

    {
        let f = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(format!("{}/tag_pp.csv", to_dir))?;
        let mut w = std::io::BufWriter::new(f);

        // write NO_TAG first
        writeln!(
            w,
            "NO_TAG,{},{}",
            no_tags.len(),
            no_tags
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        )?;

        // write the other normal entries
        for (k, v) in tags {
            writeln!(
                w,
                "{},{},{}",
                k,
                v.len(),
                v.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            )?;
        }
    }

    {
        let f = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(format!("{}/geotag_pp.csv", to_dir))?;
        let mut w = std::io::BufWriter::new(f);

        for v in geotags {
            writeln!(
                w,
                "{},{},{},{},{},{},{:010x}",
                v.0,
                v.1.time,
                v.1.latitude,
                v.1.longitude,
                v.1.domain_num,
                v.1.url_num1,
                v.1.url_num2
            )?;
        }
    }

    Ok(())
}

fn hikaku<T: AsRef<Path>>(tag_pp: T, geotag_pp: T) -> Result<()> {
    let tags_ids = {
        let f = std::fs::File::open(tag_pp)?;
        let r = std::io::BufReader::new(f);

        let mut ids = HashSet::new();

        for l in r.lines().skip(1) {
            let l = l?;
            let it = l.split(',').skip(2);
            ids.extend(it.map(|s| s.parse::<u64>().unwrap()));
        }
        ids
    };

    let geotags_ids = {
        let f = std::fs::File::open(geotag_pp)?;
        let r = std::io::BufReader::new(f);

        let mut ids = HashSet::new();

        for l in r.lines() {
            let l = l?;
            let id: u64 = l.split(',').next().unwrap().parse().unwrap();
            ids.insert(id);
        }
        ids
    };

    if tags_ids != geotags_ids {
        let diff = tags_ids.difference(&geotags_ids).collect::<Vec<_>>();
        if diff.len() < 100 {
            println!("{} {:?}", diff.len(), diff);
        } else {
            println!("{} {:?}", diff.len(), &diff[0..100]);
        }
    }

    Ok(())
}

#[derive(clap::StructOpt)]
enum Opt {
    #[clap(name = "tag-pp", about = "Preprocess tags")]
    TagPp {
        tag: PathBuf,
        to: PathBuf,
    },
    #[clap(name = "geotag-pp", about = "Preprocess geotags")]
    GeoTagPp {
        tag_pp: PathBuf,
        geotag: PathBuf,
        to: PathBuf,
    },
    #[clap(
        name = "gen-test",
        about = "Extract small tag_pp and geotag_pp for testing"
    )]
    GenTest {
        tag: String,
        geotag: String,
        to_dir: String,
        num: usize,
    },
    #[cfg(feature = "ultimate")]
    #[clap(name = "geotag-pp")]
    Ultimate,
    Hikaku {
        tag_pp: PathBuf,
        geotag_pp: PathBuf,
    },
}

fn main() {
    let opt: Opt = clap::Parser::parse();

    match opt {
        Opt::TagPp { tag, to } => sc_tag_pp(tag, to).unwrap(),
        Opt::GeoTagPp { tag_pp, geotag, to } => sc_geotag_pp(tag_pp, geotag, to).unwrap(),
        Opt::GenTest {
            tag,
            geotag,
            to_dir,
            num,
        } => sc_gen_test(tag, geotag, to_dir, num).unwrap(),
        #[cfg(feature = "unfair")]
        Opt::Ultimate => unfair::ultimate(),
        Opt::Hikaku { tag_pp, geotag_pp } => hikaku(tag_pp, geotag_pp).unwrap(),
    }
}
