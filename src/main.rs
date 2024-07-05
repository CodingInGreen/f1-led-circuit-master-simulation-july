// main.rs
mod led_coords;
mod driver_info;

use chrono::{DateTime, Duration, Utc};
use eframe::{egui, App, Frame};
use reqwest::Client;
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize, Serializer};
use serde::ser::SerializeStruct;
use std::collections::HashMap;
use std::error::Error as StdError;
use std::result::Result;
use std::time::Instant;
use tokio;
use futures::stream;
use futures::StreamExt;
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

struct PlotApp {
    update_rate_ms: u64,
    frames: Vec<UpdateFrame>,
    led_coordinates: Vec<LedCoordinate>,
    start_time: Instant,
    race_time: f64, // Elapsed race time in seconds
    race_started: bool,
    driver_info: Vec<DriverInfo>,
    current_index: usize,
    led_states: HashMap<usize, egui::Color32>, // Tracks the current state of the LEDs
    speed: i32, // Playback speed multiplier
}

impl PlotApp {
    fn new(
        update_rate_ms: u64,
        frames: Vec<UpdateFrame>,
        led_coordinates: Vec<LedCoordinate>,
        driver_info: Vec<DriverInfo>,
    ) -> PlotApp {
        PlotApp {
            update_rate_ms,
            frames,
            led_coordinates,
            start_time: Instant::now(),
            race_time: 0.0,
            race_started: false,
            driver_info,
            current_index: 0,
            led_states: HashMap::new(), // Initialize empty LED state tracking
            speed: 1,
        }
    }

    fn reset(&mut self) {
        self.start_time = Instant::now();
        self.race_time = 0.0;
        self.race_started = false;
        self.current_index = 0;
        self.led_states.clear(); // Reset LED states
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
        self.led_states.clear();

        if self.current_index > 0 {
            let frame = &self.frames[self.current_index - 1];

            for driver_data in &frame.drivers {
                if let Some(driver) = driver_data {
                    let color = self.driver_info.iter()
                        .find(|&d| d.number == driver.driver_number)
                        .map_or(egui::Color32::WHITE, |d| d.color);
                    self.led_states.insert(driver.led_num, color);
                }
            }
        }
    }
}

impl App for PlotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.update_race();

        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::new("layer"),
        ));

        let (min_x, max_x) = self.led_coordinates.iter().fold(
            (f64::INFINITY, f64::NEG_INFINITY),
            |(min, max), coord| {
                (min.min(coord.x_led), max.max(coord.x_led))
            },
        );
        let (min_y, max_y) = self.led_coordinates.iter().fold(
            (f64::INFINITY, f64::NEG_INFINITY),
            |(min, max), coord| {
                (min.min(coord.y_led), max.max(coord.y_led))
            },
        );

        let width = max_x - min_x;
        let height = max_y - min_y;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.separator();
                ui.label(format!(
                    "Race Time: {:02}:{:02}:{:05.2}",
                    (self.race_time / 3600.0).floor() as u32, // hours
                    ((self.race_time % 3600.0) / 60.0).floor() as u32, // minutes
                    self.race_time % 60.0 // seconds with milliseconds
                ));
                ui.separator();

                if ui.button("START").clicked() {
                    self.race_started = true;
                    self.start_time = Instant::now();
                    self.current_index = 0;
                    self.led_states.clear(); // Clear LED states when race starts
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
                    .size = 8.0; // Set the font size to 8.0 (or any other size you prefer)

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
                        ui.add_space(5.0); // Space between legend items
                    });
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            for coord in &self.led_coordinates {
                let norm_x = ((coord.x_led - min_x) / width) as f32 * (ui.available_width() - 60.0); // Adjust for left/right margin
                let norm_y = (ui.available_height() - 60.0)
                    - (((coord.y_led - min_y) / height) as f32 * (ui.available_height() - 60.0)); // Adjust for top/bottom margin

                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(norm_x + 30.0, norm_y + 30.0), // Adjust position to include margins
                        egui::vec2(20.0, 20.0),
                    ),
                    egui::Rounding::same(0.0),
                    egui::Color32::BLACK,
                );

                if let Some(&color) = self.led_states.get(&coord.led_number) {
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(norm_x + 30.0, norm_y + 30.0), // Adjust position to include margins
                            egui::vec2(20.0, 20.0),
                        ),
                        egui::Rounding::same(0.0),
                        color,
                    );
                }
            }
        });

        ctx.request_repaint(); // Request the GUI to repaint
    }
}

fn main() -> Result<(), Box<dyn StdError>> {
    let coordinates = read_coordinates()?; // Unwrap the result here

    // Initialize the runtime for async execution
    let runtime = tokio::runtime::Runtime::new()?;
    let raw_data = runtime.block_on(fetch_data())?;

    let frames = generate_update_frames(&raw_data, &coordinates);
    let driver_info = get_driver_info();

    let update_rate_ms = 100; // Assuming update rate is 100 ms as in the previous code
    let app = PlotApp::new(update_rate_ms, frames, coordinates, driver_info);

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "F1-LED-CIRCUIT SIMULATION",
        native_options,
        Box::new(|_cc| Box::new(app)),
    )?;

    Ok(())
}

async fn fetch_data() -> Result<Vec<LocationData>, Box<dyn StdError>> {
    let session_key = "9149";
    let driver_numbers = vec![
        1, 2, 4, 10, 11, 14, 16, 18, 20, 22, 23, 24, 27, 31, 40, 44, 55, 63, 77, 81,
    ];
    let start_time_str = "2023-08-27T12:58:56.200Z";
    let end_time_str = "2023-08-27T13:20:54.300Z";

    let client = Client::new();
    let mut all_data: Vec<LocationData> = Vec::new();
    
    // Define the chunk size in seconds and the step interval for each chunk
    let chunk_size_secs = 180; // Fetch 3 minute of data per chunk
    let step_interval_secs = 180; // Move by 3 minute each iteration

    let start_time = DateTime::parse_from_rfc3339(start_time_str)?.with_timezone(&Utc);
    let end_time = DateTime::parse_from_rfc3339(end_time_str)?.with_timezone(&Utc);

    let mut current_start_time = start_time;

    while current_start_time < end_time {
        let current_end_time = (current_start_time + Duration::seconds(chunk_size_secs)).min(end_time);

        println!("Fetching data from {} to {}", current_start_time, current_end_time); // Debug

        let tasks: Vec<_> = driver_numbers.iter().map(|&driver_number| {
            let client = &client;
            let url = format!(
                "https://api.openf1.org/v1/location?session_key={}&driver_number={}&date>{}&date<{}",
                session_key, driver_number, current_start_time.to_rfc3339(), current_end_time.to_rfc3339(),
            );
            async move {
                let resp = client.get(&url).send().await;
                match resp {
                    Ok(resp) if resp.status().is_success() => {
                        let data: Vec<LocationData> = resp.json().await.unwrap_or_else(|_| vec![]);
                        Some(data.into_iter().filter(|d| d.x != 0.0 && d.y != 0.0).collect::<Vec<_>>())
                    }
                    _ => None,
                }
            }
        }).collect();

        let results = stream::iter(tasks).buffer_unordered(10).collect::<Vec<_>>().await;

        for result in results {
            if let Some(data) = result {
                println!("Fetched {} entries for a driver", data.len()); // Debug
                all_data.extend(data);
            }
        }

        println!("Total data size: {}", all_data.len()); // Debug

        current_start_time = current_start_time + Duration::seconds(step_interval_secs);
    }

    // Sort the data by the date field
    all_data.sort_by_key(|d| d.date);
    Ok(all_data)
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

        // Insert the driver data into the frame
        for slot in frame.drivers.iter_mut() {
            if slot.is_none() {
                *slot = Some(driver_data);
                break;
            }
        }

        // Once the frame is full, push it to the frames vector and start a new frame
        if frame.drivers.iter().all(|slot| slot.is_some()) {
            frames.push(frame);
            frame = UpdateFrame {
                drivers: [None; 20],
            };
        }
    }

    // Push the last frame if it has any data
    if frame.drivers.iter().any(|slot| slot.is_some()) {
        frames.push(frame);
    }

    frames
}
