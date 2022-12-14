use actix_web::{get, web::Data, HttpRequest, HttpResponse};
use serde::Serialize;
use itertools::Itertools;

use crate::{AppState, DATE_FORMATTING};

pub const TEMPLATE: &'static str = include_str!("index.hbs");

const TITLE_CHAR_LIMIT: usize = 60;
const CONTENT_CHAR_LIMIT: usize = 700;

#[derive(Serialize)]
struct Thread {
    thread_id: String,
    user_id: String,
    created: String,
    last_updated: String,
    title: String,
    content: String,
    overflow: bool,
    num_comments: i32,
    multiple_comments: bool,
}

#[derive(Serialize)]
struct PageState {
    num_threads: usize,
    threads: Vec<Thread>,
    user_id: String,
}

#[get("/")]
pub async fn get_index(req: HttpRequest, data: Data<AppState>) -> HttpResponse {
    let mut resp = HttpResponse::Ok();
    let user_id = crate::manage_cookies(&req, &data, &mut resp).await;

    let threads: Vec<Thread> = sqlx::query!(
        r#"
        SELECT 
            threads.thread_id,
            threads.user_id,
            threads.created,
            threads.last_updated,
            threads.title,
            threads.content,
            COALESCE(comments.num_comments, 0) AS "num_comments!"
        FROM
            threads
        LEFT JOIN
            (SELECT thread_id, COUNT(thread_id) num_comments
            FROM comments
            GROUP BY thread_id) comments
            ON threads.thread_id = comments.thread_id
        "#
    )
    .fetch_all(&data.database)
    .await
    .unwrap()
    .into_iter()
    // I have to sort it manually here in rust instead of in the sql
    // query because sqlx doesn't like ORDER BY in this query?
    .sorted_by(|a, b| Ord::cmp(&b.last_updated, &a.last_updated))
    .map(|x| {
        let num_comments: i32 = x.num_comments;

        let mut title = x.title;

        if title.len() > TITLE_CHAR_LIMIT {
            title = truncate_by_chars(title, TITLE_CHAR_LIMIT);
            title.push_str("...");
        }

        let mut content = x.content;

        let overflow = if content.len() > CONTENT_CHAR_LIMIT {
            content = truncate_by_chars(content, CONTENT_CHAR_LIMIT);
            content.push_str("...");

            true
        } else {
            false
        };

        Thread {
            thread_id: x.thread_id,
            user_id: x.user_id,
            created: x.created.format(DATE_FORMATTING).to_string(),
            last_updated: x.last_updated.format(DATE_FORMATTING).to_string(),
            title,
            content,
            overflow,
            num_comments,
            multiple_comments: num_comments > 1
        }
    })
    .collect();

    let num_threads = threads.len();

    let page: String = data
        .template_registry
        .render(
            "index",
            &PageState {
                threads,
                num_threads,
                user_id,
            },
        )
        .unwrap();

    return resp.body(page);
}

fn truncate_by_chars(s: String, max_width: usize) -> String {
    s.chars().take(max_width).collect()
}
