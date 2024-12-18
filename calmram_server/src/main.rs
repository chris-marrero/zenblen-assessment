use calmram_lib::{Config, Menu, Order};
use rocket::{time::Date, State};
use rocket_db_pools::{Connection, Database};
use sqlx::{
    self,
    postgres::types::PgMoney,
    types::{time::OffsetDateTime, Decimal},
    Row,
};

use serde::{Deserialize, Serialize};
use serde_json::json;
use ws::Message;

#[macro_use]
extern crate rocket;

#[get("/")]
async fn index(mut db: Connection<Db>) -> String {
    let toppings = sqlx::query("SELECT * FROM toppings")
        .fetch_all(&mut **db)
        .await
        .unwrap();

    toppings
        .iter()
        .fold(String::new(), |acc, row| acc + row.get("name") + " ")
}

#[get("/assets/<file>")]
async fn assets(file: &str) -> Option<rocket::fs::NamedFile> {
    rocket::fs::NamedFile::open(std::path::Path::new("assets/").join(file))
        .await
        .ok()
}

#[get("/kiosk")]
async fn kiosk(
    ws: ws::WebSocket,
    config: &State<Config>,
    mut db: Connection<Db>,
) -> ws::Stream!['_] {
    ws::Stream! { ws =>
        for await message in ws {
            match message? {
                Message::Text(text) => {
                    if text == "config" {
                        yield Message::text(json!(config.inner()).to_string());
                    } else if let Ok(complete_order) = serde_json::from_str::<Order>(&text) {
                        let base_cost = config.menu.bases.iter().find(|base| base.id == complete_order.base).unwrap().price;
                        let toppings_cost = complete_order.toppings.iter().fold(0.0, |acc, topping_id| {
                            let topping = config.menu.toppings.iter().find(|topping| topping.id == *topping_id).unwrap();
                            acc + topping.price.unwrap_or(0.0)
                        });
                        let total = PgMoney::from_decimal(Decimal::from_f32_retain(base_cost + toppings_cost).unwrap(), 2);
                        sqlx::query("INSERT INTO orders (time, price) VALUES ($1, $2)")
                            .bind(OffsetDateTime::now_utc())
                            .bind(total)
                            .execute(&mut **db)
                            .await
                            .unwrap();
                        println!("Received order: {:?}", complete_order);
                    } else {
                        println!("Received unexpected message: {:?}", text);
                    }
                }
                m => println!("Received unexpected message: {:?}", m),
            }
        }
    }
}

#[launch]
fn rocket() -> _ {
    let config: Config = serde_json::from_str(include_str!("../Config.json")).unwrap();
    rocket::build()
        .attach(Db::init())
        .manage(config)
        .mount("/", routes![index, assets, kiosk])
}

#[derive(Database)]
#[database("db")]
struct Db(sqlx::PgPool);
