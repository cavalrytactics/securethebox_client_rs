use graphql_client::{GraphQLQuery, Response as GQLResponse};

use seed::{prelude::*, *};
use serde::{Deserialize, Serialize};

mod shared;

// Global types and Constant values
type Name = String;
type Author = String;

const API_URL: &str = "http://c2.local:8000";
const WS_URL: &str = "ws://c2.local:8000";

// ------ ------
//    GraphQL
// ------ ------
macro_rules! generate_query {
    ($query:ident) => {
        //
        // graphql_client basics
        //
        #[derive(GraphQLQuery)]
        #[graphql(
            schema_path = "graphql/schema.graphql",
            query_path = "graphql/queries.graphql",
            response_derives = "Debug"
        )]
        struct $query;
    };
}

generate_query!(QBooks);
generate_query!(MCreateBook);
generate_query!(MDeleteBook);

async fn send_graphql_request<V, T>(variables: &V) -> fetch::Result<T>
where
    V: Serialize,
    T: for<'de> Deserialize<'de> + 'static,
{
    Request::new(API_URL)
        .method(Method::Post)
        .json(variables)?
        .fetch()
        .await?
        .check_status()?
        .json()
        .await
}

// ------ ------
//     Init
// ------ ------

fn init(_: Url, orders: &mut impl Orders<Msg>) -> Model {
    //
    // GraphQL Query fetch data
    //
    orders.perform_cmd(async {
        Msg::BooksFetched(send_graphql_request(&QBooks::build_query(q_books::Variables)).await)
    });
    //
    // Init Model default values
    //
    Model {
        books: Option::Some(Vec::new()),
        sent_messages_count: 0,
        messages: Vec::new(),
        input_text_name: String::new(),
        input_text_author: String::new(),
        web_socket: create_websocket(orders),
        web_socket_reconnector: None,
        selected_name: std::default::Default::default(),
        selected_author: std::default::Default::default(),
    }
}
//
// websocket client connect to server
//
fn create_websocket(orders: &impl Orders<Msg>) -> WebSocket {
    WebSocket::builder(WS_URL, orders)
        .on_open(|| Msg::WebSocketOpened)
        .on_message(Msg::MessageReceived)
        .on_close(Msg::WebSocketClosed)
        .on_error(|| Msg::WebSocketFailed)
        .build_and_open()
        .unwrap()
}

// ------ ------
//     Model
// ------ ------

// #[derive(Default)]
struct Model {
    sent_messages_count: usize,
    messages: Vec<String>,
    input_text_name: String,
    input_text_author: String,
    selected_name: Option<Name>,
    selected_author: Option<Author>,
    web_socket: WebSocket,
    web_socket_reconnector: Option<StreamHandle>,
    books: Option<Vec<q_books::QBooksBooks>>,
}

// ------ ------
//    Update
// ------ ------

enum Msg {
    BooksFetched(fetch::Result<GQLResponse<q_books::ResponseData>>),
    BookCreated(fetch::Result<GQLResponse<q_books::ResponseData>>),
    BookCreatedClick(Name, Author),
    WebSocketOpened,
    MessageReceived(WebSocketMessage),
    CloseWebSocket,
    WebSocketClosed(CloseEvent),
    WebSocketFailed,
    ReconnectWebSocket(usize),
    InputTextNameChanged(String),
    InputTextAuthorChanged(String),
}

fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        //
        // GraphQL functions
        //
        Msg::BooksFetched(Ok(GQLResponse {
            data: Some(data), ..
        })) => {
            model.books = Some(data.books);
        }
        Msg::BookCreated(Ok(GQLResponse { data: Some(_), .. })) => {
            println!("Created Book");
        }
        Msg::BookCreatedClick(name, author) => {
            model.selected_name = Some(name.clone());
            model.selected_author = Some(author.clone());
            orders.perform_cmd(async {
                Msg::BookCreated(
                    send_graphql_request(&MCreateBook::build_query(m_create_book::Variables {
                        name,
                        author,
                    }))
                    .await,
                )
            });
        }
        Msg::BookCreated(error) => log!(error),
        Msg::BooksFetched(error) => log!(error),
        //
        // Websocket functions
        //
        Msg::WebSocketOpened => {
            model.web_socket_reconnector = None;
            {
                model
                    .web_socket
                    .send_json(&shared::ClientMessageGQLInit {
                        r#type: "connection_init".to_string(),
                        payload: shared::PayloadEmp {},
                    })
                    .unwrap();
            }
            {
                model
                    .web_socket
                    .send_json(&shared::ClientMessageGQLPay {
                        id: "1".to_string(),
                        r#type: "start".to_string(),
                        payload: {
                            shared::Payload {
                                query: "subscription {books(mutationType: CREATED) {id}}"
                                    .to_string(),
                            }
                        },
                    })
                    .unwrap();
            }
            log!("WebSocket connection is open now");
        }
        Msg::MessageReceived(message) => {
            log!("Client received a message");
            let json_message = message.json::<serde_json::Value>().unwrap();
            log!("{}", json_message);
            model
                .messages
                .push(format!("{}", message.json::<serde_json::Value>().unwrap()));
        }
        Msg::CloseWebSocket => {
            model.web_socket_reconnector = None;
            model
                .web_socket
                .close(None, Some("user clicked Close button"))
                .unwrap();
        }
        Msg::WebSocketClosed(close_event) => {
            log!("==================");
            log!("WebSocket connection was closed:");
            log!("Clean:", close_event.was_clean());
            log!("Code:", close_event.code());
            log!("Reason:", close_event.reason());
            log!("==================");

            // Chrome doesn't invoke `on_error` when the connection is lost.
            if !close_event.was_clean() && model.web_socket_reconnector.is_none() {
                model.web_socket_reconnector = Some(
                    orders.stream_with_handle(streams::backoff(None, Msg::ReconnectWebSocket)),
                );
            }
        }
        Msg::WebSocketFailed => {
            log!("WebSocket failed");
            if model.web_socket_reconnector.is_none() {
                model.web_socket_reconnector =
                    Some(orders.stream_with_handle(streams::backoff(None, Msg::ReconnectWebSocket)))
            }
        }
        Msg::ReconnectWebSocket(retries) => {
            log!("Reconnect attempt:", retries);
            model.web_socket = create_websocket(orders);
        }
        //
        // Handles input change state
        //
        Msg::InputTextNameChanged(input_text) => {
            model.input_text_name = input_text;
        }
        Msg::InputTextAuthorChanged(input_text) => {
            model.input_text_author = input_text;
        }
    }
}

// ------ ------
//     View
// ------ ------
fn view(model: &Model) -> Node<Msg> {
    div![
        style! {
            St::Color => "#5730B3",
            St::BackgroundColor => "#1B1B21",
            St::Margin => "auto",
            St::Display => "flex",
            St::FlexDirection => "column",
            St::JustifyContent => "center",
            St::AlignItems => "center",
            St::Width => vh(100),
            St::Height => vh(100),
        },
        //
        // HEADER
        //
        div![
            h1!["SecureTheBox Client", C!["title"]],
            div!["Check console log (should be subscribed)"],
            C!["container"],
        ],
        //
        // Create Book
        //
        div![
            h3!["Create Book", C!["description"]],
            div![
                //
                // Text Input
                //
                input![
                    id!("text_input_name"),
                    attrs! {
                        At::Type => "text",
                        At::Value => model.input_text_name,
                        At::Placeholder => "book name",
                    },
                    //
                    // Local State Management
                    //
                    input_ev(Ev::Input, Msg::InputTextNameChanged),
                    C!["input"],
                ],
                input![
                    id!("text_input_author"),
                    attrs! {
                        At::Type => "text",
                        At::Value => model.input_text_author,
                        At::Placeholder => "book author",
                    },
                    input_ev(Ev::Input, Msg::InputTextAuthorChanged),
                    C!["input"],
                ],
                //
                // Button Click to trigger function
                //
                button![
                    "Create Book",
                    ev(Ev::Click, {
                        let name = model.input_text_name.to_owned();
                        let author = model.input_text_author.to_owned();
                        move |_| Msg::BookCreatedClick(name.to_string(), author.to_string())
                    }),
                    style! {
                        St::Color => "#1B1B21",
                        St::BackgroundColor => "#8F5BDE",
                    },
                    C!["button"]
                ]
            ],
            C!["container"]
        ],
        // Websocket Section
        // Use for scoring engine, server status
        if model.web_socket.state() == web_socket::State::Open {
            div![
                C!["container"],
                //
                // Close Socket (temp)
                //
                button![
                    ev(Ev::Click, |_| Msg::CloseWebSocket),
                    "Close",
                    style! {
                        St::Color => "#1B1B21",
                        St::BackgroundColor => "#8F5BDE",
                    },
                    C!["button"]
                ],
            ]
        } else {
            div![p![em!["Connecting or closed"]], C!["container"]]
        },
        //
        // Footer
        //
        footer![
            C!["container"],
            p![format!("{} messages", model.messages.len())],
            p![format!("{} messages sent", model.sent_messages_count)],
            //
            // Map websocket messages
            //
            model.messages.iter().map(|message| p![message]),
        ],
    ]
}

// ------ ------
//     Start
// ------ ------

#[wasm_bindgen(start)]
pub fn start() {
    App::start("app", init, update, view);
}
