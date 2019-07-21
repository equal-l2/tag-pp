use once_cell::sync::Lazy;
use regex::Regex;
use tag_geotag::*;
use std::collections::HashSet;

static TAG_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(\d+),(.*)$").unwrap());
static QUOTE_RE: Lazy<Regex> = Lazy::new(|| Regex::new("\"\"\"(.*)\"\"\"").unwrap());
static GEOTAG_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"^(\d{{8,10}}),(.+),(.+),(.+),{}(\d){}(\d{{1,4}})/\d{{8,10}}_([0-9a-f]{{10}}){}$",
        URL_PREFIX, URL_COMMON, URL_SUFFIX
    ))
    .unwrap()
});


#[derive(Debug)]
pub enum ParseError {
    NoTag(u64),
    NoMatch,
}

pub fn parse_string_to_tag_id(s: &String) -> Option<(String, u64)> {
    if let Some(i) = TAG_RE.captures(&s) {
        let id = i.get(1).unwrap().as_str().parse::<u64>().unwrap();
        let key = i.get(2).unwrap().as_str();
        let key = match QUOTE_RE.captures(&key) {
            Some(i) => i.get(1).unwrap().as_str(),
            None => key,
        };
        Some((key.to_owned(), id))
    } else {
        None
    }
}

pub fn parse_string_to_id_geotag(s: &String, no_tags: &HashSet<u64>) -> Result<(u64, GeoTag), ParseError> {
    if let Some(i) = GEOTAG_RE.captures(&s) {
        let mut i = i.iter().skip(1);
        let id = i.next().unwrap().unwrap().as_str().parse().unwrap();
        if no_tags.contains(&id) {
            return Err(ParseError::NoTag(id));
        }
        let time = {
            let s = i.next().unwrap().unwrap().as_str();
            chrono::NaiveDateTime::parse_from_str(&s[1..s.len() - 1], "%Y-%m-%d %H:%M:%S")
                .unwrap()
                .timestamp() as i32
        };
        let latitude: f64 = i.next().unwrap().unwrap().as_str().parse().unwrap();
        let longitude: f64 = i.next().unwrap().unwrap().as_str().parse().unwrap();
        let domain_num = i.next().unwrap().unwrap().as_str().parse().unwrap();
        let url_num1 = i.next().unwrap().unwrap().as_str().parse().unwrap();
        let url_num2 = u64::from_str_radix(i.next().unwrap().unwrap().as_str(), 16).unwrap();
        Ok((
            id,
            GeoTag {
                time,
                latitude,
                longitude,
                domain_num,
                url_num1,
                url_num2,
            },
        ))
    } else {
        Err(ParseError::NoMatch)
    }
}
