use graphql_client::{GraphQLQuery, Response as GQLResponse};

use seed::{prelude::*, *};
use serde::{Deserialize, Serialize};

mod shared;

// Global types and Constant values
type Id = String;
type Name = String;
type Author = String;
type Points = String;

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
generate_query!(MUpdateBook);
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
        // books: Option::Some(Vec::new()),
        messages: Vec::new(),
        input_text_name: String::new(),
        input_text_author: String::new(),
        input_text_points: String::new(),
        web_socket: create_websocket(orders),
        web_socket_reconnector: None,
        selected_name: std::default::Default::default(),
        selected_author: std::default::Default::default(),
        selected_points: std::default::Default::default(),
        selected_id: std::default::Default::default(),
        seconds: 0,
        timer_handle: Some(orders.stream_with_handle(streams::interval(100, || Msg::OnTick))),
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
    messages: Vec<Message>,
    input_text_name: String,
    input_text_author: String,
    input_text_points: String,
    selected_name: Option<Name>,
    selected_author: Option<Author>,
    selected_points: Option<Points>,
    selected_id: Option<Id>,
    web_socket: WebSocket,
    web_socket_reconnector: Option<StreamHandle>,
    // books: Option<Vec<q_books::QBooksBooks>>,
    seconds: u32,
    timer_handle: Option<StreamHandle>,
}

// Parse GraphQL Subscription Message
#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    id: String,
    name: String,
    author: String,
    points: String,
}

// Message from the server to the client.
#[derive(Serialize, Deserialize, Debug)]
pub struct ServerMessage {
    id: String,
    payload: MessagePayload,
    r#type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessagePayload {
    data: PayloadData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PayloadData {
    books: DataBook,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DataBook{
    mutation_type: String,
    id: String,
    name: String,
    author: String,
    points: String,
}

// ------ ------
//    Update
// ------ ------

enum Msg {
    BooksFetched(fetch::Result<GQLResponse<q_books::ResponseData>>),
    BookDeleted(fetch::Result<GQLResponse<q_books::ResponseData>>),
    BookDeletedClick(Id),
    BookUpdated(fetch::Result<GQLResponse<q_books::ResponseData>>),
    BookUpdatedClick(Id, Name, Author, Points),
    BookCreated(fetch::Result<GQLResponse<q_books::ResponseData>>),
    BookCreatedClick(Name, Author, Points),
    WebSocketOpened,
    MessageReceived(WebSocketMessage),
    WebSocketClosed(CloseEvent),
    WebSocketFailed,
    ReconnectWebSocket(usize),
    InputTextNameChanged(String),
    InputTextAuthorChanged(String),
    InputTextPointsChanged(String),
    OnTick,
}

fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        //
        // Interval
        //
        Msg::OnTick => {
            if model.seconds != 100 { model.seconds += 1;
            }
        }
        //
        // GraphQL functions
        //
        Msg::BooksFetched(Ok(GQLResponse {
            data: Some(data), ..
        })) => {
            let vec_books_length = data.books.len();
            for book_index in 0..vec_books_length {
                model.messages.push(
                    Message {
                        id: data.books[book_index].id.to_string(),
                        name: data.books[book_index].name.to_string(),
                        author: data.books[book_index].author.to_string(),
                        points: data.books[book_index].points.to_string(),
                    }
                )
            }
        }
        Msg::BookCreated(Ok(GQLResponse {
            data: Some(_), ..
        })) => {
            log!("Created Book");
        }
        //
        // Trigger query on click
        //
        Msg::BookCreatedClick(name, author, points) => {
            model.selected_name = Some(name.clone());
            model.selected_author = Some(author.clone());
            model.selected_points = Some(points.clone());
            orders.perform_cmd(async {
                Msg::BookCreated(
                    send_graphql_request(&MCreateBook::build_query(m_create_book::Variables {
                        name,
                        author,
                        points,
                    }))
                    .await,
                )
            });
        }
        Msg::BookCreated(error) => log!(error),
        Msg::BookUpdated(Ok(GQLResponse {
            data: Some(_), ..
        })) => {
            log!("Deleted Book");
        }
        Msg::BookUpdatedClick(id, name, author, points) => {
            model.selected_id = Some(id.clone());
            if let Some(index) = model.messages.iter().position(|message| message.id.to_string() == id) {
                model.messages[index] = Message {
                    id: id.clone(),
                    name: name.clone(),
                    author: author.clone(),
                    points: points.clone(),
                }
            }
            orders.perform_cmd(async {
                Msg::BookUpdated(
                    send_graphql_request(&MUpdateBook::build_query(m_update_book::Variables {
                        id,
                        name,
                        author,
                        points,
                    }))
                    .await,
                )
            });
        }
        Msg::BookUpdated(error) => log!(error),
        Msg::BookDeleted(Ok(GQLResponse {
            data: Some(_), ..
        })) => {
            log!("Updated Book");
        }
        Msg::BookDeletedClick(id) => {
            model.selected_id = Some(id.clone());
            if let Some(index) = model.messages.iter().position(|message| message.id.to_string() == id) {
                model.messages.remove(index);
            }
            orders.perform_cmd(async {
                Msg::BookDeleted(
                    send_graphql_request(&MDeleteBook::build_query(m_delete_book::Variables {
                       id
                    }))
                    .await,
                )
            });
        }
        Msg::BookDeleted(error) => log!(error),
        Msg::BooksFetched(error) => log!(error),
        //
        // Websocket functions
        //
        Msg::WebSocketOpened => {
            model.web_socket_reconnector = None;
            {
                model.web_socket
                    .send_json(&shared::ClientMessageGQLInit {
                        r#type: "connection_init".to_string(),
                        payload: shared::PayloadEmp {},
                    })
                    .unwrap();
            }
            //
            // Start GraphQL Subscription Query
            //
            {
                model.web_socket
                    .send_json(&shared::ClientMessageGQLPay {
                        // Set ID of this subscription
                        id: "some_id".to_string(),
                        r#type: "start".to_string(),
                        payload: {
                            shared::Payload {
                                query: "subscription {
                                    books {
                                        mutationType,
                                        id,
                                        name,
                                        author,
                                        points,
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
            let book = &json_message["payload"]["data"]["books"];
            if json_message["type"] == "connection_ack" {
                log!("CONNECTED");
            } else if json_message["type"] == "data" {
                log!("MESSAGE",json_message);
                let mutation_type = book["mutationType"].to_string().replace("\"", "");
                let id = book["id"].to_string().replace("\"", "");
                let name = book["name"].to_string().replace("\"", "");
                let author = book["author"].to_string().replace("\"", "");
                let points = book["points"].to_string().replace("\"", "");
                match mutation_type.as_str() {
                    "CREATED" => {
                        model.messages
                            .push(Message { id, name, author, points });
                    }
                    "UPDATED" => {
                        if let Some(index) = model.messages.iter().position(|message| message.id.to_string() == id) {
                            model.messages[index] = Message {
                                id: id.clone(),
                                name: name.clone(),
                                author: author.clone(),
                                points: points.clone(),
                            }
                        }
                    }
                    "DELETED" => {
                        if let Some(index) = model.messages.iter().position(|message| message.id.to_string() == id) {
                            model.messages.remove(index);
                        }
                    }
                    _ => { }
                }
            }
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
        Msg::InputTextPointsChanged(input_text) => {
            model.input_text_points = input_text;
        }
    }
}

// ------ ------
//     View
// ------ ------
fn view(model: &Model) -> Node<Msg> {
    div![C!["overflow-auto"],
        style! {
            St::BackgroundColor => "#282a36",
            St::Height => vh(100),
        },
        //
        // HEADER
        //
        nav![C!["navbar navbar-expand-lg navbar-dark bg-dark"],
            a![C!["navbar-brand"], "LOGO",
                style!{
                    St::Color => "#50fa7b",
                }
            ],
            button![C!["navbar-toggler"],
                attrs! { At::Type => "button", },
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
                        attrs! { At::Type => "button", },
                        style! {
                            St::BackgroundColor => "#9580ff"
                        }
                    ],
                    button![C!["btn btn-secondary mr-sm-2"], "Log in",
                        attrs! { At::Type => "button" },
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
                    label!["name"],
                    style![
                        St::Color => "#9580ff",
                    ],
                    input![C!["form-control"],
                        id!("text_input_name"),
                        attrs! {
                            At::Type => "text",
                            At::Value => model.input_text_name,
                            At::Placeholder => "name",
                        },
                        input_ev(Ev::Input, Msg::InputTextNameChanged),
                    ],
                ],
                div![C!["form-group"],
                    label!["author"],
                    style![
                        St::Color => "#9580ff",
                    ],
                    input![C!["form-control"],
                        id!("text_input_author"),
                        attrs! {
                            At::Type => "text",
                            At::Value => model.input_text_author,
                            At::Placeholder => "author",
                        },
                        input_ev(Ev::Input, Msg::InputTextAuthorChanged),
                    ],
                ],
                div![C!["form-group"],
                    label!["points"],
                    style![
                        St::Color => "#9580ff",
                    ],
                    input![C!["form-control"],
                        id!("text_input_points"),
                        attrs! {
                            At::Type => "text",
                            At::Value => model.input_text_points,
                            At::Placeholder => "points",
                        },
                        input_ev(Ev::Input, Msg::InputTextPointsChanged),
                    ],
                ],
            ],
            div![C!["container"],
                div![C!["row"],
                    div![C!["col-sm"],
                        // Button Click to trigger CREATE function
                        button![C!["btn"], "Create Book",
                            ev(Ev::Click, {
                                let name = model.input_text_name.to_owned();
                                let author = model.input_text_author.to_owned();
                                let points = model.input_text_points.to_owned();
                                move |_| Msg::BookCreatedClick(name.to_string(), author.to_string(), points.to_string())
                            }),
                            style! {
                                St::BackgroundColor => "#50fa7b",
                            }
                        ],
                    ],
                    //
                    // # of websocket messages
                    //
                    div![C!["col-sm"],
                        p![format!("messages: {}", model.messages.len()),
                            style![
                                St::Color => "#FFFFFF",
                            ],
                        ],
                    ],
                    //
                    // Interval update
                    //
                    div![C!["col-sm"],
                        p![C!["progress"],
                            div![C!["progress-bar progress-bar-striped progress-bar-animated"],
                                style![
                                    St::Width => format!("{}%", model.seconds)
                                ],
                                format!("{}%", model.seconds)
                            ]
                        ],
                    ]
                ],
            ],
            //
            // Scoring
            //
            div![
                table![C!["table table-striped table-bordered table-dark"],
                    thead![
                        tr![
                            th![ attrs! { At::Scope => "col", }, "ID" ],
                            th![ attrs! { At::Scope => "col", }, "Name" ],
                            th![ attrs! { At::Scope => "col", }, "Author" ],
                            th![ attrs! { At::Scope => "col", }, "Points" ],
                            th![ attrs! { At::Scope => "col", }, "Action" ],
                        ]
                    ],
                    tbody![
                        model.messages.iter().map(|message|
                            tr![
                                td![ attrs! { At::Scope => "col", }, format!("{}", message.id) ],
                                td![ attrs! { At::Scope => "col", }, format!("{}", message.name) ],
                                td![ attrs! { At::Scope => "col", }, format!("{}", message.author) ],
                                td![ attrs! { At::Scope => "col", }, format!("{}", message.points) ],
                                td![ attrs! { At::Scope => "col", },
                                    button![C!["btn"], format!("Update"),
                                        attrs!{ At::Value => &message.id },
                                        {
                                            let id = message.id.clone();
                                            let name = "test".to_string();
                                            let author = "test".to_string();
                                            let points = "test".to_string();
                                            ev(Ev::Click, {
                                                move |_| Msg::BookUpdatedClick(id, name, author, points)
                                            })
                                        },
                                        style! {
                                            St::BackgroundColor => "#50fa7b",
                                        }
                                    ],
                                    button![C!["btn"], format!("Delete"),
                                        attrs!{ At::Value => &message.id },
                                        {
                                            let id = message.id.clone();
                                            ev(Ev::Click, {
                                                move |_| Msg::BookDeletedClick(id)
                                            })
                                        },
                                        style! {
                                            St::BackgroundColor => "#50fa7b",
                                        }
                                    ],
                                ],
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
    ]
}

// ------ ------
//     Start
// ------ ------

#[wasm_bindgen(start)]
pub fn start() {
    App::start("app", init, update, view);
}
