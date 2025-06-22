use chrono::NaiveDateTime;
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone)]
pub struct Post {
    pub name: String,
    pub date_time: chrono::NaiveDateTime,
    pub path: std::path::PathBuf,
}

impl Post {
    pub fn new(name: String, date_time: chrono::NaiveDateTime, path: std::path::PathBuf) -> Self {
        Post {
            name,
            date_time,
            path,
        }
    }
}

/// Parses org exported html from the posts directory
pub fn init_posts() -> HashMap<String, Post> {
    // this is only run on server startup, so you'll need to restart the
    // webserver whenever you add a post
    let mut posts: HashMap<String, Post> = HashMap::new();
    let post_paths = fs::read_dir("posts").unwrap();

    for path in post_paths {
        // get the path
        let path = path.unwrap().path();

        // get the filename
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        if filename.ends_with("html") {
            // read the file
            let contents = fs::read_to_string(&path).unwrap();
            let fragment = Html::parse_fragment(&contents);

            // parse title
            let mut title = String::new();
            let title_selector = Selector::parse("title").unwrap();
            for element in fragment.select(&title_selector) {
                title = element.inner_html().to_string()
            }

            // parse date
            let mut date: NaiveDateTime = NaiveDateTime::default();
            let date_selector = Selector::parse("p.date").unwrap();
            for element in fragment.select(&date_selector) {
                // this is fragile
                let date_string: String = element.inner_html()[9..].to_string();
                date = NaiveDateTime::parse_from_str(&date_string, "%Y-%m-%d %a %H:%M").unwrap();
            }

            posts.insert(filename, Post::new(title, date, path.to_path_buf()));
        }
    }

    posts
}
