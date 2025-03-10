// CRATES
use crate::utils::{error, format_num, format_url, param, request, rewrite_url, val, Comment, Flags, Flair, Post};
use actix_web::{HttpRequest, HttpResponse, Result};

use async_recursion::async_recursion;

use askama::Template;
use chrono::{TimeZone, Utc};

// STRUCTS
#[derive(Template)]
#[template(path = "post.html", escape = "none")]
struct PostTemplate {
	comments: Vec<Comment>,
	post: Post,
	sort: String,
}

pub async fn item(req: HttpRequest) -> HttpResponse {
	let path = format!("{}.json?{}&raw_json=1", req.path(), req.query_string());
	let sort = param(&path, "sort");

	// Log the post ID being fetched in debug mode
	#[cfg(debug_assertions)]
	dbg!(req.match_info().get("id").unwrap_or(""));

	// Send a request to the url, receive JSON in response
	match request(&path).await {
		// Otherwise, grab the JSON output from the request
		Ok(res) => {
			// Parse the JSON into Post and Comment structs
			let post = parse_post(&res[0]).await.unwrap();
			let comments = parse_comments(&res[1]).await.unwrap();

			// Use the Post and Comment structs to generate a website to show users
			let s = PostTemplate { comments, post, sort }.render().unwrap();
			HttpResponse::Ok().content_type("text/html").body(s)
		}
		// If the Reddit API returns an error, exit and send error page to user
		Err(msg) => error(msg.to_string()).await,
	}
}

// UTILITIES
async fn media(data: &serde_json::Value) -> (String, String) {
	let post_type: &str;
	let url = if !data["preview"]["reddit_video_preview"]["fallback_url"].is_null() {
		post_type = "video";
		format_url(data["preview"]["reddit_video_preview"]["fallback_url"].as_str().unwrap().to_string())
	} else if !data["secure_media"]["reddit_video"]["fallback_url"].is_null() {
		post_type = "video";
		format_url(data["secure_media"]["reddit_video"]["fallback_url"].as_str().unwrap().to_string())
	} else if data["post_hint"].as_str().unwrap_or("") == "image" {
		post_type = "image";
		format_url(data["preview"]["images"][0]["source"]["url"].as_str().unwrap().to_string())
	} else {
		post_type = "link";
		data["url"].as_str().unwrap().to_string()
	};

	(post_type.to_string(), url)
}

// POSTS
async fn parse_post(json: &serde_json::Value) -> Result<Post, &'static str> {
	// Retrieve post (as opposed to comments) from JSON
	let post: &serde_json::Value = &json["data"]["children"][0];

	// Grab UTC time as unix timestamp
	let unix_time: i64 = post["data"]["created_utc"].as_f64().unwrap().round() as i64;
	// Parse post score and upvote ratio
	let score = post["data"]["score"].as_i64().unwrap();
	let ratio: f64 = post["data"]["upvote_ratio"].as_f64().unwrap_or(1.0) * 100.0;

	// Determine the type of media along with the media URL
	let media = media(&post["data"]).await;

	// Build a post using data parsed from Reddit post API
	Ok(Post {
		id: val(post, "id"),
		title: val(post, "title"),
		community: val(post, "subreddit"),
		body: rewrite_url(&val(post, "selftext_html")),
		author: val(post, "author"),
		author_flair: Flair(
			val(post, "author_flair_text"),
			val(post, "author_flair_background_color"),
			val(post, "author_flair_text_color"),
		),
		permalink: val(post, "permalink"),
		score: format_num(score),
		upvote_ratio: ratio as i64,
		post_type: media.0,
		flair: Flair(
			val(post, "link_flair_text"),
			val(post, "link_flair_background_color"),
			if val(post, "link_flair_text_color") == "dark" {
				"black".to_string()
			} else {
				"white".to_string()
			},
		),
		flags: Flags {
			nsfw: post["data"]["over_18"].as_bool().unwrap_or(false),
			stickied: post["data"]["stickied"].as_bool().unwrap_or(false),
		},
		media: media.1,
		time: Utc.timestamp(unix_time, 0).format("%b %e %Y %H:%M UTC").to_string(),
	})
}

// COMMENTS
#[async_recursion]
async fn parse_comments(json: &serde_json::Value) -> Result<Vec<Comment>, &'static str> {
	// Separate the comment JSON into a Vector of comments
	let comment_data = json["data"]["children"].as_array().unwrap();

	let mut comments: Vec<Comment> = Vec::new();

	// For each comment, retrieve the values to build a Comment object
	for comment in comment_data {
		let unix_time: i64 = comment["data"]["created_utc"].as_f64().unwrap_or(0.0).round() as i64;
		if unix_time == 0 {
			continue;
		}

		let score = comment["data"]["score"].as_i64().unwrap_or(0);
		let body = rewrite_url(&val(comment, "body_html"));

		let replies: Vec<Comment> = if comment["data"]["replies"].is_object() {
			parse_comments(&comment["data"]["replies"]).await.unwrap_or_default()
		} else {
			Vec::new()
		};

		comments.push(Comment {
			id: val(comment, "id"),
			body,
			author: val(comment, "author"),
			score: format_num(score),
			time: Utc.timestamp(unix_time, 0).format("%b %e %Y %H:%M UTC").to_string(),
			replies,
			flair: Flair(
				val(comment, "author_flair_text"),
				val(comment, "author_flair_background_color"),
				val(comment, "author_flair_text_color"),
			),
		});
	}

	Ok(comments)
}
