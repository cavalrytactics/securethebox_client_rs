# securethebox_client_rs

- Install requirements
```
cargo install wasm-bindgen-cli
```

- build main
```
cargo build --target wasm32-unknown-unknown --bin securethebox_client_rs_app
wasm-bindgen --target web --no-typescript --out-dir static/ --out-name app target/wasm32-unknown-unknown/debug/securethebox_client_rs_app.wasm
```

- build web worker
```
cargo build --target wasm32-unknown-unknown --bin securethebox_client_rs_worker
wasm-bindgen --target no-modules --no-typescript --out-dir static/ --out-name worker target/wasm32-unknown-unknown/debug/securethebox_client_rs_worker.wasm
```

- start web server
```
cargo web start
```

## build for prod
- build files target/deploy path
```
cargo web deploy
```

## firebase hosting deploy
```
firebase login
firebase deploy 
```