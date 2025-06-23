use bincode::{Decode, Encode};
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use scraper::{Html, Selector};
use sled::Db;
use std::fs;
use std::path::PathBuf;
use std::str;

// Post metadata is stored in a database alongside the posts because if I cannot
// know the timezone the blogposts were written in, we pull from the local
// timezone when the posts were added instead and then store that in the
// database

#[derive(Debug, Clone, Encode, Decode)]
pub struct Post {
    pub name: String,
    date_time: String,
    pub path: std::path::PathBuf,
}
impl Post {
    pub fn new(name: String, date_time: chrono::DateTime<Utc>, path: std::path::PathBuf) -> Self {
        Post {
            name,
            date_time: date_time.to_rfc3339(),
            path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Metadata {
    db: Db,
}
impl Metadata {
    pub fn new() -> Result<Self, sled::Error> {
        let meta = Metadata {
            db: sled::open("posts/metadata")?,
        };
        let post_paths = fs::read_dir("posts").unwrap();

        for path in post_paths {
            // get the path
            let path = path.unwrap().path();
            meta.add_post(path)
        }
        Ok(meta)
    }

    pub fn get_post(self, filename: &str) -> Option<Post> {
        if let Ok(Some(bins)) = self.db.get(filename) {
            Some(
                bincode::decode_from_slice(&bins[..], bincode::config::standard())
                    .unwrap()
                    .0,
            )
        } else {
            None
        }
    }

    pub fn add_post(&self, path: PathBuf) {
        // get the filename
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        if filename.ends_with("html") && !self.db.contains_key(&filename).unwrap() {
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
            let mut date: DateTime<Utc> = DateTime::default();
            let date_selector = Selector::parse("p.date").unwrap();
            for element in fragment.select(&date_selector) {
                // this is fragile
                let date_string: String = element.inner_html()[9..].to_string();
                let naive =
                    NaiveDateTime::parse_from_str(&date_string, "%Y-%m-%d %a %H:%M").unwrap();
                let local_date = Local.from_local_datetime(&naive).unwrap();
                date = local_date.to_utc();
            }

            let post = Post::new(title, date, path.to_path_buf());
            let bins = bincode::encode_to_vec(post, bincode::config::standard()).unwrap();
            self.db.insert(filename, bins).unwrap();
        }
    }

    /// This function returns all posts in the db sorted by date
    ///
    /// This is neccessary as we pull the posts from a directory (no garuntee of
    /// ordering) and we store the post metadata in a db (again, no garuntee of
    /// ordering)
    ///
    /// Passing a None to the format string formats the datetime in rfc3339
    pub fn get_posts_sorted(self, format: Option<&str>) -> Vec<(String, String, String)> {
        let mut posts: Vec<_> = self
            .db
            .into_iter()
            .map(|x| {
                let (filename, val) = x.unwrap();
                // decode the post data form the db
                let post: Post = bincode::decode_from_slice(&val[..], bincode::config::standard())
                    .unwrap()
                    .0;
                // convert the string into datetime so we can easily sort
                let date_time = chrono::DateTime::parse_from_rfc3339(&post.date_time).unwrap();
                (filename, post.name, date_time, post.path)
            })
            .collect();

        posts.sort_by(|a, b| b.2.cmp(&a.2));

        posts
            .into_iter()
            // NOTE: this isnt really a proper converter to the atom time standard,
            // it does not consider time zones
            .map(|(filename, name, date_time, _)| {
                let date = if let Some(format) = format {
                    date_time.format(format).to_string()
                } else {
                    date_time.to_rfc3339()
                };
                (String::from_utf8_lossy(&filename).to_string(), name, date) // sketchy
            })
            .collect()
    }
}
