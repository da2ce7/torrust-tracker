// code-review: should we use macros to return the exact line where the assert fails?

use reqwest::Response;
use torrust_tracker::apis::resources::auth_key::AuthKey;
use torrust_tracker::apis::resources::stats::Stats;
use torrust_tracker::apis::resources::torrent::{ListItem, Torrent};

// Resource responses

pub async fn assert_stats(response: Response, stats: Stats) {
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-type").unwrap(), "application/json");
    assert_eq!(response.json::<Stats>().await.unwrap(), stats);
}

pub async fn assert_torrent_list(response: Response, torrents: Vec<ListItem>) {
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-type").unwrap(), "application/json");
    assert_eq!(response.json::<Vec<ListItem>>().await.unwrap(), torrents);
}

pub async fn assert_torrent_info(response: Response, torrent: Torrent) {
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-type").unwrap(), "application/json");
    assert_eq!(response.json::<Torrent>().await.unwrap(), torrent);
}

pub async fn assert_auth_key_utf8(response: Response) -> AuthKey {
    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json; charset=utf-8"
    );
    response.json::<AuthKey>().await.unwrap()
}

// OK response

pub async fn assert_ok(response: Response) {
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-type").unwrap(), "application/json");
    assert_eq!(response.text().await.unwrap(), "{\"status\":\"ok\"}");
}

// Error responses

pub async fn assert_bad_request(response: Response, body: &str) {
    assert_eq!(response.status(), 400);
    assert_eq!(response.headers().get("content-type").unwrap(), "text/plain; charset=utf-8");
    assert_eq!(response.text().await.unwrap(), body);
}

pub async fn assert_not_found(response: Response) {
    assert_eq!(response.status(), 404);
    // todo: missing header in the response
    //assert_eq!(response.headers().get("content-type").unwrap(), "text/plain; charset=utf-8");
    assert_eq!(response.text().await.unwrap(), "");
}

pub async fn assert_torrent_not_known(response: Response) {
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-type").unwrap(), "application/json");
    assert_eq!(response.text().await.unwrap(), "\"torrent not known\"");
}

pub async fn assert_invalid_infohash_param(response: Response, invalid_infohash: &str) {
    assert_bad_request(
        response,
        &format!("Invalid URL: invalid infohash param: string \"{invalid_infohash}\", expected a 40 character long string"),
    )
    .await;
}

pub async fn assert_invalid_auth_key_param(response: Response, invalid_auth_key: &str) {
    assert_bad_request(response, &format!("Invalid auth key id param \"{}\"", &invalid_auth_key)).await;
}

pub async fn assert_invalid_key_duration_param(response: Response, invalid_key_duration: &str) {
    assert_bad_request(
        response,
        &format!("Invalid URL: Cannot parse `\"{invalid_key_duration}\"` to a `u64`"),
    )
    .await;
}

pub async fn assert_token_not_valid(response: Response) {
    assert_unhandled_rejection(response, "token not valid").await;
}

pub async fn assert_unauthorized(response: Response) {
    assert_unhandled_rejection(response, "unauthorized").await;
}

pub async fn assert_failed_to_remove_torrent_from_whitelist(response: Response) {
    assert_unhandled_rejection(response, "failed to remove torrent from whitelist").await;
}

pub async fn assert_failed_to_whitelist_torrent(response: Response) {
    assert_unhandled_rejection(response, "failed to whitelist torrent").await;
}

pub async fn assert_failed_to_reload_whitelist(response: Response) {
    assert_unhandled_rejection(response, "failed to reload whitelist").await;
}

pub async fn assert_failed_to_generate_key(response: Response) {
    assert_unhandled_rejection(response, "failed to generate key").await;
}

pub async fn assert_failed_to_delete_key(response: Response) {
    assert_unhandled_rejection(response, "failed to delete key").await;
}

pub async fn assert_failed_to_reload_keys(response: Response) {
    assert_unhandled_rejection(response, "failed to reload keys").await;
}

async fn assert_unhandled_rejection(response: Response, reason: &str) {
    assert_eq!(response.status(), 500);
    assert_eq!(response.headers().get("content-type").unwrap(), "text/plain; charset=utf-8");

    let reason_text = format!("Unhandled rejection: Err {{ reason: \"{reason}");
    let response_text = response.text().await.unwrap();
    assert!(
        response_text.contains(&reason_text),
        ":\n  response: `\"{response_text}\"`\n  dose not contain: `\"{reason_text}\"`."
    );
}
