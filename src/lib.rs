use graphql_client::{GraphQLQuery, Response as GQLResponse};
mod shared;
use seed::{prelude::*, *};
use serde::{Deserialize, Serialize};
// Allows sort_by
use itertools::Itertools;
use rasciigraph::{plot, Config};

// Global types and Constant values
type Id = String;
type Name = String;
type Author = String;
type Points = String;

// TODO: Change these urls for production
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
        input_text_points: std::default::Default::default(),
        web_socket: create_websocket(orders),
        web_socket_reconnector: None,
        selected_name: std::default::Default::default(),
        selected_author: std::default::Default::default(),
        selected_points: std::default::Default::default(),
        selected_id: std::default::Default::default(),
        seconds: 0,
        //
        // Generate 10 vec values = 0.0
        //
        graph: vec![0.0; 100],
        problems: vec![],
        timer_handle: Some(orders.stream_with_handle(streams::interval(1000, || Msg::OnTick))),
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
    seconds: i64,
    graph: Vec<f64>,
    problems: Vec<Problem>,
    timer_handle: Option<StreamHandle>,
}

// Parse GraphQL Subscription Message
#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    id: String,
    name: String,
    author: String,
    points: u8,
    problems: Vec<Problem>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Problem {
    letter: String,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DataBook{
    mutation_type: String,
    id: String,
    name: String,
    author: String,
    points: u8,
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
            if model.seconds != 600000 {
                model.seconds += 1000;
                if model.graph.len() < 100 {
                    model.graph.push(1.0)
                } else {
                    // Remove the first duplicate value
                    model.graph.remove(0);
                    model.graph.push(1.0);
                }
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
                        points: data.books[book_index].points.to_string().parse().unwrap(),
                        problems: vec![
                            Problem{ letter:"A".to_string() },
                            Problem{ letter:"B".to_string() },
                        ],
                    }
                );
                // if model.problems.iter().any(|i| i.letter != "A") {
                model.problems.push(
                    Problem{ letter:"A".to_string() },
                );
                // }
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
                    points: points.clone().parse().unwrap(),
                    problems: vec![
                        Problem{ letter:"A".to_string() },
                        Problem{ letter:"B".to_string() },
                    ],
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
                let points = book["points"].to_string().replace("\"", "").parse().unwrap();
                let problems =  vec![
                        Problem{ letter:"A".to_string() },
                        Problem{ letter:"B".to_string() },
                    ];
                match mutation_type.as_str() {
                    "CREATED" => {
                        model.messages
                            .push(Message { id, name, author, points, problems });
                    }
                    "UPDATED" => {
                        if let Some(index) = model.messages.iter().position(|message| message.id.to_string() == id) {
                            model.messages[index] = Message {
                                id: id.clone(),
                                name: name.clone(),
                                author: author.clone(),
                                points: points.clone(),
                                problems: problems.clone(),
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
    let mut clock = shared::Clock::new();
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
                            At::Type => "number",
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
            div![
                // The class required by GitHub styles. See `index.html`.
                C!["markdown-body"],
                style![
                    St::Color => "#9580ff",
                ],
                md!( plot(
                        model.graph.clone(),
                        Config::default().with_offset(10).with_height(10)
                    ).as_str()
                ),
            ],
            //
            // Scoring
            //
            div![
                table![C!["table table-striped table-bordered table-dark"],
                    //
                    // Table headers
                    //
                    thead![
                        tr![
                            th![ C!["text-center"], attrs! { At::Scope => "col", }, "#" ],
                            th![ C!["text-center"], attrs! { At::Scope => "col", }, "ID" ],
                            th![ C!["text-center"], attrs! { At::Scope => "col", }, "Name" ],
                            th![ C!["text-center"], attrs! { At::Scope => "col", }, "Author" ],
                            th![ C!["text-center"], attrs! { At::Scope => "col", }, "Points" ],
                            model.problems.iter().map(| problem |
                                {
                                    th![ C!["text-center"], attrs! { At::Scope => "col", }, a![&problem.letter],br![],span!["100"] ]
                                }
                            ),
                            th![ C!["text-center"], attrs! { At::Scope => "col", }, "Actions" ],
                            
                            // th![ C!["text-center"], attrs! { At::Scope => "col", }, a!["A"],br![],span!["100"] ],
                            // th![ C!["text-center"], attrs! { At::Scope => "col", }, a!["B"],br![],span!["200"] ],
                            // th![ C!["text-center"], attrs! { At::Scope => "col", }, a!["C"],br![],span!["300"] ],
                            // th![ C!["text-center"], attrs! { At::Scope => "col", }, a!["D"],br![],span!["400"] ],
                            // th![ C!["text-center"], attrs! { At::Scope => "col", }, a!["E"],br![],span!["500"] ],
                        ]
                    ],
                    tbody![
                        //
                        // Sort rows in table in decending order by 'points' key/value
                        // Label new position
                        //
                        // GOTCHA 1: using enumerate() after sorted_by well create
                        // sorted index
                        //
                        // GOTCHA 2: enumerate() creates a tuple (index, &thing)
                        // index = 0, &thing = 1
                        //
                        // Position/Index = some.0
                        // Key/Value = some.1.key
                        //
                        model.messages.iter().sorted_by(|a, b| Ord::cmp(&b.points, &a.points)).enumerate().map(| message |
                            {
                                clock.set_time_ms(model.seconds);
                            tr![
                                td![ C!["text-center"], attrs! { At::Scope => "col", }, format!("{}", message.0+1 ) ],
                                td![ C!["text-center"], attrs! { At::Scope => "col", }, format!("{}", message.1.id) ],
                                td![ C!["text-center"], attrs! { At::Scope => "col", }, format!("{}", message.1.name) ],
                                td![ C!["text-center"], attrs! { At::Scope => "col", }, format!("{}", message.1.author) ],
                                td![ C!["text-center"], attrs! { At::Scope => "col", }, format!("{}", message.1.points) ],
                                td![ C!["text-center"], attrs! { At::Scope => "col", }, a![format!("{}", "Score")],br![],span![format!("{}", clock.get_time()) ]],
                                td![ C!["text-center"], attrs! { At::Scope => "col", }, a![format!("{}", "Score")],br![],span![format!("{}", clock.get_time()) ]],
                                td![ C!["text-center"], attrs! { At::Scope => "col", }, a![format!("{}", "Score")],br![],span![format!("{}", clock.get_time()) ]],
                                td![ C!["text-center"], attrs! { At::Scope => "col", }, a![format!("{}", "Score")],br![],span![format!("{}", clock.get_time()) ]],
                                td![ C!["text-center"], attrs! { At::Scope => "col", }, a![format!("{}", "Score")],br![],span![format!("{}", clock.get_time()) ]],
                                td![ C!["text-center"], attrs! { At::Scope => "col", },
                                    button![C!["btn"], format!("Update"),
                                        attrs!{ At::Value => &message.1.id },
                                        {
                                            let id = message.1.id.clone();
                                            let name = "test".to_string();
                                            let author = "test".to_string();
                                            let points = "100".to_string();
                                            ev(Ev::Click, {
                                                move |_| Msg::BookUpdatedClick(id, name, author, points)
                                            })
                                        },
                                        style! {
                                            St::BackgroundColor => "#50fa7b",
                                        }
                                    ],
                                    button![C!["btn"], format!("Delete"),
                                        attrs!{ At::Value => &message.1.id },
                                        {
                                            let id = message.1.id.clone();
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
                            },
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
