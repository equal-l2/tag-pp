use std::collections::{BTreeMap, HashSet};
use std::io::BufRead;
use std::io::Write;
use std::prelude::v1::*;

mod parse;
use parse::*;

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
    let mut tags = BTreeMap::new();
    let mut no_tags = Vec::new();

    {
        let f = std::fs::File::open(tag).unwrap();
        let r = std::io::BufReader::new(f);

        // read all entries and put them into Map
        for s in r.lines() {
            let mut s = s.unwrap();
            if s.ends_with('\n') {
                s.pop();
            }
            if let Some(i) = parse_string_to_tag_id(&s) {
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
    }

    let f = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
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
fn sc_geotag_pp(tag_pp: String, geotag: String, to: String) {
    // retrieve NO_TAG
    let no_tags = {
        let f = std::fs::File::open(tag_pp).unwrap();
        let mut r = std::io::BufReader::new(f);
        let mut buf = String::new();
        r.read_line(&mut buf).unwrap();
        if buf.ends_with('\n') {
            buf.pop();
        }
        let mut it = buf.split(',').skip(1);
        if it.next().unwrap().parse::<u64>().unwrap() == 0 {
            println!("NO_TAG is empty");
            Default::default()
        } else {
            it.map(|s| s.parse().unwrap()).collect()
        }
    };

    eprintln!("tag read");

    let f = std::fs::File::open(geotag).unwrap();
    let r = std::io::BufReader::new(f);

    let mut geotags = Vec::new();
    for s in r.lines() {
        let mut s = s.unwrap();
        if s.ends_with('\n') {
            s.pop();
        }
        match parse_string_to_id_geotag(&s, &no_tags) {
            Ok(i) => geotags.push(i),
            Err(e) => match e {
                ParseError::NoTag(_) => { /* no-op */ }
                _ => eprintln!("Ignored : {}", s),
            },
        }
    }
    eprintln!("Entries: {}", geotags.len());

    let f = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(to)
        .unwrap();
    let mut w = std::io::BufWriter::new(f);

    for v in geotags {
        writeln!(
            w,
            "{},{},{},{},{},{},{:010x}",
            v.0, v.1.time, v.1.latitude, v.1.longitude, v.1.domain_num, v.1.url_num1, v.1.url_num2
        );
    }
}

fn sc_gen_test(tag: String, geotag: String, to_dir: String, num: usize) {
    // read geotags
    let f = std::fs::File::open(geotag).unwrap();
    let r = std::io::BufReader::new(f);
    let v = Default::default();

    let geotags: Vec<(_, _)> = r
        .lines()
        .filter_map(|s| parse_string_to_id_geotag(&s.unwrap(), &v).ok())
        .take(num)
        .collect();

    let nums: HashSet<_> = geotags.iter().map(|g| g.0).collect();
    eprintln!("GeoTag entries: {}", geotags.len());

    // read tags
    let f = std::fs::File::open(tag).unwrap();
    let r = std::io::BufReader::new(f);
    let mut tags = BTreeMap::new();
    let mut no_tags = Vec::new();

    for s in r.lines() {
        let mut s = s.unwrap();
        if s.ends_with('\n') {
            s.pop();
        }
        if let Some(i) = parse_string_to_tag_id(&s) {
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

    let f = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(format!("{}/tag_pp.csv", to_dir))
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
        );
    }

    let f = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(format!("{}/geotag_pp.csv", to_dir))
        .unwrap();
    let mut w = std::io::BufWriter::new(f);

    for v in geotags {
        writeln!(
            w,
            "{},{},{},{},{},{},{:010x}",
            v.0, v.1.time, v.1.latitude, v.1.longitude, v.1.domain_num, v.1.url_num1, v.1.url_num2
        );
    }
}

fn ultimate() {
    const ENTRY_COUNT: usize = 100;
    use std::collections::HashMap;
    let geotags = {
        let mut geotags = HashMap::new();

        let f = std::fs::File::open("geotag_pp.csv").unwrap();
        let r = std::io::BufReader::new(f);

        for s in r.lines() {
            let mut s = s.unwrap();
            if s.ends_with('\n') {
                s.pop();
            }
            let ret = tag_geotag::GeoTag::from_str_to_geotag(&s).expect(&s);
            geotags.insert(ret.0, ret.1);
        }
        geotags
    };
    println!("Geotag read");

    let tags = {
        let f = std::fs::File::open("tag_pp.csv").unwrap();
        let r = std::io::BufReader::new(f);

        let mut tags: HashMap<String, Vec<_>> = HashMap::new();
        // Note that tag_pp.csv has "NO_TAG" at the first line
        for s in r.lines().skip(1) {
            let mut s = s.unwrap();
            if s.ends_with('\n') {
                s.pop();
            }
            let mut sp = s.split(',');
            let key = sp.next().unwrap();
            // skip the size column
            let ids = {
                let mut raw = sp.skip(1).map(|s| s.parse().unwrap()).collect::<Vec<_>>();
                raw.sort_by(|a, b| geotags[a].time.cmp(&geotags[b].time).reverse());
                raw.into_iter().take(ENTRY_COUNT).collect()
            };

            tags.insert(key.to_owned(), ids);
        }

        tags
    };
    println!("Tag read");

    {
        let f = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open("tag_ultimate.csv")
            .unwrap();
        let mut w = std::io::BufWriter::new(f);

        for (k, v) in tags.iter() {
            writeln!(
                w,
                "{},{}",
                k,
                v.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            );
        }
    }
    println!("Tag wrote");

    {
        let f = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open("geotag_ultimate.csv")
            .unwrap();
        let mut w = std::io::BufWriter::new(f);

        let occur_id = {
            let mut raw: Vec<_> = tags.values().flatten().collect();
            raw.sort_unstable();
            raw.dedup();
            raw
        };

        for id in occur_id {
            let v = &geotags[id];
            writeln!(
                w,
                "{},{},{},{},{},{},{:010x}",
                id, v.time, v.latitude, v.longitude, v.domain_num, v.url_num1, v.url_num2
            );
        }
    }
    println!("Geotag wrote");
}

fn main() {
    let mut args = std::env::args().skip(1);
    let sc = args.next().expect("Subcommand missing");

    match sc.as_str() {
        "tag-pp" => {
            let tag = args.next().expect("tag-pp: tag missing");
            let to = args.next().expect("tag-pp: To missing");
            sc_tag_pp(tag, to);
        }
        "geotag-pp" => {
            let tag_pp = args.next().expect("geotag-pp: Tag_pp missing");
            let geotag = args.next().expect("geotag-pp: Geotag missing");
            let to = args.next().expect("geotag-pp: To missing");
            sc_geotag_pp(tag_pp, geotag, to);
        }
        "ultimate" => {
            ultimate();
        }
        "gen-test" => {
            let tag = args.next().expect("gen-test: Tag missing");
            let geotag = args.next().expect("gen-test: Geotag missing");
            let to_dir = args.next().expect("gen-test: To_dir missing");
            let num = args
                .next()
                .expect("gen-test: Num missing")
                .parse()
                .expect("gen-test: Num is not numeric");
            sc_gen_test(tag, geotag, to_dir, num);
        }
        _ => panic!("Unknown subcommand"),
    }
}
