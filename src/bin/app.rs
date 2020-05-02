fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::start_app::<securethebox_client_rs::Model>();
}
