#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use freya::prelude::*;
use tokio::process::Child;

use tunnel_manager::aws::connect_to_tunnel;

const ICON: &[u8] = include_bytes!("../assets/icon.png");
const LOGO: &[u8] = include_bytes!("../assets/logo.svg");

fn main() {
    launch_cfg(
        app,
        LaunchConfig::<()>::new()
            .with_title("Gardin Tunnel Manager")
            .with_size(430., 120.)
            // .with_min_size(430., 120.)
            // .with_max_size(430., 120.)
            .with_icon(LaunchConfig::load_icon(ICON)),
    )
}

#[component]
fn GardinLogo() -> Element {
    let logo = static_bytes(LOGO);
    rsx!(
        svg {
            width: "70",
            height: "50",
            svg_data: logo,
            margin: "0 18 0 0",
        }
    )
}

#[component]
fn DeviceInput(device_id: Signal<String>) -> Element {
    rsx!(
        rect {
            width: "flex(1)",
            height: "100%",
            main_align: "center",
            cross_align: "center",
            spacing: "10",
            label {
                "Device ID"
            }
            Input {
                value: device_id,
                // placeholder: "G111000",
                width: "fill",
                // onvalidate: |validator: InputValidator| {
                //     validator.set_valid(validator.text().parse::<u8>().is_ok())
                // },
                onchange: move |txt| {
                    device_id.set(txt);
                },
            }
        }
    )
}

#[component]
fn ConnectButton(device_id: Signal<String>, proxy_process: Signal<Option<Child>>) -> Element {
    let mut loading = use_signal(|| false);
    let mut connected= use_signal(|| false);
    // TODO: Make this an enum rather
    let mut show_popup = use_signal(|| String::new());
    
    rsx!(
        rect {
            width: "flex(1)",
            height: "100%",
            direction: "horizontal",
            main_align: "center",
            cross_align: "center",
            spacing: "10",
            FilledButton {
                theme: theme_with!(ButtonTheme {
                    background: "#89BC2B".into(),
                    hover_background: "rgb(117, 168, 23)".into(),
                    font_theme: FontThemeWith {
                        color: Some("black".into()),
                    }
                }),
                onclick: move |_| {
                    spawn(async move {
                        if *connected.read() {
                            let mut child = proxy_process.take().unwrap();
                            if child.kill().await.is_err() {
                                show_popup.set(String::from("Failed to kill proxy process"));
                            }
                            proxy_process.set(Option::None);
                            connected.set(false);
                            return;
                        }
                        
                        if device_id.read().is_empty() {
                            show_popup.set(String::from("No Device"));
                            return;
                        }
                        loading.set(true);
                        let result = connect_to_tunnel(&device_id.read()).await;
                        if result.is_err() {
                            show_popup.set(result.err().unwrap_or_else(|| String::from("Unknown Error")));
                        } else {
                            connected.set(true);
                            proxy_process.set(Some(result.unwrap()));
                            // let _ = proxy_process.take().unwrap().wait().await;
                        }
                        loading.set(false);
                    });
                },
                label { 
                    if *connected.read() {
                        "Disconnect"
                    } else {
                        "Connect"
                    }
                }
            }
            if *loading.read() {
                Loader {}
            }
            if !show_popup.read().is_empty() {
                Popup {
                    oncloserequest: move |_| {
                        show_popup.write().clear()
                    },
                    PopupContent {
                        label {
                            if show_popup.read().as_str() == "No Device" {
                                "Device ID cannot be empty"
                            }
                        }
                    }
                }
            }
        }
    )
}

fn app() -> Element {
    use_init_theme(|| DARK_THEME);

    let device_id = use_signal(String::new);
    let proxy_process = use_signal(|| Option::<Child>::None);

    rsx!(
        Body {
            rect {
                width: "fill",
                height: "fill",
                direction: "horizontal",
                content: "flex",
                padding: "24",
                GardinLogo {}
                DeviceInput {device_id}
                ConnectButton {device_id, proxy_process}
            }
        }
    )
}