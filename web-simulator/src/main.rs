use btmesh_common::opcode::Opcode;
use btmesh_models::{
    generic::{
        battery::{
            GenericBatteryFlags, GenericBatteryFlagsCharging, GenericBatteryFlagsIndicator,
            GenericBatteryFlagsPresence, GenericBatteryMessage, GenericBatteryStatus,
        },
        onoff::{GenericOnOffMessage, GenericOnOffServer},
    },
    sensor::SensorStatus,
    Message, Model,
};
use gloo_timers::callback::Interval;
use gloo_utils::document;
use rand::prelude::random;
use sensor_model::*;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, HtmlInputElement as InputElement};
use yew::prelude::*;

mod http;
mod mqtt;

use http::HttpPublisher;

pub struct Simulator {
    _battery: Interval,
    _sensor: Interval,
}

pub enum SimulatorState {
    Running(Simulator),
    Stopped,
}

pub struct MatrixState {
    on: bool,
    brightness: u8,
}

impl core::fmt::Display for SimulatorState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        match self {
            Self::Running(_) => write!(f, "Running"),
            Self::Stopped => write!(f, "Stopped"),
        }
    }
}

pub trait Publisher {
    fn publish<M: Message>(&self, msg: &M) -> anyhow::Result<()> {
        let mut opcode: heapless::Vec<u8, 16> = heapless::Vec::new();
        msg.opcode()
            .emit(&mut opcode)
            .map_err(|_| std::fmt::Error)?;

        let mut parameters: heapless::Vec<u8, 386> = heapless::Vec::new();
        msg.emit_parameters(&mut parameters)
            .map_err(|_| std::fmt::Error)?;
        let message = RawMessage {
            location: 0,
            opcode: opcode.to_vec(),
            parameters: parameters.to_vec(),
        };
        let data = serde_json::to_string(&message).map_err(|_| std::fmt::Error)?;

        self.send(data)
    }

    fn send(&self, payload: String) -> anyhow::Result<()>;
}

#[derive(Clone, Default)]
struct Refs {
    pub url: NodeRef,
    pub application: NodeRef,
    pub device: NodeRef,
    pub password: NodeRef,
    pub interval: NodeRef,
}

#[function_component(App)]
fn app() -> Html {
    let matrix = use_state(|| MatrixState {
        on: false,
        brightness: 128,
    });
    let state = use_state(|| SimulatorState::Stopped);

    let url =
        use_state(|| "https://web-simulator-eclipsecon-2022.apps.sandbox.drogue.world".to_string());
    let application = use_state(|| "eclipsecon-hackathon".to_string());
    let device = use_state(|| "simulator1".to_string());
    let password = use_state(|| "hey-rodney".to_string());
    let refs = Refs::default();

    let onclick = {
        let state = state.clone();
        let matrix = matrix.clone();

        let url = url.clone();
        let application = application.clone();
        let device = device.clone();
        let password = password.clone();
        let refs = refs.clone();

        Callback::from(move |_| {
            let url = Some((*url).clone()).filter(|s| !s.is_empty());
            let application = Some((*application).clone()).filter(|s| !s.is_empty());
            let device = Some((*device).clone()).filter(|s| !s.is_empty());
            let password = Some((*password).clone()).filter(|s| !s.is_empty());

            let interval = document()
                .get_element_by_id("interval")
                .map(|e| e.dyn_ref::<InputElement>().map(|input| input.value()))
                .flatten()
                .filter(|s| !s.is_empty());

            let inputs = [
                &refs.url,
                &refs.application,
                &refs.device,
                &refs.password,
                &refs.interval,
            ];

            match &*state {
                SimulatorState::Running(_) => {
                    set_disabled(false, inputs);
                    state.set(SimulatorState::Stopped);
                }
                SimulatorState::Stopped => match (url, application, device, password, interval) {
                    (
                        Some(url),
                        Some(application),
                        Some(device),
                        Some(password),
                        Some(interval),
                    ) => {
                        let url = reqwest::Url::parse(&format!("{}/v1/sensor", url)).unwrap();
                        let username = format!("{}@{}", device, application);

                        let m = matrix.clone();
                        let on_command = Callback::from(move |command: RawMessage| {
                            let (opcode, _) = Opcode::split(&command.opcode[..]).unwrap();
                            if let Ok(Some(GenericOnOffMessage::Set(msg))) =
                                GenericOnOffServer::parse(&opcode, &command.parameters)
                            {
                                m.set(MatrixState {
                                    on: msg.on_off == 1,
                                    brightness: m.brightness,
                                });
                            }
                        });

                        let publisher =
                            Arc::new(HttpPublisher::new(url, username, password, on_command));

                        // Battery
                        let interval = interval.parse::<u32>().unwrap();
                        let start_rand: u32 = random::<u32>() % 2000;
                        let send_interval = start_rand + (interval * 1000);
                        log::info!("Publishing battery data at interval {} ms", send_interval);
                        let p = publisher.clone();
                        let _battery = Interval::new(send_interval, move || {
                            let battery = GenericBatteryMessage::Status(GenericBatteryStatus::new(
                                0,
                                0,
                                0,
                                GenericBatteryFlags {
                                    presence: GenericBatteryFlagsPresence::NotPresent,
                                    indicator: GenericBatteryFlagsIndicator::Unknown,
                                    charging: GenericBatteryFlagsCharging::NotChargeable,
                                },
                            ));

                            let _ = p.publish(&battery);
                        });

                        // Sensor
                        let start_rand: u32 = random::<u32>() % 2000;
                        let send_interval = start_rand + (interval * 1000);
                        log::info!("Publishing sensor data at interval {} ms", send_interval);
                        let p = publisher.clone();
                        let _sensor = Interval::new(send_interval, move || {
                            let sensor: SensorMessage =
                                SensorMessage::Status(SensorStatus::new(SensorPayload {
                                    temperature: 22,
                                    acceleration: Default::default(),
                                    noise: 0,
                                }));

                            let _ = p.publish(&sensor);
                        });

                        let sim = Simulator { _battery, _sensor };
                        set_disabled(true, inputs);
                        state.set(SimulatorState::Running(sim));
                    }
                    _ => {
                        gloo_dialogs::alert("One or more fields are missing a value");
                    }
                },
            }
        })
    };

    let dotcolor = if matrix.on { "doton" } else { "dotoff" };
    let opacity = if matrix.on {
        matrix.brightness as f32 / 255.0
    } else {
        1.0
    };
    let style = format!("opacity:{}", opacity);

    html! {
        <>
        <h1>{ "Device Simulator" }</h1>
        <p><b>{"STATE: "}</b>{&*state}</p>
        <p>{ "This application simulates the payloads generated by the EclipseCon 2022 Hackathon mesh network. "}</p>
        <p><b>{"URL: "}</b></p>
        <input id="url" type="text" class="config" value={(*url).clone()} size="25" ref={refs.url.clone()}
            oninput={{
                let url = url.clone();
                Callback::from(move |_| {
                    if let Some(input) = refs.url.cast::<InputElement>() {
                        url.set(input.value());
                    }
                })
        }} />
        <p><b>{"Application: "}</b></p>
        <input id="application" type="text" class="config" value={(*application).clone()} size="25" ref={refs.application.clone()}
            oninput={{
                let application = application.clone();
                Callback::from(move |_| {
                    if let Some(input) = refs.application.cast::<InputElement>() {
                        application.set(input.value());
                    }
                })
        }} />
        <p><b>{"Device: "}</b></p>
        <input id="device" type="text" class="config" value={(*device).clone()} size="25" ref={refs.device.clone()}
            oninput={{
                let device = device.clone();
                Callback::from(move |_| {
                    if let Some(input) = refs.device.cast::<InputElement>() {
                        device.set(input.value());
                    }
                })
        }} />
        <p><b>{"Password: "}</b></p>
        <input id="password" type="password" class="config" value={(*password).clone()} size="25" pattern="[0-9]+" ref={refs.password.clone()}
            oninput={{
                let password = password.clone();
                Callback::from(move |_| {
                    if let Some(input) = refs.password.cast::<InputElement>() {
                        password.set(input.value());
                    }
                })
        }} />
        <p><b>{"Interval (seconds): "}</b></p>
        <input id="interval" type="text" class="config" value="5" size="25" ref={refs.interval.clone()} />
        <br />
        <br />
        <button id="submit" {onclick}>{match &*state {
            SimulatorState::Running(_) => "Stop",
            SimulatorState::Stopped => "Run",
        }}</button>
        <br />
        <h2>{"Display"}</h2>
            <span id="0x0" class={dotcolor} style={style.clone()}/>
            <span id="0x1" class={dotcolor} style={style.clone()}/>
            <span id="0x2" class={dotcolor} style={style.clone()}/>
            <span id="0x3" class={dotcolor} style={style.clone()}/>
            <span id="0x4" class={dotcolor} style={style.clone()}/>
        <br />
            <span id="1x0" class={dotcolor} style={style.clone()} />
            <span id="1x1" class={dotcolor} style={style.clone()} />
            <span id="1x2" class={dotcolor} style={style.clone()} />
            <span id="1x3" class={dotcolor} style={style.clone()} />
            <span id="1x4" class={dotcolor} style={style.clone()} />
        <br />
            <span id="2x0" class={dotcolor} style={style.clone()} />
            <span id="2x1" class={dotcolor} style={style.clone()} />
            <span id="2x2" class={dotcolor} style={style.clone()} />
            <span id="2x3" class={dotcolor} style={style.clone()} />
            <span id="2x4" class={dotcolor} style={style.clone()} />
        <br />
            <span id="3x0" class={dotcolor} style={style.clone()} />
            <span id="3x1" class={dotcolor} style={style.clone()} />
            <span id="3x2" class={dotcolor} style={style.clone()} />
            <span id="3x3" class={dotcolor} style={style.clone()} />
            <span id="3x4" class={dotcolor} style={style.clone()} />
        <br />
            <span id="4x0" class={dotcolor} style={style.clone()} />
            <span id="4x1" class={dotcolor} style={style.clone()} />
            <span id="4x2" class={dotcolor} style={style.clone()} />
            <span id="4x3" class={dotcolor} style={style.clone()} />
            <span id="4x4" class={dotcolor} style={style.clone()} />
        <br />
        </>
    }
}

fn set_disabled<'e, I>(state: bool, elements: I)
where
    I: IntoIterator<Item = &'e NodeRef>,
{
    for e in elements {
        if let Some(ele) = e.cast::<InputElement>() {
            ele.set_disabled(state);
        }
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::start_app::<App>();
}
