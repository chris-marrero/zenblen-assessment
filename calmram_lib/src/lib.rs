use serde::{Deserialize, Serialize};

pub use i32 as BaseId;
pub use i32 as ToppingsId;

#[derive(Deserialize, Serialize, Clone)]
pub struct Base {
    pub name: String,
    pub price: f32,
    pub image_url: String,
    pub id: BaseId,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Toppings {
    pub name: String,
    pub price: Option<f32>,
    pub image_url: String,
    pub id: ToppingsId,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct SpiceLevel {
    pub name: String,
    pub level: i32,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Menu {
    pub bases: Vec<Base>,
    pub toppings: Vec<Toppings>,
    pub spice_levels: Vec<SpiceLevel>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Order {
    pub base: BaseId,
    pub toppings: Vec<ToppingsId>,
    pub spice_level: i32,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    pub menu: Menu,
    pub default_order: Order,
}
