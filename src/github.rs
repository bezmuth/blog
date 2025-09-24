use octocrab::{
    models::{self, events::{payload::{EventPayload::PushEvent, PushEventPayload, WrappedEventPayload}, EventType}}, Octocrab, Page
};

pub struct PushInfo {
    pub repo_name: String,
    pub commits: Vec<CommitInfo>,
}
pub struct CommitInfo {
    pub commit_message: String,
    pub commit_url: String,
}

pub async fn get_user_events() -> octocrab::Result<()> {
    let token = std::env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN env variable is required");

    let octocrab = Octocrab::builder().personal_token(token).build()?;

    let page: Page<models::events::Event> =
        octocrab.get("/users/bezmuth/events", None::<&()>).await?;

    let events: Vec<_> = page
        .items
        .iter()
        .filter(|event| event.r#type == EventType::PushEvent)
        .map(|event| {
            if let Some(push_event) = &event.payload {
                if let PushEvent(res) = push_event.specific.clone().unwrap() {
                    let amount = res.commits.len();
                    let repo = format!("www.github.com{}", res.commits.first().unwrap().url.path());
                    println!("{:?}, {}", amount, repo);
                }
            }
        }).collect();

    Ok(())
}
