use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    f32::consts::PI,
    fs,
    thread::sleep,
    time::Duration,
    vec,
};

use calmram_lib::{Base, BaseId, Config, Menu, Order, SpiceLevel, Toppings, ToppingsId};
use iced::{
    alignment::{Horizontal, Vertical},
    daemon::Appearance,
    font::{self, load, Family},
    futures::{executor::ThreadPool, future::join_all},
    padding::{bottom, left, top},
    theme::Palette,
    widget::{
        button, column, container, horizontal_rule, row, stack, text, vertical_rule, Image, Row,
        Space,
    },
    window::{change_mode, Mode},
    Color, ContentFit, Element, Font,
    Length::{self, Fill, FillPortion, Shrink},
    Padding, Pixels, Radians, Rotation, Settings, Task, Theme,
};
use iced_fonts::{nerd::icon_to_string, Nerd, NERD_FONT, NERD_FONT_BYTES};
use serde::{Deserialize, Serialize};
use serde_json::json;
use websocket::{codec::ws, stream::sync::NetworkStream};

const SERVER_URL: &str = "localhost:8000";
const CHILL_FONT: Font = Font {
    family: Family::Name("Chill Script"),
    style: font::Style::Italic,
    weight: font::Weight::Normal,
    stretch: font::Stretch::Normal,
};

#[derive(Debug, Clone)]
enum Page {
    Menu,
    Order,
    OrderComplete,
}

struct State {
    config: Config,
    current_order: Order,
    current_page: Page,
    ws_server: websocket::sync::Client<Box<dyn NetworkStream + Send>>,
}

#[derive(Debug, Clone)]
enum Message {
    SelectBase(BaseId),
    ToggleTopping(ToppingsId),
    SelectSpiceLevel(i32),
    SetPage(Page),
    Reset,
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::SelectBase(base_id) => {
            state.current_order.base = base_id;
            Task::none()
        }
        Message::ToggleTopping(topping_id) => {
            if state.current_order.toppings.contains(&topping_id) {
                state.current_order.toppings.retain(|id| id != &topping_id);
            } else {
                state.current_order.toppings.push(topping_id);
            }
            Task::none()
        }
        Message::SelectSpiceLevel(spice_level_id) => {
            state.current_order.spice_level = spice_level_id;
            Task::none()
        }
        Message::SetPage(page) => {
            state.current_page = page.clone();
            if let Page::OrderComplete = page {
                state
                    .ws_server
                    .send_message(&websocket::Message::text(
                        json!(state.current_order).to_string(),
                    ))
                    .unwrap();
                Task::done(Message::Reset).chain(Task::future(async {
                    sleep(Duration::from_secs(5));
                    Message::SetPage(Page::Menu)
                }))
            } else {
                Task::none()
            }
        }
        Message::Reset => {
            state.current_order = state.config.default_order.clone();
            Task::none()
        }
    }
}

fn base_button<'a>(state: &'a State, base: &Base) -> Element<'a, Message> {
    let image = container(
        Image::new(format!("assets/{}", base.image_url))
            .height(Length::FillPortion(3))
            .width(Length::Fill)
            .content_fit(ContentFit::Fill),
    )
    .padding(10);

    let name = text(base.name.clone())
        .font(CHILL_FONT)
        .align_x(Horizontal::Center)
        .size(40);

    let price = text(format!("${:.2}", base.price))
        .font(CHILL_FONT)
        .size(40);

    let details = column![name, price,]
        .padding(top(10))
        .height(Fill)
        .width(Fill)
        .align_x(Horizontal::Center);

    let button = button(column![image, details])
        .on_press(Message::SelectBase(base.id))
        .width(Fill)
        .height(Fill)
        .style(|_, _| button::Style {
            background: None,
            ..Default::default()
        });

    if state.current_order.base == base.id {
        container(button)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(Color::from_rgba(
                    0.0, 0.0, 0.0, 0.2,
                ))),
                ..Default::default()
            })
            .into()
    } else {
        button.into()
    }
}

fn step_header(title: &str, icon: Nerd) -> Element<Message> {
    container(row![
        text(icon_to_string(icon))
            .font(NERD_FONT)
            .size(70)
            .height(Fill)
            .color(Color::BLACK)
            .align_y(Vertical::Center),
        container(text(title).font(CHILL_FONT).size(60).color(Color::BLACK))
            .padding(Padding {
                top: 10.0,
                left: 30.0,
                ..Default::default()
            })
            .height(Fill)
            .align_y(Vertical::Center),
    ])
    .padding(left(20))
    .align_y(Vertical::Center)
    .height(FillPortion(1))
    .into()
}

fn base_step_view(state: &State) -> Element<Message> {
    let header = step_header("Choose your base", Nerd::NumericOneCircle);

    let body = row(state
        .config
        .menu
        .bases
        .iter()
        .map(|base| base_button(state, base)))
    .height(FillPortion(4))
    .align_y(Vertical::Center);

    column![header, body].height(Fill).into()
}

fn topping_button<'a>(state: &'a State, topping: &Toppings) -> Element<'a, Message> {
    let image = container(
        Image::new(format!("assets/{}", topping.image_url))
            .height(Length::FillPortion(3))
            .width(Length::Fill)
            .content_fit(ContentFit::Contain),
    )
    .padding(10);

    let name = text(topping.name.clone())
        .font(CHILL_FONT)
        .align_x(Horizontal::Center)
        .size(40);

    let price: Element<Message> = if let Some(price) = topping.price.filter(|p| p > &0.0) {
        text(format!("${:.2}", price))
            .font(CHILL_FONT)
            .size(40)
            .into()
    } else {
        Space::with_height(Pixels(40.0)).into()
    };

    let details = column![name, price,]
        .padding(top(10))
        .height(Fill)
        .width(Fill)
        .align_x(Horizontal::Center);

    let button = button(column![image, details])
        .on_press(Message::ToggleTopping(topping.id))
        .width(Fill)
        .height(Fill)
        .style(|_, _| button::Style {
            background: None,
            ..Default::default()
        });

    if state.current_order.toppings.contains(&topping.id) {
        container(button)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(Color::from_rgba(
                    0.0, 0.0, 0.0, 0.2,
                ))),
                ..Default::default()
            })
            .into()
    } else {
        button.into()
    }
}

fn toppings_step_view(state: &State) -> Element<Message> {
    let header = step_header("Choose your toppings", Nerd::NumericTwoCircle);

    let num_toppings = state.config.menu.toppings.len();
    let top_toppings = state.config.menu.toppings[..num_toppings / 2].to_vec();
    let bottom_toppings = state.config.menu.toppings[num_toppings / 2..].to_vec();

    let top_row = row(top_toppings
        .iter()
        .map(|topping| topping_button(state, topping)));

    let bottom_row = row(bottom_toppings
        .iter()
        .map(|topping| topping_button(state, topping)));

    let body = column![top_row, bottom_row].height(FillPortion(4));

    column![header, body].height(Fill).into()
}

fn spice_level_button<'a>(state: &'a State, spice_level: &SpiceLevel) -> Element<'a, Message> {
    let image = container(
        Image::new(format!("assets/spice.png"))
            .height(Fill)
            .width(Length::Fill)
            .content_fit(ContentFit::ScaleDown),
    );

    let button = button(image)
        .on_press(Message::SelectSpiceLevel(spice_level.level))
        .width(Fill)
        .height(Fill)
        .style(|_, _| button::Style {
            background: None,
            ..Default::default()
        });

    if spice_level.level <= state.current_order.spice_level {
        container(button)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(Color::from_rgba(
                    0.0, 0.0, 0.0, 0.2,
                ))),
                ..Default::default()
            })
            .into()
    } else {
        button.into()
    }
}

fn spice_level_view(state: &State) -> Element<Message> {
    let header = step_header("Choose your spice level", Nerd::NumericThreeCircle);

    let buttons = row(state
        .config
        .menu
        .spice_levels
        .iter()
        .map(|spice_level| spice_level_button(state, spice_level)))
    .height(FillPortion(4))
    .align_y(Vertical::Center);

    let spice_level = state
        .config
        .menu
        .spice_levels
        .iter()
        .find(|spice_level| spice_level.level == state.current_order.spice_level)
        .unwrap();

    let body = row!(
        container(buttons).width(FillPortion(3)),
        container(
            text(spice_level.name.clone())
                .font(CHILL_FONT)
                .size(40)
                .color(Color::BLACK)
        )
        .width(FillPortion(2))
        .padding(20)
        .height(Fill)
        .align_y(Vertical::Center)
    )
    .height(FillPortion(3));

    column![header, body].height(Fill).into()
}

fn next_button(state: &State) -> Element<Message> {
    button(
        text(icon_to_string(Nerd::ChevronRight))
            .font(NERD_FONT)
            .size(100)
            .height(Fill)
            .width(Fill)
            .center(),
    )
    .on_press(Message::SetPage(Page::Order))
    .style(|_, _| button::Style {
        background: Some(iced::Background::Color(Color::from_rgb8(219, 84, 97))),
        text_color: Color::BLACK,
        ..Default::default()
    })
    .height(Fill)
    .width(Fill)
    .into()
}

fn menu_view(state: &State) -> Element<Message> {
    row![
        column![
            container(row![
                Space::with_width(Length::FillPortion(1)),
                Image::new("assets/Logo.png")
                    .rotation(Rotation::Solid(Radians(-PI / 16.0)))
                    .width(Length::FillPortion(10)),
                Space::with_width(Length::FillPortion(1)),
            ])
            .height(Length::Fill)
            .center(Length::Fill),
            horizontal_rule(2),
            base_step_view(&state)
        ]
        .width(Length::FillPortion(1))
        .height(Length::Fill),
        vertical_rule(2),
        column![
            container(toppings_step_view(&state)).height(FillPortion(3)),
            horizontal_rule(2),
            row![
                container(spice_level_view(&state)).width(FillPortion(5)),
                vertical_rule(2),
                container(next_button(&state)).width(FillPortion(1))
            ]
            .height(FillPortion(2))
        ]
    ]
    .into()
}

fn preview_order(state: &State) -> Element<Message> {
    let base = state
        .config
        .menu
        .bases
        .iter()
        .find(|base| base.id == state.current_order.base)
        .unwrap();

    let base_element = text(base.name.clone())
        .font(CHILL_FONT)
        .size(80)
        .color(Color::BLACK)
        .align_x(Horizontal::Center);

    let toppings: Vec<&Toppings> = state
        .current_order
        .toppings
        .iter()
        .map(|topping| {
            state
                .config
                .menu
                .toppings
                .iter()
                .find(|t| t.id == *topping)
                .unwrap()
        })
        .collect();
    let num_toppings = toppings.len();

    let toppings_element: Element<Message> = if num_toppings > 4 {
        let top_row = row(toppings[..num_toppings / 2]
            .iter()
            .map(|topping| Image::new(format!("assets/{}", topping.image_url)).into()));

        let bottom_row = row(toppings[num_toppings / 2..]
            .iter()
            .map(|topping| Image::new(format!("assets/{}", topping.image_url)).into()));

        column!(top_row, bottom_row).into()
    } else {
        row(toppings
            .iter()
            .map(|topping| Image::new(format!("assets/{}", topping.image_url)).into()))
        .into()
    };

    let spice_level_element = row((0..=state.current_order.spice_level).map(|_| {
        Image::new("assets/spice.png")
            .content_fit(ContentFit::Contain)
            .into()
    }));

    column![base_element, toppings_element, spice_level_element,]
        .align_x(Horizontal::Center)
        .into()
}

fn order_summary_view(state: &State) -> Element<Message> {
    let base = state
        .config
        .menu
        .bases
        .iter()
        .find(|base| base.id == state.current_order.base)
        .unwrap();

    let base_item = (base.name.clone(), base.price);

    let topping_items = state
        .current_order
        .toppings
        .iter()
        .filter_map(|topping_id| {
            let topping = state
                .config
                .menu
                .toppings
                .iter()
                .find(|t| t.id == *topping_id)
                .unwrap();
            if let Some(price) = topping.price.filter(|p| p > &0.0) {
                Some((topping.name.clone(), price))
            } else {
                None
            }
        });

    let items = vec![base_item].into_iter().chain(topping_items);
    let total = items.clone().fold(0.0, |acc, (_, price)| acc + price);

    container(
        column(
            items
                .map(|(name, price)| {
                    text(format!("{name} - ${:.2}", price))
                        .font(CHILL_FONT)
                        .size(40)
                        .width(Fill)
                        .align_x(Horizontal::Right)
                        .color(Color::BLACK)
                        .into()
                })
                .chain(vec![horizontal_rule(2).into()])
                .chain(vec![text(format!("Total: ${:.2}", total))
                    .font(CHILL_FONT)
                    .size(40)
                    .width(Fill)
                    .align_x(Horizontal::Right)
                    .color(Color::BLACK)
                    .into()]),
        )
        .width(Fill)
        .height(Shrink)
        .padding(20),
    )
    .align_y(Vertical::Bottom)
    .width(Fill)
    .height(Fill)
    .into()
}

fn pay_view() -> Element<'static, Message> {
    button(row![
        column![
            Image::new("assets/applepay.png")
                .content_fit(ContentFit::Contain)
                .width(Fill)
                .height(Fill),
            Image::new("assets/googlepay.png")
                .content_fit(ContentFit::Contain)
                .width(Fill)
                .height(Fill),
        ],
        text("or Tap/Insert")
            .font(CHILL_FONT)
            .size(40)
            .color(Color::BLACK)
            .width(Fill)
            .height(Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center),
    ])
    .on_press(Message::SetPage(Page::OrderComplete))
    .into()
}

fn order_view(state: &State) -> Element<Message> {
    let order_preview = column![
        container(step_header("Your Order", Nerd::NumericFourCircle)).height(FillPortion(1)),
        horizontal_rule(2),
        stack!(
            Image::new("assets/bowl.png")
                .content_fit(ContentFit::Contain)
                .width(Fill)
                .height(Fill),
            container(preview_order(state)).center(Fill)
        )
        .height(FillPortion(8))
    ]
    .width(FillPortion(3));

    let order_summary_and_pay = column![
        container(order_summary_view(state)).height(FillPortion(6)),
        container(pay_view()).height(Fill)
    ]
    .width(FillPortion(1))
    .height(Fill);

    row![order_preview, vertical_rule(2), order_summary_and_pay,].into()
}

fn order_complete_view() -> Element<'static, Message> {
    container(
        column![
            container(Image::new("assets/Logo.png").rotation(Rotation::Solid(Radians(-PI / 16.0)))),
            // .align_y(Vertical::Bottom),
            container(
                text("Thank you for your order!")
                    .font(CHILL_FONT)
                    .size(80)
                    .color(Color::BLACK)
                    .align_x(Horizontal::Center)
            )
        ]
        .height(Fill)
        .width(Fill)
        .align_x(Horizontal::Center),
    )
    .height(Fill)
    .width(Fill)
    .align_y(Vertical::Center)
    .into()
}

fn view(state: &State) -> Element<Message> {
    stack!(
        Image::new("assets/background.png").content_fit(ContentFit::Fill),
        match state.current_page {
            Page::Menu => menu_view(state),
            Page::Order => order_view(state),
            Page::OrderComplete => order_complete_view(),
        }
    )
    .into()
}

async fn fetch_asset(url: String) {
    let asset = surf::get(format!("http://{SERVER_URL}/assets/{url}"))
        .await
        .unwrap()
        .body_bytes()
        .await
        .unwrap();
    fs::write(format!("assets/{url}"), asset).unwrap();
}

fn fetch_all_assets(config: Config) -> Task<()> {
    let mut requests = vec![];
    let existing_assets: HashSet<String> = fs::read_dir("assets")
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();

    for topping in config.menu.toppings {
        if !existing_assets.contains(&topping.image_url) {
            requests.push(fetch_asset(topping.image_url.clone()));
        }
    }

    for base in config.menu.bases {
        if !existing_assets.contains(&base.image_url) {
            requests.push(fetch_asset(base.image_url.clone()));
        }
    }

    let final_assets = vec![
        "Logo.png",
        "background.png",
        "spice.png",
        "bowl.png",
        "applepay.png",
        "googlepay.png",
    ];
    for asset in final_assets {
        if !existing_assets.contains(&asset.to_owned()) {
            requests.push(fetch_asset(asset.to_owned()));
        }
    }

    if requests.len() > 0 {
        Task::future(async {
            join_all(requests).await;
        })
    } else {
        Task::none()
    }
}

fn main() -> iced::Result {
    let mut ws_server = websocket::ClientBuilder::new("ws://127.0.0.1:8000/kiosk")
        .unwrap()
        .connect(None)
        .unwrap();

    ws_server
        .send_message(&websocket::Message::text("config"))
        .unwrap();

    let config: Config = match ws_server.recv_message().unwrap() {
        websocket::OwnedMessage::Text(text) => serde_json::from_str(&text).unwrap(),
        _ => panic!("Unexpected message type"),
    };

    iced::application("CalmRam Client", update, view)
        .settings(Settings {
            fonts: vec![
                Cow::Borrowed(include_bytes!("../assets/ChillScript.ttf")),
                Cow::Borrowed(NERD_FONT_BYTES),
            ],
            antialiasing: true,
            ..Default::default()
        })
        .theme(|_| {
            let mut palette = Palette::LIGHT;
            palette.background = Color::TRANSPARENT;
            Theme::custom("CalmRam Theme".to_string(), palette)
        })
        .run_with(move || {
            let fetch_images = fetch_all_assets(config.clone());
            let window_id = iced::window::get_latest();

            let current_order = config.default_order.clone();
            (
                State {
                    config,
                    current_order,
                    current_page: Page::Menu,
                    ws_server,
                },
                fetch_images
                    .discard()
                    .chain(window_id)
                    .map(|id| change_mode::<()>(id.unwrap(), Mode::Fullscreen))
                    .discard(),
            )
        })
}
