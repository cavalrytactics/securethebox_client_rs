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

struct Model {
    sent_messages_count: usize,
    // messages: Vec<String>,
    messages: Vec<Message>,
    input_text_name: String,
    input_text_author: String,
    selected_name: Option<Name>,
    selected_author: Option<Author>,
    web_socket: WebSocket,
    web_socket_reconnector: Option<StreamHandle>,
    books: Option<Vec<q_books::QBooksBooks>>,
}

// Parse GraphQL Subscription Message
#[derive(serde::Deserialize)]
pub struct Message {
    id: String,
    name: String,
    author: String,
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
                //
                // Start GraphQL Subscription Query
                //
                model
                    .web_socket
                    .send_json(&shared::ClientMessageGQLPay {
                        // Set ID of this subscription
                        id: "some_id".to_string(),
                        r#type: "start".to_string(),
                        payload: {
                            shared::Payload {
                                query: "subscription {
                                    books(mutationType: CREATED) {
                                        id,
                                        name,
                                        author
                                    }
                                }".to_string(),
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
            match json_message["type"].to_string().as_str() {
                "\"data\"" => {
                    //
                    // Add message to stack
                    //
                    model
                        .messages
                        .push(Message{
                            id: json_message["payload"]["data"]["books"]["id"].to_string(),
                            name: json_message["payload"]["data"]["books"]["name"].to_string(),
                            author: json_message["payload"]["data"]["books"]["author"].to_string()
                        });
                    log!("Store payload:", json_message);
                }
                "\"connection_ack\"" => {
                    log!("Websocket: Connected");
                }
                // Default Catch all
                _ => {
                    //
                    // Unknown type
                    //
                    log!("wut Payload:", json_message);
                    log!("wut Type:",json_message["type"]);
                }
            }
        }
        Msg::CloseWebSocket => {
            model.web_socket_reconnector = None;
            model
                .web_socket
                .close(None, Some("user clicked Close button"))
                .unwrap();
        }
        Msg::WebSocketClosed(close_event) => {
            log!("================================");
            log!("WebSocket connection was closed:");
            log!("Clean:", close_event.was_clean());
            log!("Code:", close_event.code());
            log!("Reason:", close_event.reason());
            log!("================================");

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
            St::BackgroundColor => "#282a36",
            St::Height => vh(100),
        },
        //
        // HEADER
        //
        nav![C!["navbar navbar-expand-lg navbar-dark bg-dark"],
            a![C!["navbar-brand"], "SECURETHEBOX",
                style!{
                    St::Color => "#50fa7b",
                }
            ],
            button![C!["navbar-toggler"],
                attrs! {
                    At::Type => "button",
                },
                span![C!["navbar-toggler-icon"]]
            ],
            div![C!["collapse navbar-collapse"],
                ul![C!["navbar-nav mr-auto"],
                    li![C!["nav-item active"],
                        a![C!["nav-link"], "Home"]
                    ]
                ],
                form![C!["form-inline my-2 my-lg-0"],
                    button![C!["btn btn-secondary mr-sm-2"], "Sign Up",
                        attrs! {
                            At::Type => "button",
                        },
                        style! {
                            St::BackgroundColor => "#9580ff"
                        }
                    ],
                    button![C!["btn btn-secondary mr-sm-2"], "Log in",
                        attrs! {
                            At::Type => "button"
                        },
                        style! {
                            St::BackgroundColor => "#50fa7b"
                        }
                    ],
                ]
            ],
        ],
        //
        // Create Book
        //
        div![C!["container"],
            h3![C!["description"], "Create Book",
                style!{
                    St::Color => "#50fa7b"
                },
            ],
            form![
                div![C!["form-group"],
                    label!["Book Name"],
                    style![
                        St::Color => "#9580ff",
                    ],
                    input![C!["form-control"],
                        id!("text_input_name"),
                        attrs! {
                            At::Type => "text",
                            At::Value => model.input_text_name,
                            At::Placeholder => "book name",
                        },
                        input_ev(Ev::Input, Msg::InputTextNameChanged),
                    ],
                ],
                div![C!["form-group"],
                    label!["Book Author"],
                    style![
                        St::Color => "#9580ff",
                    ],
                    input![C!["form-control"],
                        id!("text_input_author"),
                        attrs! {
                            At::Type => "text",
                            At::Value => model.input_text_author,
                            At::Placeholder => "book author",
                        },
                        input_ev(Ev::Input, Msg::InputTextAuthorChanged),
                    ],
                ],
            ],
            div![
                //
                // Button Click to trigger function
                //
                button![C!["btn"], "Create Book",
                    ev(Ev::Click, {
                        let name = model.input_text_name.to_owned();
                        let author = model.input_text_author.to_owned();
                        move |_| Msg::BookCreatedClick(name.to_string(), author.to_string())
                    }),
                    style! {
                        St::BackgroundColor => "#50fa7b",
                    }
                ]
            ],
            div![
                //
                // Scoring
                //
                table![C!["table table-bordered table-dark"],
                    thead![
                        tr![
                            th![ attrs! { At::Scope => "col", }, "#" ],
                            th![ attrs! { At::Scope => "col", }, "ID" ],
                            th![ attrs! { At::Scope => "col", }, "Name" ],
                            th![ attrs! { At::Scope => "col", }, "Author" ],
                        ]
                    ],
                    tbody![
                        model.messages.iter().map(|message| 
                            tr![
                                th![ attrs! { At::Scope => "col", }, "1" ],
                                td![ attrs! { At::Scope => "col", }, format!("{}",message.id) ],
                                td![ attrs! { At::Scope => "col", }, format!("{}",message.name) ],
                                td![ attrs! { At::Scope => "col", }, format!("{}",message.author) ],
                                style![
                                    St::Color => "#FFFFFF",
                                ],
                            ]
                        ),
                        
                    ]
                ],
                style! {
                    St::MarginTop => px(15),
                }
            ],
        ],
        // Footer
        //
        div![
            C!["container"],
            p![format!("{} messages", model.messages.len()),
                style![
                    St::Color => "#FFFFFF",
                ],
            ],
            // p![format!("{} messages sent", model.sent_messages_count),
            //     style![
            //         St::Color => "#9580ff",
            //     ],
            // ],
            //
            // Map websocket messages
            //
            // ul![model.messages.iter().map(|message| li![message]),
            //     style![
            //         St::Color => "#FFFFFF",
            //     ],
            // ]
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
