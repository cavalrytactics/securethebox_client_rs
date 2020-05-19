# securethebox_client_rs

- Install requirements
```
cargo install cargo-web
cargo install cargo-make
cargo install wasm-bindgen-cli
cargo install graphql_client_cli
cargo install microserver
```

- start web server (locally)
```
cargo make start
OR
pier wasms
```

## build for prod
- build files /pkg path
```
cargo make build_client_release
OR
pier wasmr
```

## firebase hosting deploy
```
firebase login
firebase deploy 
```