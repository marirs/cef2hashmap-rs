use crate::{Error, Result};
use fancy_regex::Regex;
use std::collections::HashMap;

lazy_static! {
    static ref CEF: Regex = Regex::new(
        &[
            r"^(?:(?:<(?P<pri>\d+)>))?",
            r"(?P<agentReceivedTime>(?:[a-z]+[ ])?",
            r"(?:[\d :\-T]+\b[ ]?))?",
            r"(?P<ahost>.*)",
            r"CEF:0(?:\|(?P<deviceVendor>.*?(?=\|)))?",
            r"(?:\|(?P<deviceProduct>.*?(?=\|)))?",
            r"(?:\|(?P<deviceVersion>.*?(?=\|)))?",
            r"(?:\|(?P<signatureId>.*?(?=\|)))?",
            r"(?:\|(?P<name>.*?(?=\|)))?",
            r"(?:\|(?P<severity>.*?(?=\|)))?",
            r"\|(?P<cef_ext>.*)"
        ]
        .join("")
    )
    .unwrap();
    static ref CEF_EXT: Regex =
        Regex::new(r"[ ]?(?P<key>[^= ]+?)=(?P<value>[^=]*?)(?=(?:[ ][^= ]*?=|$))").unwrap();
    static ref TEMPLATES: HashMap<&'static str, &'static str> = {
        include_str!("../assets/template_keys.txt")
            .lines()
            .map(|s| s.split_at(s.find(":").unwrap()))
            .map(|(key, val)| {
                (
                    key.trim(),
                    val[1..].split(',').map(|s| s.trim()).collect::<Vec<_>>(),
                )
            })
            .collect::<HashMap<&str, Vec<&str>>>()
            .iter()
            .flat_map(|(key, val)| {
                val.iter()
                    .map(|v| (*v, *key))
                    .collect::<HashMap<&str, &str>>()
            })
            .collect()
    };
}

pub trait CefToHashMap {
    fn to_hashmap(&self, keep_raw: bool) -> Result<HashMap<String, String>>;
}

impl CefToHashMap for &str {
    fn to_hashmap(&self, keep_raw: bool) -> Result<HashMap<String, String>> {
        cef_to_map(self, keep_raw)
    }
}

impl CefToHashMap for String {
    fn to_hashmap(&self, keep_raw: bool) -> Result<HashMap<String, String>> {
        cef_to_map(self, keep_raw)
    }
}

/// Convert the CEF String into HashMap
fn cef_to_map(cef_str: &str, keep_raw: bool) -> Result<HashMap<String, String>> {
    if !cef_str.to_lowercase().contains("cef:0|") {
        return Err(Error::NotCef);
    }
    let mut header = get_cef_as_kv(cef_str);
    if header.contains_key("cef_ext") {
        let extension = get_cef_ext_as_kv(header.get("cef_ext").unwrap());
        header.remove("cef_ext");
        header.extend(extension)
    }
    // if found, add the facility & priority/severity from syslog line
    if header.contains_key("pri") {
        let pri = header.get("pri").unwrap().parse::<u8>().unwrap();
        header.remove("pri");
        let facility = (pri >> 3).to_string();
        let priority = (pri & 7).to_string();
        header.insert("syslogFacility".to_string(), facility);
        header.insert("syslogPriority".to_string(), priority);
    }
    // Keep the raw log cef str
    if keep_raw {
        header.insert("rawEvent".to_string(), cef_str.trim().to_string());
    }

    Ok(apply_template(header))
}

/// Gets the CEF Header as well as the CEF_Extension in hashmap
fn get_cef_as_kv(cef_str: &str) -> HashMap<String, String> {
    let caps = CEF.captures(cef_str).unwrap();
    CEF.capture_names()
        .flatten()
        .filter_map(|n| {
            Some((
                n.trim().to_string(),
                caps.as_ref()?.name(n)?.as_str().trim().to_string(),
            ))
        })
        .filter(|(_, v)| !v.is_empty())
        .collect()
}

/// Extracts all the key=value pairs into HashMap from the CEF_Extension
fn get_cef_ext_as_kv(cef_ext: &str) -> HashMap<String, String> {
    CEF_EXT
        .captures_iter(cef_ext)
        .flat_map(|c| {
            c.into_iter()
                .flat_map(|m| {
                    m.iter()
                        .map(|x| x.unwrap().as_str().trim())
                        .filter(|s| s.contains('='))
                        .collect::<Vec<_>>()
                })
                .map(|s| s.split_at(s.find('=').unwrap()))
                .map(|(k, v)| (k.trim().to_string(), v[1..].trim().to_string()))
                .collect::<HashMap<String, String>>()
        })
        .collect()
}

fn apply_template(map: HashMap<String, String>) -> HashMap<String, String> {
    let mut res = HashMap::new();
    for (key, val) in &map {
        match TEMPLATES.get(&key.as_str()) {
            Some(k) => {
                res.insert(k.to_string(), val.clone());
            }
            None => {
                res.insert(key.clone(), val.clone());
            }
        }
    }
    res
}