extern crate serde;
extern crate quick_xml;

use serde::{Deserialize, Deserializer};
use quick_xml::de::{from_str, DeError};
use std::time::{Duration, Instant};
use quick_xml::Reader;
use quick_xml::events::Event;
extern crate chrono;
use std::fs;
use rayon::prelude::*;
use std::io::{Read, Write};
use std::fs::File;
use std::io::copy;
use std::error::Error;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::hash::Hash;
use chrono::prelude::*;
use reqwest::Url;
use reqwest::blocking::Response;

extern crate select;
use std::io::BufReader;
use crate::select::predicate::Predicate;
use select::document::Document;
use select::predicate::Name;
use std::path::Path;

#[derive(Debug, Deserialize, PartialEq)]
struct SiteMap {
    loc: String,
    lastmod: DateTime<Utc>
}

#[derive(Debug, Deserialize, PartialEq)]
struct SiteMapIndex {
    sitemap: Vec<SiteMap>
}

#[derive(Debug, Deserialize, PartialEq)]
struct UrlSet {
    url: Vec<SiteMap>
}

fn check_if_exists(page_data: &SiteMap) -> bool {
    let url = Url::parse(&page_data.loc).unwrap();
    let path = &format!("static{}/index.html", url.path());
    let metadata = fs::metadata(path);
    if metadata.is_ok() {
        if let Ok(modified) = metadata.unwrap().modified() {
            let fmtime = chrono::DateTime::from(modified);
            if  fmtime > page_data.lastmod {
                // println!("Skipping {}", post_url);
                return true;
            } else {
                println!("Overwriting {} {} {}", url, fmtime, page_data.lastmod);
            }
        } else {
            println!("Invalid metadata for {}", url);

        }
    }
    return false;
}

fn write_file(path: String, content: &String) {
    println!("Writing {}", path);
    let dir = fs::create_dir_all(format!("static{}", path)).unwrap();
    let content = content.replace("https://ghost.rolisz.ro", "https://rolisz.ro");
    fs::write(format!("static{}/index.html", path), content);
}

fn write_xml(path: String, content: &String) {
    let parent = Path::new(&path).parent().and_then(Path::to_str).unwrap();
    println!("Parent {} {}", path, parent);
    let dir = fs::create_dir_all(format!("static{}", parent)).unwrap();
    let content = content.replace("https://ghost.rolisz.ro", "https://rolisz.ro");
    fs::write(format!("static{}", path.trim_end_matches("/")), content);
}

fn has_bad_extension(url: &&str) -> bool{
    let ext = Path::new(&url).extension();
    ext.is_none() && !url.ends_with("/rss/")
    //return url.ends_with("ico") || url.to_lowercase().ends_with("jpg") || url.to_lowercase().ends_with("png")
}

fn get_links_from_html(html: &String) -> HashSet<String> {
     Document::from(html.as_str())
            .find(Name("a").or(Name("link")))
            .filter_map(|n| n.attr("href"))
            .filter(has_bad_extension)
            .filter_map(|x| {
                let new_url = Url::parse(x);
                return match new_url {
                    Ok(new_url) => {
                        if new_url.has_host() && new_url.host_str().unwrap() == "ghost.rolisz.ro" {
                            return Some(x.to_string())
                        } else {
                            //println!("Rejecting {}", x);
                            None
                        }
                    },
                    Err(e) => {
                        // Relative urls are not parsed by Reqwest
                        if x.starts_with('/') {
                            Some(format!("https://ghost.rolisz.ro{}", x))
                        } else {
                            // println!("Parse error {}", x);
                            None
                        }
                    }
                }
            }).collect::<HashSet<String>>()
}

fn get_page(client: &reqwest::blocking::Client, post_url: &String) -> Option<String> {
    let page = client.get(post_url).send();
    if let Ok(mut pg) = page {
        println!("Status for {}: {}", post_url, pg.status());
        if !pg.status().is_success() {
            return None;
        }
        let mut buffer = String::new();
        pg.read_to_string(&mut buffer).unwrap();

        let links = get_links_from_html(&buffer);

        write_file(pg.url().path().to_string(), &buffer);
        //copy(&mut html, &mut file);

        return Some(buffer);
    }
    return None;
}

fn get_non_html(client: &reqwest::blocking::Client, url: &String) -> Option<String> {
    let mut res = client.get(url).send().unwrap();
    println!("Status: {} {}", res.status(), url);

    let mut body  = String::new();
    res.read_to_string(&mut body).unwrap();
    println!("URL to write {}", res.url().path());
    write_xml(res.url().path().to_string(), &body);
    return Some(body);
}

fn main() -> Result<(), reqwest::Error> {
    let mut visited = Arc::new(Mutex::new(HashSet::new()));
    let mut new_links = Arc::new(Mutex::new(HashSet::new()));
    new_links.lock().unwrap().insert("https://ghost.rolisz.ro/".to_string());
    let now = Instant::now();
    let client = reqwest::blocking::Client::new();

    let starting_url = "https://ghost.rolisz.ro/sitemap.xml".to_string();
    let mut body = get_non_html(&client, &starting_url).unwrap();
    visited.lock().unwrap().insert(starting_url);
    let html: SiteMapIndex = from_str(&body).unwrap();

    get_non_html(&client, &"https://ghost.rolisz.ro/sitemap.xsl".to_string());
    get_non_html(&client, &"https://ghost.rolisz.ro/robots.txt".to_string());
    get_non_html(&client, &"https://ghost.rolisz.ro/404.html".to_string());
    get_page(&client, &"https://ghost.rolisz.ro/rss".to_string());
    get_page(&client, &"https://ghost.rolisz.ro/personal/rss".to_string());
    get_page(&client, &"https://ghost.rolisz.ro/tech/rss".to_string());
    get_page(&client, &"https://ghost.rolisz.ro/ghost/api/v3/content/posts/?key=55469b5d4503c566ce6601684e&limit=all&fields=title%2Curl%2Cexcerpt%2Ccustom_excerpt%2Cpublished_at%2Cupdated_at&order=updated_at%20DESC&include=tags&formats=plaintext".to_string());

    for link in html.sitemap {
        let link_url = &link.loc;
        // Pages are handled separately elsewhere, because RSS is included here.
        if link_url.ends_with("sitemap-pages.xml") {
            continue;
        }
        let mut body  = get_non_html(&client, link_url).unwrap();
        visited.lock().unwrap().insert(link_url.to_string());

        let sitemap: UrlSet = from_str(&body).unwrap();

        sitemap.url.par_iter().for_each(|post_link| {
            if post_link.loc != "https://ghost.rolisz.ro/" {
                visited.lock().unwrap().insert(post_link.loc.to_string());
            }
            if check_if_exists(post_link) {
                return
            }
            let res = get_page(&client, &post_link.loc);
            new_links.lock().unwrap().extend(get_links_from_html(&res.unwrap()));
        });

    }

    let mut links : Vec<String> = vec![];
    for v in new_links.lock().unwrap().iter() {
        links.push(v.to_string());
    }
        println!("{:#?}", links);

    let mut temp = HashSet::new();
    while !links.is_empty() {
        links.iter().for_each(|link| {
            if !visited.lock().unwrap().contains(link) {
                let res = get_page(&client, &link);
                visited.lock().unwrap().insert(link.to_string());
                if res.is_some() {
                    temp.extend(get_links_from_html(&res.unwrap()));
                }
            }
        });
        links = Vec::new();
        for v in temp.iter() {
            let stuff = visited.lock().unwrap();
            if !stuff.contains(v) {
                links.push(v.to_string());
            }
        }
    }
    println!("{}", now.elapsed().as_secs());
    //println!("{:#?}", visited.lock().unwrap());
    println!("{:#?}", new_links.lock().unwrap());
    Ok(())
}