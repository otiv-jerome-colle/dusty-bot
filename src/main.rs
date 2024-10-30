mod dusty;

use slack_morphism::prelude::*;

use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full};
use hyper::Response;
use tracing::*;

use axum::Extension;
use std::convert::Infallible;
use std::env;
use std::sync::Arc;
use tokio::net::TcpListener;

async fn test_oauth_install_function(
    resp: SlackOAuthV2AccessTokenResponse,
    _client: Arc<SlackHyperClient>,
    _states: SlackClientEventsUserState,
) {
    println!("{:#?}", resp);
}
async fn test_error_install() -> String {
    "Error while installing".to_string()
}

async fn test_push_event(
    Extension(_environment): Extension<Arc<SlackHyperListenerEnvironment>>,
    Extension(event): Extension<SlackPushEvent>,
) -> Response<BoxBody<Bytes, Infallible>> {
    println!("got event: {event:?}");

    match event {
        SlackPushEvent::UrlVerification(url_ver) => {
            Response::new(Full::new(url_ver.challenge.into()).boxed())
        }
        SlackPushEvent::EventCallback(callback) => {
            let SlackEventCallbackBody::Message(message) = callback.event else {
                return Response::new(Empty::new().boxed());
            };
            if message.sender.bot_id.is_some() {
                return Response::new(Empty::new().boxed());
            }
            let Some(content) = message.content else {
                return Response::new(Empty::new().boxed());
            };
            let Some(text) = content.text else {
                return Response::new(Empty::new().boxed());
            };

            let response = dusty::handle_dusty_query(&text);

            if !response.is_empty() {
                let message_content = SlackMessageContent::new().with_text(response);

                let post_chat_req = SlackApiChatPostMessageRequest::new(
                    message.origin.channel.unwrap(),
                    message_content,
                );

                let client = _environment.client.clone();
                let token_value = env::var("SLACK_BOT_TOKEN").unwrap();
                let token = SlackApiToken::new(token_value.into());

                let session = client.open_session(&token);
                let res = session.chat_post_message(&post_chat_req).await;
                if res.is_err() {
                    warn!("Response couldn't be sent");
                }
            }
            Response::new(Empty::new().boxed())
        }
        _ => Response::new(Empty::new().boxed()),
    }
}
fn test_error_handler(
    err: Box<dyn std::error::Error + Send + Sync>,
    _client: Arc<SlackHyperClient>,
    _states: SlackClientEventsUserState,
) -> HttpStatusCode {
    println!("{:#?}", err);

    // Defines what we return Slack server
    HttpStatusCode::BAD_REQUEST
}

async fn test_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client: Arc<SlackHyperClient> =
        Arc::new(SlackClient::new(SlackClientHyperConnector::new()?));

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
    info!("Loading server: {}", addr);

    let oauth_listener_config = SlackOAuthListenerConfig::new(
        config_env_var("SLACK_CLIENT_ID")?.into(),
        config_env_var("SLACK_CLIENT_SECRET")?.into(),
        config_env_var("SLACK_BOT_SCOPE")?,
        config_env_var("SLACK_REDIRECT_HOST")?,
    );

    let listener_environment: Arc<SlackHyperListenerEnvironment> = Arc::new(
        SlackClientEventsListenerEnvironment::new(client.clone())
            .with_error_handler(test_error_handler),
    );
    let signing_secret: SlackSigningSecret = config_env_var("SLACK_SIGNING_SECRET")?.into();

    let listener: SlackEventsAxumListener<SlackHyperHttpsConnector> =
        SlackEventsAxumListener::new(listener_environment.clone());

    // build our application route with OAuth nested router and Push/Command/Interaction events
    let app = axum::routing::Router::new()
        .nest(
            "/auth",
            listener.oauth_router("/auth", &oauth_listener_config, test_oauth_install_function),
        )
        .route("/error", axum::routing::get(test_error_install))
        .route(
            "/push",
            axum::routing::post(test_push_event).layer(
                listener
                    .events_layer(&signing_secret)
                    .with_event_extractor(SlackEventsExtractors::push_event()),
            ),
        );

    axum::serve(TcpListener::bind(&addr).await.unwrap(), app)
        .await
        .unwrap();

    Ok(())
}

pub fn config_env_var(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|e| format!("{}: {}", name, e))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let subscriber = tracing_subscriber::fmt()
        // .with_env_filter("axum_events_api_server=debug,slack_morphism=debug")
        .finish();
    subscriber::set_global_default(subscriber)?;

    test_server().await?;

    Ok(())
}
