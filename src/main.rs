use std::collections::{HashMap, HashSet};
use std::io::BufRead;
use std::io::Write;
use std::prelude::v1::*;
use tag_geotag::*;

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
fn sc_tag_pp(tag: String, to: String) {
    let mut tags = HashMap::new();
    let mut no_tags = HashSet::new();

    {
        let f = std::fs::File::open(tag).unwrap();
        let r = std::io::BufReader::new(f);

        let tag_re = regex::Regex::new(r"^(\d+),(.*)$").unwrap();

        // read all entries and put them into HashMap
        for s in r.lines() {
            let mut s = s.unwrap();
            if s.ends_with('\n') {
                s.pop();
            }
            if let Some(i) = tag_re.captures(&s) {
                let id = i.get(1).unwrap().as_str().parse::<u64>().unwrap();
                let key = i.get(2).unwrap().as_str();
                if key.is_empty() {
                    no_tags.insert(id);
                } else {
                    tags.entry(key.to_owned()).or_insert_with(Vec::new).push(id);
                }
            } else {
                eprintln!("Ignored : {}", s);
            }
        }
        eprintln!("Entries: {}", tags.len());
        eprintln!("NO_TAG len: {}", no_tags.len());
    }

    let f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(to)
        .unwrap();
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
    );

    // write other normal entries
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
        );
    }
}

/// Preprocess geotag.csv
///
/// This reads "NO_TAG" from tag_pp.csv, then read geotag.csv, remove any entries contained in
/// "NO_TAG", finally transform URL into 2 elements.
///
/// Each lines of resulting csv is defined as below:
///
/// <line> ::= <id>,<time>,<latitude>,<longitude>,<server-num>,<url-part>
///
/// The original URL is in the form as below:
/// <url> ::= http://farm<server-num>.static.flickr.com/<url-part>.jpg
///
/// We remove common parts and extract the distinct parts.
/// By the transformation, we can reduce data length by around 30%.
///
fn sc_geotag_pp(tag: String, geotag: String, to: String) {
    // retrieve NO_TAG
    let no_tags: HashSet<u64> = {
        let f = std::fs::File::open(tag).unwrap();
        let mut r = std::io::BufReader::new(f);
        let mut buf = String::new();
        r.read_line(&mut buf).unwrap();
        if buf.ends_with('\n') {
            buf.pop();
        }
        buf.split(',').skip(2).map(|s| s.parse().unwrap()).collect()
    };

    eprintln!("tag read");

    let geotag_re = regex::Regex::new(&format!(
        r"^(\d+),(.+),(.+),(.+),{}(\d){}(.+){}$",
        URL_PREFIX, URL_COMMON, URL_SUFFIX
    ))
    .unwrap();

    let f = std::fs::File::open(geotag).unwrap();
    let r = std::io::BufReader::new(f);

    let mut geotags = HashMap::new();
    for s in r.lines() {
        let mut s = s.unwrap();
        if s.ends_with('\n') {
            s.pop();
        }
        if let Some(i) = geotag_re.captures(&s) {
            let mut i = i.iter().skip(1);
            let id: u64 = i.next().unwrap().unwrap().as_str().parse().unwrap();
            if no_tags.contains(&id) {
                continue;
            }

            let time = {
                let s = i.next().unwrap().unwrap().as_str();
                chrono::NaiveDateTime::parse_from_str(s, "\"%Y-%m-%d %H:%M:%S\"").unwrap()
            };
            let latitude: f64 = i.next().unwrap().unwrap().as_str().parse().unwrap();
            let longitude: f64 = i.next().unwrap().unwrap().as_str().parse().unwrap();
            let serv_num = i.next().unwrap().unwrap().as_str().chars().next().unwrap();
            let url_part = i.next().unwrap().unwrap().as_str().to_owned();
            geotags.insert(
                id,
                GeoTag {
                    time,
                    latitude,
                    longitude,
                    serv_num,
                    url_part,
                },
            );
        } else {
            eprintln!("Ignored : {}", s);
        }
    }
    eprintln!("Entries: {}", geotags.len());

    let f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(to)
        .unwrap();
    let mut w = std::io::BufWriter::new(f);

    for (k, v) in geotags {
        writeln!(
            w,
            "{},{:?},{},{},{},{}",
            k, v.time, v.latitude, v.longitude, v.serv_num, v.url_part
        );
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let sc = args.next().expect("First argument missing");

    match sc.as_str() {
        "tag-pp" => {
            let tag = args.next().expect("tag-pp: tag missing");
            let to = args.next().expect("tag-pp: To missing");
            sc_tag_pp(tag, to);
        }
        "geotag-pp" => {
            let tag = args.next().expect("tag-pp: Tag missing");
            let geotag = args.next().expect("tag-pp: Geotag missing");
            let to = args.next().expect("tag-pp: To missing");
            sc_geotag_pp(tag, geotag, to);
        }
        _ => panic!("First argument is empty"),
    }
}
