use graphql_client::{GraphQLQuery, Response as GQLResponse};

use indexmap::IndexMap;
use seed::{prelude::*, *};
use serde::{Deserialize, Serialize};

mod shared;

const API_URL: &str = "http://c2.local:8000";
const WS_URL: &str = "ws://c2.local:8000";

// ------ ------
//    GraphQL
// ------ ------
macro_rules! generate_query {
    ($query:ident) => {
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
// generate_query!(QBook);
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
    orders.perform_cmd(async {
        // Msg::BooksFetched(send_graphql_request(&QBooks::build_query(q_books::Variables)).await)
        Msg::BookCreated(
            send_graphql_request(&MCreateBook::build_query(m_create_book::Variables {
                name: "AAA".to_string(),
                author: "BBB".to_string(),
            }))
            .await,
        )
    });
    Model {
        books: Option::Some(Vec::new()),
        sent_messages_count: 0,
        messages: Vec::new(),
        input_text: String::new(),
        web_socket: create_websocket(orders),
        web_socket_reconnector: None,
    }
}

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
    input_text: String,
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
    WebSocketOpened,
    MessageReceived(WebSocketMessage),
    CloseWebSocket,
    WebSocketClosed(CloseEvent),
    WebSocketFailed,
    ReconnectWebSocket(usize),
    InputTextChanged(String),
    SendMessage(shared::ClientMessageGQLPay),
}

fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::BooksFetched(Ok(GQLResponse {
            data: Some(data), ..
        })) => {
            model.books = Some(data.books);
        }
        Msg::BookCreated(Ok(GQLResponse {
            data: Some(data), ..
        })) => {
            println!("Created Book");
            // model.books = Some(data.books);
        }
        Msg::BookCreated(error) => log!(error),
        Msg::BooksFetched(error) => log!(error),
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
        Msg::InputTextChanged(input_text) => {
            model.input_text = input_text;
        }
        Msg::SendMessage(msg) => {
            model.web_socket.send_json(&msg).unwrap();
            model.input_text.clear();
            model.sent_messages_count += 1;
        }
    }
}

// ------ ------
//     View
// ------ ------

fn view(model: &Model) -> Vec<Node<Msg>> {
    vec![div![
        //
        // HEADER
        //
        div![
            h1!["SecureTheBox Client", C!["title"]],
            div!["Check console log (should be subscribed)"],
            model.messages.iter().map(|message| p![message]),
            C!["container"],
        ],
        // Create Book
        //
        div![
            h3!["Create Book", C!["description"]],
            div![
                input![
                    id!("text"),
                    attrs! {
                        At::Type => "text",
                        At::Value => model.input_text;
                    },
                    input_ev(Ev::Input, Msg::InputTextChanged),
                    C!["input"],
                ],
                button!["Create", C!["button"]]
            ],
            C!["container"]
        ],
        // Websocket Section
        // Use for scoring engine, server status
        if model.web_socket.state() == web_socket::State::Open {
            div![
                C!["container"],
                input![
                    id!("text"),
                    attrs! {
                        At::Type => "text",
                        At::Value => model.input_text;
                    },
                    input_ev(Ev::Input, Msg::InputTextChanged),
                    C!["input"],
                ],
                button![
                    ev(Ev::Click, {
                        let message_text = model.input_text.to_owned();
                        move |_| {
                            Msg::SendMessage(shared::ClientMessageGQLPay {
                                id: "1".to_string(),
                                r#type: "start".to_string(),
                                payload: {
                                    shared::Payload {
                                        query: message_text.to_string(),
                                    }
                                },
                            })
                        }
                    }),
                    "Send",
                    C!["button"]
                ],
                button![
                    ev(Ev::Click, |_| Msg::CloseWebSocket),
                    "Close",
                    C!["button"]
                ],
            ]
        } else {
            div![p![em!["Connecting or closed"]], C!["container"]]
        },
        // Footer
        footer![
            C!["container"],
            p![format!("{} messages", model.messages.len())],
            p![format!("{} messages sent", model.sent_messages_count)]
        ],
    ]]
}

// ------ ------
//     Start
// ------ ------

#[wasm_bindgen(start)]
pub fn start() {
    App::start("app", init, update, view);
}
