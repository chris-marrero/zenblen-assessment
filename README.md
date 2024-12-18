To run:
1. Install Rust.
2. Install Postgres
3. Create a database called `db` by calling `createdb db` in the terminal. If that name is taken, url for postgres db can be set in calmram_server/Rocket.toml
4. Connect with `psql db` and create a table with the command below:
```
CREATE TABLE orders (
    time TIMESTAMP WITH TIME ZONE,
    price MONEY
);
```
5. Open terminal in calmram_server and call `cargo run`
6. After the server is up, open a new terminal in calmram_client and call `cargo run`. Images must first be downloaded, so quit on intial boot and call `cargo run` again. Experience is best in full screen.
