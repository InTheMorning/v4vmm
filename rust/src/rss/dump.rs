use anyhow::{Context, Result};
use reqwest;
use rss::{
    extension::{Extension, ExtensionMap},
    Channel,
};
use std::io::Cursor;

// tool to help us figure out how feeds are parsed
pub fn cmd_rss_dump(feed_url: &str) -> Result<()> {
    println!("Fetching: {feed_url}");

    let body = reqwest::blocking::Client::new()
        .get(feed_url)
        .send()
        .with_context(|| format!("GET {feed_url}"))?
        .error_for_status()
        .with_context(|| format!("HTTP error for {feed_url}"))?
        .bytes()
        .with_context(|| format!("read body {feed_url}"))?;

    let feed = Channel::read_from(Cursor::new(body)).context("parse RSS")?;

    println!("--- channel ---");
    println!("title: {}", feed.title());
    println!("link : {}", feed.link());
    println!("desc : {}", feed.description());
    dump_exts("channel extensions", feed.extensions());

    println!("--- items (first 3) ---");
    for (i, item) in feed.items().iter().take(3).enumerate() {
        println!("#{}", i + 1);
        println!("  title: {:?}", item.title());
        println!("  guid : {:?}", item.guid().map(|g| g.value()));
        println!("  enc  : {:?}", item.enclosure().map(|e| e.url()));
        println!("  pub  : {:?}", item.pub_date());

        // itunes tags
        let itunes = item.itunes_ext();

        let dur = itunes.and_then(|it| it.duration()).map(|s| s.to_string());
        let expl = itunes.and_then(|it| it.explicit()).map(|s| s.to_string());
        let img = itunes.and_then(|it| it.image()).map(|s| s.to_string());

        println!("  itunes:duration => {:?}", dur);
        println!("  itunes:explicit => {:?}", expl);
        println!("  itunes:image    => {:?}", img);

        dump_exts("  item extensions", item.extensions());
    }

    Ok(())
}

pub fn dump_exts(label: &str, exts: &ExtensionMap) {
    println!("{label}:");
    for (ns, keys) in exts {
        println!("  NS: {ns}");
        for (k, vec) in keys {
            println!("    key: {k} ({} value(s))", vec.len());
            for (j, ext) in vec.iter().enumerate() {
                dump_one_ext(j, ext, 6);
            }
        }
    }
}

pub fn dump_one_ext(idx: usize, ext: &Extension, indent: usize) {
    let pad = " ".repeat(indent);
    println!("{pad}[{idx}] value={:?} attrs={:?}", ext.value, ext.attrs);

    if !ext.children.is_empty() {
        // print only child keys + counts (keeps it readable)
        let child_summary: Vec<String> = ext
            .children
            .iter()
            .map(|(k, v)| format!("{k}={}", v.len()))
            .collect();
        println!("{pad}    children: {}", child_summary.join(", "));
    }
}
