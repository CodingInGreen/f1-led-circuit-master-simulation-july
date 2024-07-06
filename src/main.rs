mod driver_info;
mod led_coords;

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use driver_info::{get_driver_info, DriverInfo};
use iced::{
    executor, Application, Command, Element, Length, Settings, Subscription, Theme,
};
use iced::widget::{button, column, container, row, slider, text};
use iced::time;
use led_coords::{read_coordinates, LedCoordinate};
use reqwest::Client;
use serde::de::{self, Deserializer};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::{HashMap, VecDeque};
use std::error::Error as StdError;
use std::result::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LocationData {
    x: f64,
    y: f64,
    #[serde(deserialize_with = "deserialize_datetime")]
    date: DateTime<Utc>,
    driver_number: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DriverData {
    pub driver_number: u32,
    pub led_num: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct UpdateFrame {
    pub drivers: [Option<DriverData>; 20],
}

#[derive(Debug, Clone)]
pub struct VisualizationData {
    pub update_rate_ms: u64,
    pub frames: Vec<UpdateFrame>, // Dynamic-size array
}

// Implement custom Serialize and Deserialize for VisualizationData
impl Serialize for VisualizationData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("VisualizationData", 2)?;
        state.serialize_field("update_rate_ms", &self.update_rate_ms)?;
        state.serialize_field("frames", &self.frames[..])?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for VisualizationData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct VisualizationDataHelper {
            update_rate_ms: u64,
            frames: Vec<UpdateFrame>,
        }

        let helper = VisualizationDataHelper::deserialize(deserializer)?;
        Ok(VisualizationData {
            update_rate_ms: helper.update_rate_ms,
            frames: helper.frames,
        })
    }
}

fn deserialize_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    DateTime::parse_from_rfc3339(&s)
        .map_err(de::Error::custom)
        .map(|dt| dt.with_timezone(&Utc))
}

#[derive(Clone, Debug)]
enum Message {
    StartRace,
    StopRace,
    UpdateRace,
    ApiDataFetched(Result<Vec<UpdateFrame>, FetchError>),
    SpeedChanged(i32),
}

#[derive(Clone, Debug)]
struct FetchError(Arc<dyn StdError + Send + Sync>);

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone)]
struct PlotApp {
    update_rate_ms: u64,
    frames: VecDeque<UpdateFrame>,
    led_coordinates: Vec<LedCoordinate>,
    start_time: Instant,
    race_time: f64,
    race_started: bool,
    driver_info: Vec<DriverInfo>,
    current_index: usize,
    led_states: HashMap<usize, iced::Color>,
    speed: i32,
    data_fetched: bool,
}

impl PlotApp {
    fn new(update_rate_ms: u64, frames: Vec<UpdateFrame>, led_coordinates: Vec<LedCoordinate>, driver_info: Vec<DriverInfo>) -> Self {
        PlotApp {
            update_rate_ms,
            frames: VecDeque::from(frames),
            led_coordinates,
            start_time: Instant::now(),
            race_time: 0.0,
            race_started: false,
            driver_info,
            current_index: 0,
            led_states: HashMap::new(),
            speed: 1,
            data_fetched: false,
        }
    }

    fn reset(&mut self) {
        self.start_time = Instant::now();
        self.race_time = 0.0;
        self.race_started = false;
        self.current_index = 0;
        self.led_states.clear();
    }

    fn update_race(&mut self) {
        if !self.data_fetched {
            return;
        }

        if self.race_started {
            let elapsed = self.start_time.elapsed().as_secs_f64();
            self.race_time = elapsed * self.speed as f64;

            let frame_duration = self.update_rate_ms as f64 / 1000.0;
            let next_index = (self.race_time / frame_duration).floor() as usize;

            if next_index >= self.frames.len() {
                self.current_index = self.frames.len().saturating_sub(1);
            } else {
                self.current_index = next_index;
            }

            self.update_led_states();
        }
    }

    fn update_led_states(&mut self) {
        self.led_states.clear();

        if self.current_index < self.frames.len() {
            let frame = &self.frames[self.current_index];

            for driver_data in &frame.drivers {
                if let Some(driver) = driver_data {
                    let color = self.driver_info.iter().find(|&d| d.number == driver.driver_number).map_or(iced::Color::WHITE, |d| {
                        iced::Color::from_rgba8(d.color.r(), d.color.g(), d.color.b(), d.color.a() as f32 / 255.0)
                    });
                    self.led_states.insert(driver.led_num, color);
                }
            }
        }
    }

    async fn fetch_api_data(&self) -> Result<Vec<UpdateFrame>, FetchError> {
        let session_key = "9149";
        let driver_numbers = vec![1, 2, 4, 10, 11, 14, 16, 18, 20, 22, 23, 24, 27, 31, 40, 44, 55, 63, 77, 81];

        let initial_start_time_str = "2023-08-27T12:58:56.200Z";
        let end_time_str = "2023-08-27T12:58:57.674Z";

        let initial_start_time = DateTime::parse_from_rfc3339(initial_start_time_str)
            .map_err(|e| FetchError(Arc::new(e)))?
            .with_timezone(&Utc);
        let end_time = DateTime::parse_from_rfc3339(end_time_str)
            .map_err(|e| FetchError(Arc::new(e)))?
            .with_timezone(&Utc);

        let time_window = ChronoDuration::milliseconds(1001);

        let client = Client::new();
        let mut all_data: Vec<LocationData> = Vec::new();

        for driver_number in driver_numbers {
            let mut current_start_time = initial_start_time;
            while current_start_time < end_time {
                let current_end_time = current_start_time + time_window;
                let url = format!(
                    "https://api.openf1.org/v1/location?session_key={}&driver_number={}&date>{}&date<{}",
                    session_key, driver_number, current_start_time.to_rfc3339(), current_end_time.to_rfc3339(),
                );

                let mut retry_count = 0;
                let mut success = false;

                while retry_count < 6 && !success {
                    let resp = client.get(&url).send().await.map_err(|e| FetchError(Arc::new(e)))?;
                    if resp.status().is_success() {
                        let data: Vec<LocationData> = resp.json().await.map_err(|e| FetchError(Arc::new(e)))?;
                        if !data.is_empty() {
                            all_data.extend(data.into_iter().filter(|d| d.x != 0.0 && d.y != 0.0));
                        } else {
                            break;
                        }
                        success = true;
                    } else if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        retry_count += 1;
                        let backoff_time = match retry_count {
                            1 => Duration::from_secs(2),
                            2 => Duration::from_secs(4),
                            3 => Duration::from_secs(8),
                            4 => Duration::from_secs(16),
                            5 => Duration::from_secs(32),
                            _ => Duration::from_secs(64),
                        };
                        sleep(backoff_time).await;
                    } else {
                        break;
                    }
                }

                current_start_time = current_end_time;
            }
        }

        all_data.sort_by_key(|d| d.date);

        Ok(generate_update_frames(&all_data, &self.led_coordinates))
    }
}

impl Application for PlotApp {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let coordinates = read_coordinates().expect("Failed to read coordinates");
        let driver_info = get_driver_info();

        (PlotApp::new(10000, vec![], coordinates, driver_info), Command::none())
    }

    fn title(&self) -> String {
        String::from("F1-LED-CIRCUIT SIMULATION")
    }

    type Theme = iced::Theme;

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::StartRace => {
                self.race_started = true;
                self.start_time = Instant::now();
                self.current_index = 0;
                self.led_states.clear();

                Command::none()
            }
            Message::StopRace => {
                self.reset();
                Command::none()
            }
            Message::UpdateRace => {
                self.update_race();
                Command::none()
            }
            Message::ApiDataFetched(_) => {
                Command::none()
            }
            Message::SpeedChanged(speed) => {
                self.speed = speed;
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let content = column![
            row![
                button("Start").on_press(Message::StartRace),
                button("Stop").on_press(Message::StopRace),
                text(format!(
                    "Race Time: {:02}:{:02}:{:05.2}",
                    (self.race_time / 3600.0).floor() as u32,
                    ((self.race_time % 3600.0) / 60.0).floor() as u32,
                    self.race_time % 60.0
                )),
                text("Playback Speed:"),
                slider(1..=5, self.speed, Message::SpeedChanged)
            ],
            text("Driver Info:"),
            text("LED Display:"),
            text(if self.race_started { "Race started!" } else { "Race stopped." }),
            text(format!("Current speed: {}", self.speed))
        ]
        .into();

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        if self.race_started {
            time::every(Duration::from_millis(self.update_rate_ms)).map(|_| Message::UpdateRace)
        } else {
            Subscription::none()
        }
    }
}

fn generate_update_frames(raw_data: &[LocationData], coordinates: &[LedCoordinate]) -> Vec<UpdateFrame> {
    let mut frames: Vec<UpdateFrame> = vec![];
    let mut timestamp_map: HashMap<DateTime<Utc>, Vec<LocationData>> = HashMap::new();

    for data in raw_data {
        timestamp_map.entry(data.date).or_insert_with(Vec::new).push(data.clone());
    }

    for (_timestamp, data_group) in timestamp_map {
        let mut frame = UpdateFrame { drivers: [None; 20] };

        for data in data_group {
            let (nearest_coord, _distance) = coordinates
                .iter()
                .map(|coord| {
                    let distance = ((data.x - coord.x_led).powi(2) + (data.y - coord.y_led).powi(2)).sqrt();
                    (coord, distance)
                })
                .min_by(|(_, dist_a), (_, dist_b)| {
                    dist_a.partial_cmp(dist_b).unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap();

            let driver_data = DriverData { driver_number: data.driver_number, led_num: nearest_coord.led_number };

            let mut inserted = false;
            for slot in frame.drivers.iter_mut() {
                if slot.is_none() {
                    *slot = Some(driver_data);
                    inserted = true;
                    break;
                }
            }

            if !inserted || frame.drivers.iter().all(|slot| slot.is_some()) {
                frames.push(frame);
                frame = UpdateFrame { drivers: [None; 20] };

                if !inserted {
                    for slot in frame.drivers.iter_mut() {
                        if slot.is_none() {
                            *slot = Some(driver_data);
                            break;
                        }
                    }
                }
            }
        }

        if frame.drivers.iter().any(|slot| slot.is_some()) {
            frames.push(frame);
        }
    }

    frames
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError>> {
    PlotApp::run(Settings::default()).map_err(|e| Box::new(e) as Box<dyn StdError>)
}
