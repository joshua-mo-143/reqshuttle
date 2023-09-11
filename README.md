# reqshuttle: An example repo for writing a web scraper in Rust, hosted on Shuttle.

## Getting Started
You'll need `cargo-shuttle` installed. You can install it with the following command:
```sh
cargo install cargo-shuttle
```
You will also need a Shuttle API key which you can get from logging in via Github [here.](https://console.shuttle.rs/login) - then you'll need to log in via the `cargo-shuttle` CLI.

To use the repo, copy and then use the following in a terminal (clones the repo then CDs to it):
```sh
git clone https://github.com/joshua-mo-143/reqshuttle.git
cd reqshuttle
```

Then use the following:
```sh
cargo shuttle project start --idle-minutes 0
cargo shuttle deploy
```


