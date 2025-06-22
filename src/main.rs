use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Router, response::Html, routing::get};
use minijinja::{Environment, context};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::fs;
use tower_http::services::ServeDir;

mod posts;

// TODO: proper timezone handling
//
// NOTES: There is a lot of unwrap usage in this program, as I'm the only one
// using it and I want to be able to understand failures (i.e. I messed up
// something in the templates), in more business ready software I'd probably
// just return a 500 but I dont need to worry about that.

struct AppState {
    env: Environment<'static>,
    // this is essentially the metadata system, its needed right now for sorting
    // blogposts for both the index and the atom feed. key is just the filename
    // of the blogpost
    posts: HashMap<String, posts::Post>,
}

#[tokio::main]
async fn main() {
    // init template engine and add templates
    let mut env = Environment::new();
    env.add_template("layout", include_str!("../templates/layout.jinja"))
        .unwrap();
    env.add_template("home", include_str!("../templates/home.jinja"))
        .unwrap();
    env.add_template("blog_index", include_str!("../templates/blog_index.jinja"))
        .unwrap();
    env.add_template("about", include_str!("../templates/about.jinja"))
        .unwrap();
    env.add_template("blogpost", include_str!("../templates/blogpost.jinja"))
        .unwrap();
    env.add_template("atom", include_str!("../templates/atom.jinja"))
        .unwrap();

    // pass env to handlers via state
    let app_state = Arc::new(AppState {
        env,
        posts: posts::init_posts(),
    });

    // define routes
    let app = Router::new()
        .route("/", get(handler_home))
        .route("/blog", get(handler_blog_index))
        .route("/blog/{post}", get(handler_blog_post))
        .route("/feed.atom", get(handler_feed))
        .route("/about", get(handler_about))
        .nest_service("/assets", ServeDir::new("assets"))
        .with_state(app_state);

    // run it
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn handler_home(State(state): State<Arc<AppState>>) -> Result<Html<String>, StatusCode> {
    let template = state.env.get_template("home").unwrap();

    let rendered = template
        .render(context! {
            title => "Bezmuth",
            welcome_text => "Software dev, open source contributor, writer of blogs, destroyer of worlds, etc etc",
        })
        .unwrap();

    Ok(Html(rendered))
}

/// This function takes a vector of links and their post metadata and returns a list of them sorted by date
///
/// This is neccessary as we pull the posts from a directory (no garuntee of
/// ordering) and we store the post metadata in hashmap (again, no garuntee of
/// ordering) It may be better to store this in the state and generate it at
/// startup, but for now this is sufficient.
fn get_posts_sorted(
    mut posts: Vec<(String, posts::Post)>,
    date_format: &str,
) -> Vec<(String, String, String)> {
    posts.sort_by(|a, b| b.1.date_time.cmp(&a.1.date_time));

    posts
        .into_iter()
        // NOTE: this isnt really a proper converter to the atom time standard,
        // it does not consider time zones
        .map(|(link, post)| {
            let date = post.date_time.format(date_format).to_string(); // lets just pretend im always in gmt
            (link, post.name, date)
        })
        .collect()
}

async fn handler_blog_index(
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, StatusCode> {
    let template = state
        .env
        .get_template("blog_index")
        .unwrap();

    let posts: Vec<_> = state.posts.clone().into_iter().collect();

    let entries = get_posts_sorted(posts, "%Y-%m-%d");

    let rendered = template
        .render(context! {
            title => "Blog Posts",
            entries,
        })
        .unwrap();

    Ok(Html(rendered))
}

async fn handler_feed(State(state): State<Arc<AppState>>) -> Result<String, StatusCode> {
    let template = state
        .env
        .get_template("atom")
        .unwrap();

    let posts: Vec<_> = state.posts.clone().into_iter().collect();

    let entries = get_posts_sorted(posts, "%Y-%m-%dT%H:%M:%SZ");

    let mut last_post_date = String::new();
    if let Some(last_post) = entries.first() {
        last_post_date = last_post.2.clone();
    }

    let rendered = template
        .render(context! {
            last_post_date,
            entries,
        }).unwrap();
    Ok(rendered)
}

async fn handler_blog_post(
    State(state): State<Arc<AppState>>,
    Path(filename): Path<String>,
) -> Result<Html<String>, StatusCode> {
    if let Some(post) = state.posts.get(&filename) {
        let template = state
            .env
            .get_template("blogpost")
            .unwrap();
        let post_content = fs::read_to_string(post.clone().path)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let rendered = template
            .render(context! {
                title => post.name,
                blog_post => post_content,
            })
            .unwrap();

        Ok(Html(rendered))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn handler_about(State(state): State<Arc<AppState>>) -> Result<Html<String>, StatusCode> {
    let template = state
        .env
        .get_template("about")
        .unwrap();

    let rendered = template
        .render(context! {
            title => "About",
        })
        .unwrap();

    Ok(Html(rendered))
}
