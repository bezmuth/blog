use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Router, response::Html, routing::get};
use minijinja::{Environment, context};
use std::sync::Arc;
use tokio::fs;
use tower_http::services::ServeDir;

mod db;

//  I've shifted to storing the post metadata in a database, i can just watch
//  the posts folder now but I'm not sure if theres any need
//
//
// NOTES: There is a lot of unwrap usage in this program, as I'm the only one
// using it and I want to be able to understand failures (i.e. I messed up
// something in the templates), in more business ready software I'd probably
// just return a 500 but I dont need to worry about that.
//
// TODO: add github activity and recent blogposts to my home page, maybe split these into two panes?
// TODO: regenerate sorted posts only on adding a blogpost
// TODO: watch the posts dir for any changes to add new blogposts

struct AppState {
    env: Environment<'static>,
    metadata: db::Metadata,
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
        metadata: db::Metadata::new().unwrap(),
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

    let entries = &state.metadata.clone().get_posts_sorted(Some("%Y-%m-%d"))[0..10];

    let rendered = template
        .render(context! {
            title => "Bezmuth",
            welcome_text => "Software dev, open source contributor, blog writer, div centerer, destroyer of worlds, etc., etc.",
            entries,
        })
        .unwrap();

    Ok(Html(rendered))
}

async fn handler_blog_index(
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, StatusCode> {
    let template = state.env.get_template("blog_index").unwrap();

    let entries = state.metadata.clone().get_posts_sorted(Some("%Y-%m-%d"));

    let rendered = template
        .render(context! {
            title => "Blog Posts",
            entries,
        })
        .unwrap();

    Ok(Html(rendered))
}

async fn handler_feed(State(state): State<Arc<AppState>>) -> Result<String, StatusCode> {
    let template = state.env.get_template("atom").unwrap();

    let entries = state.metadata.clone().get_posts_sorted(None);

    let mut last_post_date = String::new();
    if let Some(last_post) = entries.first() {
        last_post_date.clone_from(&last_post.2);
    }

    let rendered = template
        .render(context! {
            last_post_date,
            entries,
        })
        .unwrap();
    Ok(rendered)
}

async fn handler_blog_post(
    State(state): State<Arc<AppState>>,
    Path(filename): Path<String>,
) -> Result<Html<String>, StatusCode> {
    // this just verifies the requested file exists in the database
    if let Some(title) = state.metadata.clone().get_post_title(&filename) {
        let template = state.env.get_template("blogpost").unwrap();
        let post_content = fs::read_to_string(format!("posts/{filename}"))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let rendered = template
            .render(context! {
                title,
                blog_post => post_content,
            })
            .unwrap();

        Ok(Html(rendered))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn handler_about(State(state): State<Arc<AppState>>) -> Result<Html<String>, StatusCode> {
    let template = state.env.get_template("about").unwrap();

    let rendered = template
        .render(context! {
            title => "About",
        })
        .unwrap();

    Ok(Html(rendered))
}
