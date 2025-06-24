use bincode::{Decode, Encode};
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use scraper::{Html, Selector};
use sled::Db;
use std::fs;
use std::str;

// Post metadata is stored in a database alongside the posts because I cannot
// know the timezone the blogposts were written in, we pull from the local
// timezone when the posts were added instead and then store that in the
// database. This is so I dont break any atom readers

#[derive(Debug, Clone, Encode, Decode)]
struct Post {
    pub name: String,
    date_time: String,
}
impl Post {
    fn new(name: String, date_time: chrono::DateTime<Utc>) -> Self {
        Self {
            name,
            date_time: date_time.to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Metadata {
    db: Db,
}
impl Metadata {
    pub fn new() -> Result<Self, sled::Error> {
        let meta = Self {
            db: sled::open("posts/metadata")?,
        };
        let post_paths = fs::read_dir("posts").unwrap();

        for path in post_paths {
            let filename = path.unwrap().file_name().to_string_lossy().to_string();
            meta.add_post(filename);
        }
        Ok(meta)
    }

    fn get_post(self, filename: &str) -> Option<Post> {
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
    pub fn get_post_title(self, filename: &str) -> Option<String> {
        if let Some(post) = self.get_post(filename) {
            Some(post.name)
        } else {
            None
        }
    }

    /// Two assumptions are made about post files:
    /// 1. They are valid(ish) html exported from orgmode
    /// 2. The filenames are valid rust strings
    pub fn add_post(&self, filename: String) {
        if filename.ends_with("html") && !self.db.contains_key(&filename).unwrap() {
            // read the file
            let contents = fs::read_to_string(format!("posts/{filename}")).unwrap();
            let fragment = Html::parse_fragment(&contents);

            // parse title
            let mut title = String::new();
            let title_selector = Selector::parse("title").unwrap();
            for element in fragment.select(&title_selector) {
                title = element.inner_html().to_string();
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

            let post = Post::new(title, date);
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
        // I was a bit worried about how long this function would take, but its
        // only a couple of hundered microseconds on --release with 20 test
        // files. I should really just generate this whenever I add a new file
        // but for now its fine, a bit wasteful but fine. Or I could use a
        // proper database but I think thats overkill

        // First, convert the entire db into a vector so it can be sorted
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
                // convert the filename key back into a string
                (
                    String::from_utf8_lossy(&filename).to_string(),
                    post.name,
                    date_time,
                )
            })
            .collect();

        // sort the posts
        posts.sort_by(|a, b| b.2.cmp(&a.2));

        posts
            .into_iter()
            .map(|(filename, name, date_time)| {
                // convert datetime to requested format
                let date = match format {
                    Some(fmt) => date_time.format(fmt).to_string(),
                    None => date_time.to_rfc3339(),
                };
                format.map_or_else(
                    || date_time.to_rfc3339(),
                    |fmt| date_time.format(fmt).to_string(),
                );
                (filename, name, date) // sketchy
            })
            .collect()
    }
}
