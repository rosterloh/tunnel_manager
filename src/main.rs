#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use freya::prelude::*;
use tokio::{process::Child, runtime::Builder};

use tunnel_manager::aws::connect_to_tunnel;

const ICON: &[u8] = include_bytes!("../assets/icon.png");
const LOGO: &[u8] = include_bytes!("../assets/logo.svg");
const WINDOW_TITLE: &str = concat!("Gardin Tunnel Manager v", env!("CARGO_PKG_VERSION"));

fn main() {
    let rt = Builder::new_multi_thread().enable_all().build().unwrap();
    let _rt = rt.enter();

    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new(app)
                .with_title(WINDOW_TITLE)
                .with_size(430., 120.)
                // .with_min_size(430., 120.)
                // .with_max_size(430., 120.)
                .with_icon(LaunchConfig::window_icon(ICON)),
        ),
    )
}

#[derive(PartialEq)]
struct GardinLogo;

impl Component for GardinLogo {
    fn render(&self) -> impl IntoElement {
        SvgViewer::new(LOGO)
            .width(Size::px(70.))
            .height(Size::px(50.))
            .margin((0., 18., 0., 0.))
    }
}

#[derive(PartialEq)]
struct DeviceInput {
    device_id: State<String>,
}

impl Component for DeviceInput {
    fn render(&self) -> impl IntoElement {
        rect()
            .width(Size::flex(1.))
            .height(Size::percent(100.))
            .center()
            .spacing(10.)
            .child("Device ID")
            .child(
                Input::new(self.device_id)
                    // .placeholder("G111000")
                    // .on_validate(|validator: InputValidator| {
                    //     validator.set_valid(validator.text().parse::<u8>().is_ok())
                    // })
                    .width(Size::fill()),
            )
    }
}

#[derive(PartialEq)]
struct ConnectButton {
    device_id: State<String>,
    proxy_process: State<Option<Child>>,
}

impl Component for ConnectButton {
    fn render(&self) -> impl IntoElement {
        let device_id = self.device_id;
        let mut proxy_process = self.proxy_process;
        let mut loading = use_state(|| false);
        let mut connected = use_state(|| false);
        // TODO: Make this an enum rather
        let mut show_popup = use_state(String::new);

        rect()
            .width(Size::flex(1.))
            .height(Size::percent(100.))
            .horizontal()
            .center()
            .spacing(10.)
            .child(
                Button::new()
                    .filled()
                    .background((137, 188, 43))
                    .hover_background((117, 168, 23))
                    .color(Color::BLACK)
                    .on_press(move |_| {
                        spawn(async move {
                            if *connected.peek() {
                                let mut child = proxy_process.take().unwrap();
                                if child.kill().await.is_err() {
                                    show_popup.set(String::from("Failed to kill proxy process"));
                                }
                                proxy_process.set(None);
                                connected.set(false);
                                return;
                            }

                            let id = device_id.peek().clone();
                            if id.is_empty() {
                                show_popup.set(String::from("No Device"));
                                return;
                            }
                            loading.set(true);
                            match connect_to_tunnel(&id).await {
                                Ok(child) => {
                                    connected.set(true);
                                    proxy_process.set(Some(child));
                                    // let _ = proxy_process.take().unwrap().wait().await;
                                }
                                Err(e) => show_popup.set(e),
                            }
                            loading.set(false);
                        });
                    })
                    .child(if *connected.read() {
                        "Disconnect"
                    } else {
                        "Connect"
                    }),
            )
            .maybe_child(loading.read().then(CircularLoader::new))
            .child(
                Popup::new()
                    .on_close_request(move |_| show_popup.write().clear())
                    .maybe(!show_popup.read().is_empty(), |popup| {
                        popup.child(PopupContent::new().child(
                            if show_popup.read().as_str() == "No Device" {
                                "Device ID cannot be empty"
                            } else {
                                ""
                            },
                        ))
                    }),
            )
    }
}

fn app() -> impl IntoElement {
    use_init_theme(dark_theme);

    let device_id = use_state(String::new);
    let proxy_process = use_state(|| Option::<Child>::None);

    rect()
        .expanded()
        .horizontal()
        .content(Content::flex())
        .padding(24.)
        .theme_background()
        .theme_color()
        .child(GardinLogo)
        .child(DeviceInput { device_id })
        .child(ConnectButton {
            device_id,
            proxy_process,
        })
}
