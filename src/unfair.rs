use std::io::{BufRead, Write};

pub fn ultimate() {
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
