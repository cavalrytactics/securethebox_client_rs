use yew::agent::Threaded;

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    securethebox_client_rs::native_worker::Worker::register();
}
