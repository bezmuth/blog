use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Router, response::Html, routing::get};
use minijinja::{Environment, context};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::fs;

mod posts;

struct AppState {
    env: Environment<'static>,
    // using the title as an identifier for a post isn't best practice but meh
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
        .route("/about", get(handler_about))
        .with_state(app_state);

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
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

async fn handler_blog_index(
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, StatusCode> {
    let template = state.env.get_template("blog_index").unwrap();

    // we have to sort the posts based on creation date, as we just pull in
    // posts from a directory (and store them in a hashmap) we cannot rely on ordering
    let mut post_vec: Vec<(String, posts::Post)> = state.posts.clone().into_iter().collect();
    post_vec.sort_by(|a, b| b.1.date.cmp(&a.1.date));
    // then we construct a vec of links
    let link_vec: Vec<String> = post_vec
        .into_iter()
        .map(|x| format!("<a href=\"blog/{}\">{}</a>", x.0, x.0))
        .collect();

    let rendered = template
        .render(context! {
            title => "Blog Posts",
            entries => link_vec,
        })
        .unwrap();

    Ok(Html(rendered))
}

async fn handler_blog_post(
    State(state): State<Arc<AppState>>,
    Path(title): Path<String>,
) -> Result<Html<String>, StatusCode> {
    let template = state.env.get_template("blogpost").unwrap();
    let post = state.posts.get(&title).unwrap();
    let blog_post = fs::read_to_string(post.clone().path).await.unwrap();

    let rendered = template
        .render(context! {
            blog_post,
        })
        .unwrap();

    Ok(Html(rendered))
}

async fn handler_about(State(state): State<Arc<AppState>>) -> Result<Html<String>, StatusCode> {
    let template = state.env.get_template("about").unwrap();

    let rendered = template.render(context!{
        title => "About",
        about_text => "Simple demonstration layout for an axum project with minijinja as templating engine.",
    }).unwrap();

    Ok(Html(rendered))
}
