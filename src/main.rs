mod led_coords;
mod driver_info;

use chrono::{DateTime, Utc};
use eframe::{egui, App, Frame};
use futures_util::stream::StreamExt;
use futures_util::future;
use reqwest::Client;
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize, Serializer};
use serde::ser::SerializeStruct;
use std::collections::HashMap;
use std::error::Error as StdError;
use std::result::Result;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio;
use led_coords::{LedCoordinate, read_coordinates};
use driver_info::{DriverInfo, get_driver_info};
#[derive(Debug, Serialize, Deserialize)]
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
    pub frames: Vec<UpdateFrame>,
}

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
struct RaceData {
    date: DateTime<Utc>,
    driver_number: u32,
    x_led: f64,
    y_led: f64,
}

#[derive(Clone)]
struct PlotApp {
    update_rate_ms: u64,
    frames: Vec<UpdateFrame>,
    led_coordinates: Vec<LedCoordinate>,
    start_time: Instant,
    race_time: f64,
    race_started: bool,
    data_loading_started: bool,
    data_loaded: bool,
    driver_info: Vec<DriverInfo>,
    current_index: usize,
    last_visualized_index: usize,
    led_states: Arc<Mutex<HashMap<usize, egui::Color32>>>,
    speed: i32,
    completion_sender: Option<async_channel::Sender<()>>,
    completion_receiver: Option<async_channel::Receiver<()>>,
}

impl PlotApp {
    fn new(
        update_rate_ms: u64,
        frames: Vec<UpdateFrame>,
        led_coordinates: Vec<LedCoordinate>,
        driver_info: Vec<DriverInfo>,
    ) -> PlotApp {
        let (completion_sender, completion_receiver) = async_channel::bounded(1);
        PlotApp {
            update_rate_ms,
            frames,
            led_coordinates,
            start_time: Instant::now(),
            race_time: 0.0,
            race_started: false,
            data_loading_started: false,
            data_loaded: false,
            driver_info,
            current_index: 0,
            last_visualized_index: 0,
            led_states: Arc::new(Mutex::new(HashMap::new())),
            speed: 1,
            completion_sender: Some(completion_sender),
            completion_receiver: Some(completion_receiver),
        }
    }

    fn reset(&mut self) {
        self.start_time = Instant::now();
        self.race_time = 0.0;
        self.race_started = false;
        self.current_index = 0;
        self.last_visualized_index = 0;
        self.led_states.lock().unwrap().clear();
    }

    fn update_race(&mut self) {
        if self.race_started {
            let elapsed = self.start_time.elapsed().as_secs_f64();
            self.race_time = elapsed * self.speed as f64;

            let frame_duration = self.update_rate_ms as f64 / 1000.0;
            let mut next_index = self.current_index;
            while next_index < self.frames.len() && next_index as f64 * frame_duration <= self.race_time {
                next_index += 1;
            }

            self.current_index = next_index;
            self.update_led_states();
        }
    }

    fn update_led_states(&mut self) {
        self.led_states.lock().unwrap().clear();

        if self.current_index > 0 {
            let frame = &self.frames[self.current_index - 1];

            for driver_data in &frame.drivers {
                if let Some(driver) = driver_data {
                    let color = self.driver_info.iter()
                        .find(|&d| d.number == driver.driver_number)
                        .map_or(egui::Color32::WHITE, |d| d.color);
                    self.led_states.lock().unwrap().insert(driver.led_num, color);
                }
            }
        }
    }

    async fn fetch_api_data(&mut self) -> Result<(), Box<dyn StdError + Send + Sync>> {
        println!("Starting to load data...");
        let driver_numbers = vec![
            1, 2, 4, 10, 11, 14, 16, 18, 20, 22, 23, 24, 27, 31, 40, 44, 55, 63, 77, 81,
        ];

        let mut all_drivers_complete = false;

        while !all_drivers_complete {
            let mut handles = Vec::new();
            all_drivers_complete = true;

            for &driver_number in &driver_numbers {
                let url = format!(
                    "https://api.openf1.org/v1/location?session_key={}&driver_number={}",
                    "9149", driver_number
                );

                let mut app_clone = self.clone();
                let sender_clone = self.completion_sender.clone().unwrap();
                handles.push(tokio::spawn(async move {
                    let mut stream = fetch_data_in_chunks(&url, 8 * 1024).await?;
                    let mut buffer = Vec::new();
                    let mut driver_complete = true;

                    while let Some(chunk) = stream.next().await {
                        let chunk = chunk?;
                        let run_race_data = deserialize_chunk(
                            chunk,
                            &mut buffer,
                            &app_clone.led_coordinates,
                            usize::MAX,
                            &sender_clone,
                        ).await?;

                        app_clone.frames.extend(run_race_data);
                        app_clone.frames.sort_by_key(|d| d.drivers[0].as_ref().unwrap().driver_number);

                        app_clone.update_race();

                        if !buffer.is_empty() {
                            driver_complete = false;
                        }
                    }

                    if driver_complete {
                        println!("Completed data fetching for driver number {}", driver_number);
                    }

                    Ok::<(), Box<dyn StdError + Send + Sync>>(())
                }));
            }

            let results = future::join_all(handles).await;

            for result in results {
                if let Err(e) = result {
                    eprintln!("Error fetching data: {:?}", e);
                }
            }

            for driver_number in &driver_numbers {
                let data_complete = Self::check_if_data_complete(driver_number).await;
                if !data_complete {
                    all_drivers_complete = false;
                }
            }
        }

        println!("Finished streaming data for all drivers");
        self.data_loaded = true;

        if let Some(sender) = &self.completion_sender {
            println!("Sending final completion message...");
            let _ = sender.send(()).await;
            println!("Final completion message sent.");
        }

        Ok(())
    }

    async fn check_if_data_complete(_driver_number: &u32) -> bool {
        false
    }
}

impl App for PlotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.update_race();

        if let Some(receiver) = self.completion_receiver.as_ref() {
            while let Ok(()) = receiver.try_recv() {
            }
        }

        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::new("layer"),
        ));

        let (min_x, max_x) = self
            .led_coordinates
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), coord| {
                (min.min(coord.x_led), max.max(coord.x_led))
            });
        let (min_y, max_y) = self
            .led_coordinates
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), coord| {
                (min.min(coord.y_led), max.max(coord.y_led))
            });

        let width = max_x - min_x;
        let height = max_y - min_y;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.separator();
                ui.label(format!(
                    "Race Time: {:02}:{:02}:{:05.2}",
                    (self.race_time / 3600.0).floor() as u32,
                    ((self.race_time % 3600.0) / 60.0).floor() as u32,
                    self.race_time % 60.0
                ));
                ui.separator();

                if ui.button("START").clicked() {
                    if !self.data_loading_started {
                        println!("Start button clicked, beginning data loading...");
                        self.data_loading_started = true;
                        self.race_started = true;
                        self.start_time = Instant::now();
                        let mut app_clone = self.clone();
                        let sender = self.completion_sender.clone().unwrap();
                        tokio::spawn(async move {
                            println!("Spawning data loading task...");
                            app_clone.fetch_api_data().await.unwrap();
                            let _ = sender.send(()).await;
                            println!("Data loading task completed.");
                        });
                    }
                }

                if ui.button("STOP").clicked() {
                    self.reset();
                }

                ui.label("PLAYBACK SPEED");
                ui.add(egui::Slider::new(&mut self.speed, 1..=5));
            });
        });

        egui::SidePanel::right("legend_panel").show(ctx, |ui| {
            ui.vertical(|ui| {
                let style = ui.style_mut();
                style
                    .text_styles
                    .get_mut(&egui::TextStyle::Body)
                    .unwrap()
                    .size = 8.0;

                for driver in &self.driver_info {
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "{}: {} ({})",
                            driver.number, driver.name, driver.team
                        ));
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(ui.cursor().min, egui::vec2(5.0, 5.0)),
                            0.0,
                            driver.color,
                        );
                        ui.add_space(5.0);
                    });
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            for coord in &self.led_coordinates {
                let norm_x = ((coord.x_led - min_x) / width) as f32 * (ui.available_width() - 60.0);
                let norm_y = (ui.available_height() - 60.0)
                    - (((coord.y_led - min_y) / height) as f32 * (ui.available_height() - 60.0));

                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(norm_x + 30.0, norm_y + 30.0),
                        egui::vec2(20.0, 20.0),
                    ),
                    egui::Rounding::same(0.0),
                    egui::Color32::BLACK,
                );
            }

            let led_states = self.led_states.lock().unwrap();

            for (&led_num, &color) in &*led_states {
                let coord = &self.led_coordinates[led_num];
                let norm_x = ((coord.x_led - min_x) / width) as f32 * (ui.available_width() - 60.0);
                let norm_y = (ui.available_height() - 60.0)
                    - (((coord.y_led - min_y) / height) as f32 * (ui.available_height() - 60.0));

                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(norm_x + 30.0, norm_y + 30.0),
                        egui::vec2(20.0, 20.0),
                    ),
                    egui::Rounding::same(0.0),
                    color,
                );
            }
        });

        ctx.request_repaint();
    }
}

fn generate_update_frames(
    raw_data: &[LocationData],
    coordinates: &[LedCoordinate],
) -> Vec<UpdateFrame> {
    let mut frames: Vec<UpdateFrame> = vec![];
    let mut frame = UpdateFrame {
        drivers: [None; 20],
    };

    for data in raw_data {
        let (nearest_coord, _distance) = coordinates
            .iter()
            .map(|coord| {
                let distance =
                    ((data.x - coord.x_led).powi(2) + (data.y - coord.y_led).powi(2)).sqrt();
                (coord, distance)
            })
            .min_by(|(_, dist_a), (_, dist_b)| {
                dist_a
                    .partial_cmp(dist_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();

        let driver_data = DriverData {
            driver_number: data.driver_number,
            led_num: nearest_coord.led_number,
        };

        for slot in frame.drivers.iter_mut() {
            if slot.is_none() {
                *slot = Some(driver_data);
                break;
            }
        }

        if frame.drivers.iter().all(|slot| slot.is_some()) {
            frames.push(frame);
            frame = UpdateFrame {
                drivers: [None; 20],
            };
        }
    }

    if frame.drivers.iter().any(|slot| slot.is_some()) {
        frames.push(frame);
    }

    frames
}

async fn fetch_data_in_chunks(
    url: &str,
    _chunk_size: usize,
) -> Result<
    impl futures_util::stream::Stream<Item = Result<bytes::Bytes, reqwest::Error>>,
    Box<dyn StdError + Send + Sync>,
> {
    let client = Client::new();
    let resp = client.get(url).send().await?.error_for_status()?;
    let stream = resp.bytes_stream();  // Correctly use bytes_stream from reqwest
    Ok(stream)
}

async fn deserialize_chunk(
    chunk: bytes::Bytes,
    buffer: &mut Vec<u8>,
    coordinates: &[LedCoordinate],
    max_rows: usize,
    sender: &async_channel::Sender<()>,
) -> Result<Vec<UpdateFrame>, Box<dyn StdError + Send + Sync>> {
    buffer.extend_from_slice(&chunk);

    let mut run_race_data = Vec::new();
    let mut rows_processed = 0;

    let buffer_str = String::from_utf8_lossy(&buffer);
    let mut start_pos = 0;

    while let Some(end_pos) = buffer_str[start_pos..].find("},{") {
        let json_slice = &buffer_str[start_pos..start_pos + end_pos + 1];
        let json_slice = json_slice.trim_start_matches('[').trim_end_matches(']');

        let json_slice = if !json_slice.starts_with('{') {
            if start_pos > 0 {
                buffer_str[start_pos - 1..start_pos + end_pos + 1].to_string()
            } else {
                let mut json_str = String::from("{");
                json_str.push_str(json_slice);
                json_str
            }
        } else {
            json_slice.to_string()
        };

        match serde_json::from_str::<LocationData>(&json_slice) {
            Ok(location_data) => {
                let new_run_race_data = generate_update_frames(&[location_data], coordinates);

                rows_processed += new_run_race_data.len();
                run_race_data.extend(new_run_race_data);
                start_pos += end_pos + 3;
                if rows_processed >= max_rows {
                    println!("Reached max rows limit: {}", max_rows);
                    break;
                }
            }
            Err(e) => {
                println!("Failed to deserialize LocationData: {:?}", e);
                break;
            }
        }
    }

    *buffer = buffer_str[start_pos..].as_bytes().to_vec();

    if let Ok(location_data) = serde_json::from_slice::<LocationData>(&buffer) {
        let new_run_race_data = generate_update_frames(&[location_data], coordinates);
        run_race_data.extend(new_run_race_data);
        *buffer = Vec::new();
    }

    let _ = sender.send(()).await;

    println!("Processed JSON objects in this chunk");

    Ok(run_race_data)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError>> {
    let coordinates = read_coordinates()?;
    let driver_info = get_driver_info();

    let app = PlotApp::new(100, vec![], coordinates, driver_info);

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "F1-LED-CIRCUIT SIMULATION",
        native_options,
        Box::new(|_cc| Box::new(app)),
    )?;

    Ok(())
}
