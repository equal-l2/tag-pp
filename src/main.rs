use std::collections::HashMap;
use std::io::BufRead;
use std::io::Write;
use std::prelude::v1::*;

fn sc_tag_pp(from: String, to: String) {
    let mut tags = HashMap::new();

    {
        let no_tag_key = "NO_TAG";
        let f = std::fs::File::open(from).unwrap();
        let mut r = std::io::BufReader::new(f);

        let tag_re = regex::Regex::new(r"^(\d+),(.*)$").unwrap();

        let mut buf = String::new();
        while r.read_line(&mut buf).unwrap() != 0 {
            if buf.ends_with('\n') {
                buf.pop();
            }
            if let Some(i) = tag_re.captures(&buf) {
                let id = i.get(1).unwrap().as_str();
                let key = i.get(2).map(|m| {
                    let s = m.as_str();
                    if s.is_empty() { no_tag_key } else { s }
                }).unwrap();
                tags.entry(key.to_owned())
                    .or_insert_with(Vec::new)
                    .push(id.parse::<u64>().unwrap());
                } else {
                    println!("Ignored : {}", buf);
            }
            buf.clear();
        }
        println!("Entries: {}", tags.len());
        println!("NO_TAG: {}", tags.get(no_tag_key).map_or(0, |v| v.len()));
    }

    let f = std::fs::OpenOptions::new().write(true).create(true).open(to).unwrap();
    let mut w = std::io::BufWriter::new(f);

    for (k, v) in tags {
        writeln!(w, "{},{},{}", k, v.len(), v.iter().map(ToString::to_string).collect::<Vec<_>>().join(","));
    }
}

fn sc_geotag_pp(from: String, to: String) {}

fn main() {
    let mut args = std::env::args();
    let _ = args.next(); // discard args[0] (typically executable name)
    let sc = args.next().expect("First argument missing");
    let from = args.next().expect("Second argument missing");
    let to = args.next().expect("Third argument missing");

    match sc.as_str() {
        "tag-pp" => sc_tag_pp(from, to),
        "geotag-pp" => sc_geotag_pp(from, to),
        _ => panic!("First argument is empty")
    }
}
