mod driver_info;
mod led_coords;

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use driver_info::{get_driver_info, DriverInfo};
use eframe::{egui, App, Frame};
use led_coords::{read_coordinates, LedCoordinate};
use reqwest::Client;
use serde::de::{self, Deserializer};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::{HashMap, VecDeque};
use std::error::Error as StdError;
use std::result::Result;
use std::time::{Duration, Instant};
use tokio::time::{interval, sleep};

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

#[derive(Clone)]
struct PlotApp {
    update_rate_ms: u64,
    frames: VecDeque<UpdateFrame>,
    led_coordinates: Vec<LedCoordinate>,
    start_time: Instant,
    race_time: f64, // Elapsed race time in seconds
    race_started: bool,
    driver_info: Vec<DriverInfo>,
    current_index: usize,
    led_states: HashMap<usize, egui::Color32>, // Tracks the current state of the LEDs
    speed: i32,                                // Playback speed multiplier
    data_fetched: bool,                        // Indicates whether data fetching is complete
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
            frames: VecDeque::from(frames),
            led_coordinates,
            start_time: Instant::now(),
            race_time: 0.0,
            race_started: false,
            driver_info,
            current_index: 0,
            led_states: HashMap::new(), // Initialize empty LED state tracking
            speed: 1,
            data_fetched: false, // Initialize to false
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
        if !self.data_fetched {
            return;
        }

        println!("Updating race...");

        if self.race_started {
            let elapsed = self.start_time.elapsed().as_secs_f64();
            self.race_time = elapsed * self.speed as f64;

            let frame_duration = self.update_rate_ms as f64 / 1000.0;
            let next_index = (self.race_time / frame_duration).floor() as usize;

            if next_index >= self.frames.len() {
                self.current_index = self.frames.len().saturating_sub(1); // Ensure it does not exceed frames length
            } else {
                self.current_index = next_index;
            }

            println!(
                "Current index: {}, Next index: {}",
                self.current_index, next_index
            );

            // If current_index is 0, log a warning and do not call update_led_states
            if self.current_index == 0 {
                println!("Warning: current index ({}) is 0", self.current_index);
                panic!("Panicking - we're about to be out of bounds.");
            } else {
                self.update_led_states();
            }
        }
    }

    fn update_led_states(&mut self) {
        self.led_states.clear();

        if self.current_index < self.frames.len() {
            let frame = &self.frames[self.current_index];
            println!("Processing frame: {:?}", frame);

            for driver_data in &frame.drivers {
                if let Some(driver) = driver_data {
                    let color = self
                        .driver_info
                        .iter()
                        .find(|&d| d.number == driver.driver_number)
                        .map_or(egui::Color32::WHITE, |d| d.color);
                    self.led_states.insert(driver.led_num, color);
                }
            }
        } else {
            println!("Skipping update as current_index is out of bounds");
        }

        // Debug statement to print the LED states
        println!("LED States: {:?}", self.led_states);
    }

    async fn fetch_api_data(&mut self) -> Result<(), Box<dyn StdError>> {
        let session_key = "9149";
        let driver_numbers = vec![
            1, 2, 4, 10, 11, 14, 16, 18, 20, 22, 23, 24, 27, 31, 40, 44, 55, 63, 77, 81,
        ];

        // Validate the initial start time and end time strings
        let initial_start_time_str = "2023-08-27T12:58:56.200Z";
        let end_time_str = "2023-08-27T12:58:57.674Z"; // rate limit test

        // Log the input strings for verification
        println!("Parsing initial_start_time_str: {}", initial_start_time_str);
        println!("Parsing end_time_str: {}", end_time_str);

        let initial_start_time = DateTime::parse_from_rfc3339(initial_start_time_str)
            .map_err(|e| format!("Failed to parse initial_start_time: {}", e))?
            .with_timezone(&Utc);

        let end_time = DateTime::parse_from_rfc3339(end_time_str)
            .map_err(|e| format!("Failed to parse end_time: {}", e))?
            .with_timezone(&Utc);

        // Each API call should cover a time window of 0.35 seconds
        let time_window = ChronoDuration::milliseconds(1001);

        let client = Client::new();
        let mut all_data: Vec<LocationData> = Vec::new();

        for driver_number in driver_numbers {
            let mut current_start_time = initial_start_time;
            while current_start_time < end_time {
                let current_end_time = current_start_time + time_window;
                println!(
                    "Fetching data for driver {} from {} to {}",
                    driver_number, current_start_time, current_end_time
                );
                let url = format!(
                    "https://api.openf1.org/v1/location?session_key={}&driver_number={}&date>{}&date<{}",
                    session_key, driver_number, current_start_time.to_rfc3339(), current_end_time.to_rfc3339(),
                );

                let mut retry_count = 0;
                let mut success = false;

                while retry_count < 6 && !success {
                    let resp = client.get(&url).send().await?;
                    if resp.status().is_success() {
                        let data: Vec<LocationData> = resp.json().await?;
                        println!(
                            "Fetched {} entries for driver {} from {} to {}",
                            data.len(),
                            driver_number,
                            current_start_time,
                            current_end_time
                        );
                        if !data.is_empty() {
                            all_data.extend(data.into_iter().filter(|d| d.x != 0.0 && d.y != 0.0));
                        } else {
                            break; // Stop if no data is returned
                        }
                        success = true;
                    } else if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        eprintln!(
                            "Failed to fetch data for driver {}: HTTP {} Too Many Requests",
                            driver_number,
                            resp.status()
                        );
                        retry_count += 1;
                        let backoff_time = match retry_count {
                            1 => Duration::from_secs(2),
                            2 => Duration::from_secs(4),
                            3 => Duration::from_secs(8),
                            4 => Duration::from_secs(16),
                            5 => Duration::from_secs(32),
                            _ => Duration::from_secs(64),
                        };
                        eprintln!("Retrying in {:?}...", backoff_time);
                        sleep(backoff_time).await; // Exponential backoff
                    } else {
                        eprintln!(
                            "Failed to fetch data for driver {}: HTTP {}",
                            driver_number,
                            resp.status()
                        );
                        break;
                    }
                }

                if !success {
                    eprintln!(
                        "Failed to fetch data for driver {} after {} retries",
                        driver_number, retry_count
                    );
                }

                current_start_time = current_end_time;
            }
        }

        all_data.sort_by_key(|d| d.date);

        // Print statement indicating all data has been fetched and dump data contents
        println!("All data has been successfully fetched.");
        println!("Data contents: {:#?}", all_data);

        let frames = generate_update_frames(&all_data, &self.led_coordinates);
        self.frames.extend(frames);

        // Set data_fetched to true after fetching is complete
        self.data_fetched = true;

        // Set current_index based on the fetched frames
        if !self.frames.is_empty() {
            self.current_index = 1; // Set to 1 to ensure visualization starts
        } else {
            self.current_index = 0; // Ensure it is 0 if no frames are available
        }

        Ok(())
    }

    async fn run_visualization(&mut self) {
        println!("Running Visualization...");
        let mut interval = interval(Duration::from_millis(self.update_rate_ms));
        while self.race_started {
            interval.tick().await;
            self.update_race();
            if !self.frames.is_empty() {
                self.frames.pop_front();
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
                
                    let mut app_clone = self.clone();
                    tokio::spawn(async move {
                        app_clone.fetch_api_data().await.unwrap();
                
                        // Only spawn run_visualization if data fetching is complete and current_index is not 0
                        if app_clone.data_fetched && app_clone.current_index != 0 {
                            app_clone.run_visualization().await;
                        } else {
                            eprintln!("Data fetching was not completed successfully or current_index is 0.");
                        }
                    });
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

fn generate_update_frames(
    raw_data: &[LocationData],
    coordinates: &[LedCoordinate],
) -> Vec<UpdateFrame> {
    let mut frames: Vec<UpdateFrame> = vec![];
    let mut timestamp_map: HashMap<DateTime<Utc>, Vec<LocationData>> = HashMap::new();

    println!("Generating Update Frames");

    // Group location data by timestamp
    for data in raw_data {
        timestamp_map
            .entry(data.date)
            .or_insert_with(Vec::new)
            .push(data.clone());
    }

    // Iterate over each timestamp and create frames
    for (_timestamp, data_group) in timestamp_map {
        let mut frame = UpdateFrame {
            drivers: [None; 20],
        };

        for data in data_group {
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
            let mut inserted = false;
            for slot in frame.drivers.iter_mut() {
                if slot.is_none() {
                    *slot = Some(driver_data);
                    inserted = true;
                    break;
                }
            }

            // If the frame is full, push it to the frames vector and start a new frame
            if !inserted || frame.drivers.iter().all(|slot| slot.is_some()) {
                frames.push(frame);
                frame = UpdateFrame {
                    drivers: [None; 20],
                };

                // Ensure the new frame includes the driver data if it wasn't inserted
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

        // Push the last frame if it has any data
        if frame.drivers.iter().any(|slot| slot.is_some()) {
            frames.push(frame);
        }
    }
    println!("Frames data: {:?}", frames);
    frames
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError>> {
    let coordinates = read_coordinates()?;
    let driver_info = get_driver_info();

    let app = PlotApp::new(10000, vec![], coordinates, driver_info);

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "F1-LED-CIRCUIT SIMULATION",
        native_options,
        Box::new(|_cc| Box::new(app)),
    )?;

    Ok(())
}