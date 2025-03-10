// Import Crates
use actix_web::{get, middleware::NormalizePath, web, App, HttpResponse, HttpServer};

// Reference local files
mod post;
mod proxy;
mod search;
// mod settings;
mod subreddit;
mod user;
mod utils;

// Create Services
async fn style() -> HttpResponse {
	HttpResponse::Ok().content_type("text/css").body(include_str!("../static/style.css"))
}

async fn robots() -> HttpResponse {
	HttpResponse::Ok().body(include_str!("../static/robots.txt"))
}

#[get("/favicon.ico")]
async fn favicon() -> HttpResponse {
	HttpResponse::Ok().body("")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	let args: Vec<String> = std::env::args().collect();
	let mut address = "0.0.0.0:8080".to_string();

	if args.len() > 1 {
		for arg in args {
			if arg.starts_with("--address=") || arg.starts_with("-a=") {
				let split: Vec<&str> = arg.split('=').collect();
				address = split[1].to_string();
			}
		}
	}

	// start http server
	println!("Running Libreddit v{} on {}!", env!("CARGO_PKG_VERSION"), &address);

	HttpServer::new(|| {
		App::new()
			// TRAILING SLASH MIDDLEWARE
			.wrap(NormalizePath::default())
			// DEFAULT SERVICE
			.default_service(web::get().to(utils::error))
			// GENERAL SERVICES
			.route("/style.css/", web::get().to(style))
			.route("/favicon.ico/", web::get().to(HttpResponse::Ok))
			.route("/robots.txt/", web::get().to(robots))
			// SETTINGS SERVICE
			// .route("/settings/", web::get().to(settings::get))
			// .route("/settings/save/", web::post().to(settings::set))
			// PROXY SERVICE
			.route("/proxy/{url:.*}/", web::get().to(proxy::handler))
			// SEARCH SERVICES
			.route("/search/", web::get().to(search::find))
			.route("r/{sub}/search/", web::get().to(search::find))
			// USER SERVICES
			.route("/u/{username}/", web::get().to(user::profile))
			.route("/user/{username}/", web::get().to(user::profile))
			// WIKI SERVICES
			.route("/wiki/", web::get().to(subreddit::wiki))
			.route("/wiki/{page}/", web::get().to(subreddit::wiki))
			.route("/r/{sub}/wiki/", web::get().to(subreddit::wiki))
			.route("/r/{sub}/wiki/{page}/", web::get().to(subreddit::wiki))
			// SUBREDDIT SERVICES
			.route("/r/{sub}/", web::get().to(subreddit::page))
			.route("/r/{sub}/{sort:hot|new|top|rising|controversial}/", web::get().to(subreddit::page))
			// POPULAR SERVICES
			.route("/", web::get().to(subreddit::page))
			.route("/{sort:best|hot|new|top|rising|controversial}/", web::get().to(subreddit::page))
			// POST SERVICES
			.route("/{id:.{5,6}}/", web::get().to(post::item))
			.route("/r/{sub}/comments/{id}/{title}/", web::get().to(post::item))
			.route("/r/{sub}/comments/{id}/{title}/{comment_id}/", web::get().to(post::item))
	})
	.bind(&address)
	.unwrap_or_else(|_| panic!("Cannot bind to the address: {}", address))
	.run()
	.await
}
