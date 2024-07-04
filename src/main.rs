use chrono::{DateTime, Utc};
use eframe::{egui, App, Frame};
use reqwest::Client;
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error as StdError;
use std::result::Result;
use std::time::Instant;
use tokio;

#[derive(Debug, Serialize, Deserialize)]
struct LocationData {
    x: f64,
    y: f64,
    #[serde(deserialize_with = "deserialize_datetime")]
    date: DateTime<Utc>,
    driver_number: u32,
}

#[derive(Debug, Deserialize)]
struct LedCoordinate {
    x_led: f64,
    y_led: f64,
}

#[derive(Debug)]
struct RunRace {
    date: DateTime<Utc>,
    driver_number: u32,
    x_led: f64,
    y_led: f64,
}

#[derive(Debug)]
struct DriverInfo {
    number: u32,
    name: &'static str,
    team: &'static str,
    color: egui::Color32,
}

struct PlotApp {
    coordinates: Vec<LedCoordinate>,
    run_race_data: Vec<RunRace>,
    start_time: Instant,
    race_time: f64, // Elapsed race time in seconds
    race_started: bool,
    driver_info: Vec<DriverInfo>,
    current_index: usize,
    led_states: HashMap<(i64, i64), egui::Color32>, // Tracks the current state of the LEDs
    last_positions: HashMap<u32, (i64, i64)>,       // Last known positions of each driver
    speed: i32,                                     // Playback speed multiplier
}

impl PlotApp {
    fn new(
        coordinates: Vec<LedCoordinate>,
        run_race_data: Vec<RunRace>,
        driver_info: Vec<DriverInfo>,
    ) -> PlotApp {
        PlotApp {
            coordinates,
            run_race_data,
            start_time: Instant::now(),
            race_time: 0.0,
            race_started: false,
            driver_info,
            current_index: 0,
            led_states: HashMap::new(), // Initialize empty LED state tracking
            last_positions: HashMap::new(), // Initialize empty last positions hashmap
            speed: 1,
        }
    }

    fn reset(&mut self) {
        self.start_time = Instant::now();
        self.race_time = 0.0;
        self.race_started = false;
        self.current_index = 0;
        self.led_states.clear(); // Reset LED states
        self.last_positions.clear(); // Reset last positions
    }

    fn update_race(&mut self) {
        if self.race_started {
            let elapsed = self.start_time.elapsed().as_secs_f64();
            self.race_time = elapsed * self.speed as f64;

            let mut next_index = self.current_index;
            while next_index < self.run_race_data.len() {
                let run_data = &self.run_race_data[next_index];
                let race_data_time =
                    (run_data.date - self.run_race_data[0].date).num_milliseconds() as f64 / 1000.0;
                if race_data_time <= self.race_time {
                    next_index += 1;
                } else {
                    break;
                }
            }

            self.current_index = next_index;
            self.update_led_states();
        }
    }

    fn update_led_states(&mut self) {
        self.led_states.clear();

        for run_data in &self.run_race_data[..self.current_index] {
            let coord_key = (
                Self::scale_f64(run_data.x_led, 1_000_000),
                Self::scale_f64(run_data.y_led, 1_000_000),
            );

            println!("Driver {} moved to LED position {:?}", run_data.driver_number, coord_key);

            // Update the last known position of the driver
            self.last_positions
                .insert(run_data.driver_number, coord_key);
        }

        // Update the LED states for all known positions
        for (&driver_number, &position) in &self.last_positions {
            let color = self
                .driver_info
                .iter()
                .find(|&driver| driver.number == driver_number)
                .map_or(egui::Color32::WHITE, |driver| driver.color);
            println!(
                "LED at position {:?} set to color {:?} for driver {}",
                position, color, driver_number
            );
            self.led_states.insert(position, color);
        }
    }

    fn scale_f64(value: f64, scale: i64) -> i64 {
        (value * scale as f64) as i64
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
            .coordinates
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), coord| {
                (min.min(coord.x_led), max.max(coord.x_led))
            });
        let (min_y, max_y) = self
            .coordinates
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
                    self.race_time % 60.0                     // seconds with milliseconds
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
            for coord in &self.coordinates {
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
            }

            for ((x, y), color) in &self.led_states {
                let norm_x = ((*x as f64 / 1_000_000.0 - min_x) / width) as f32
                    * (ui.available_width() - 60.0); // Adjust for left/right margin
                let norm_y = (ui.available_height() - 60.0)
                    - (((*y as f64 / 1_000_000.0 - min_y) / height) as f32
                        * (ui.available_height() - 60.0)); // Adjust for top/bottom margin

                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(norm_x + 30.0, norm_y + 30.0), // Adjust position to include margins
                        egui::vec2(20.0, 20.0),
                    ),
                    egui::Rounding::same(0.0),
                    *color,
                );
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

    let run_race_data = generate_run_race_data(&raw_data, &coordinates);

    let driver_info = vec![
        DriverInfo {
            number: 1,
            name: "Max Verstappen",
            team: "Red Bull",
            color: egui::Color32::from_rgb(30, 65, 255),
        },
        DriverInfo {
            number: 2,
            name: "Logan Sargeant",
            team: "Williams",
            color: egui::Color32::from_rgb(0, 82, 255),
        },
        DriverInfo {
            number: 4,
            name: "Lando Norris",
            team: "McLaren",
            color: egui::Color32::from_rgb(255, 135, 0),
        },
        DriverInfo {
            number: 10,
            name: "Pierre Gasly",
            team: "Alpine",
            color: egui::Color32::from_rgb(2, 144, 240),
        },
        DriverInfo {
            number: 11,
            name: "Sergio Perez",
            team: "Red Bull",
            color: egui::Color32::from_rgb(30, 65, 255),
        },
        DriverInfo {
            number: 14,
            name: "Fernando Alonso",
            team: "Aston Martin",
            color: egui::Color32::from_rgb(0, 110, 120),
        },
        DriverInfo {
            number: 16,
            name: "Charles Leclerc",
            team: "Ferrari",
            color: egui::Color32::from_rgb(220, 0, 0),
        },
        DriverInfo {
            number: 18,
            name: "Lance Stroll",
            team: "Aston Martin",
            color: egui::Color32::from_rgb(0, 110, 120),
        },
        DriverInfo {
            number: 20,
            name: "Kevin Magnussen",
            team: "Haas",
            color: egui::Color32::from_rgb(160, 207, 205),
        },
        DriverInfo {
            number: 22,
            name: "Yuki Tsunoda",
            team: "AlphaTauri",
            color: egui::Color32::from_rgb(60, 130, 200),
        },
        DriverInfo {
            number: 23,
            name: "Alex Albon",
            team: "Williams",
            color: egui::Color32::from_rgb(0, 82, 255),
        },
        DriverInfo {
            number: 24,
            name: "Zhou Guanyu",
            team: "Stake F1",
            color: egui::Color32::from_rgb(165, 160, 155),
        },
        DriverInfo {
            number: 27,
            name: "Nico Hulkenberg",
            team: "Haas",
            color: egui::Color32::from_rgb(160, 207, 205),
        },
        DriverInfo {
            number: 31,
            name: "Esteban Ocon",
            team: "Alpine",
            color: egui::Color32::from_rgb(2, 144, 240),
        },
        DriverInfo {
            number: 40,
            name: "Liam Lawson",
            team: "AlphaTauri",
            color: egui::Color32::from_rgb(60, 130, 200),
        },
        DriverInfo {
            number: 44,
            name: "Lewis Hamilton",
            team: "Mercedes",
            color: egui::Color32::from_rgb(0, 210, 190),
        },
        DriverInfo {
            number: 55,
            name: "Carlos Sainz",
            team: "Ferrari",
            color: egui::Color32::from_rgb(220, 0, 0),
        },
        DriverInfo {
            number: 63,
            name: "George Russell",
            team: "Mercedes",
            color: egui::Color32::from_rgb(0, 210, 190),
        },
        DriverInfo {
            number: 77,
            name: "Valtteri Bottas",
            team: "Stake F1",
            color: egui::Color32::from_rgb(165, 160, 155),
        },
        DriverInfo {
            number: 81,
            name: "Oscar Piastri",
            team: "McLaren",
            color: egui::Color32::from_rgb(255, 135, 0),
        },
    ];

    let app = PlotApp::new(coordinates, run_race_data, driver_info);

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
    let start_time: &str = "2023-08-27T12:58:56.200";
    let end_time: &str =  "2023-08-27T13:20:54.300";

    let client = Client::new();
    let mut all_data: Vec<LocationData> = Vec::new();

    for driver_number in driver_numbers {
        let url = format!(
            "https://api.openf1.org/v1/location?session_key={}&driver_number={}&date>{}&date<{}",
            session_key, driver_number, start_time, end_time,
        );
        eprint!("url: {}", url);
        let resp = client.get(&url).send().await?;
        if resp.status().is_success() {
            let data: Vec<LocationData> = resp.json().await?;
            all_data.extend(data.into_iter().filter(|d| d.x != 0.0 && d.y != 0.0));
        } else {
            eprintln!(
                "Failed to fetch data for driver {}: HTTP {}",
                driver_number,
                resp.status()
            );
        }
    }

    // Sort the data by the date field
    all_data.sort_by_key(|d| d.date);
    Ok(all_data)
}

fn read_coordinates() -> Result<Vec<LedCoordinate>, Box<dyn StdError>> {
    Ok(vec![
        LedCoordinate { x_led: 6413.0, y_led: 33.0 }, // U1
        LedCoordinate { x_led: 6007.0, y_led: 197.0 }, // U2
        LedCoordinate { x_led: 5652.0, y_led: 444.0 }, // U3
        LedCoordinate { x_led: 5431.0, y_led: 822.0 }, // U4
        LedCoordinate { x_led: 5727.0, y_led: 1143.0 }, // U5
        LedCoordinate { x_led: 6141.0, y_led: 1268.0 }, // U6
        LedCoordinate { x_led: 6567.0, y_led: 1355.0 }, // U7
        LedCoordinate { x_led: 6975.0, y_led: 1482.0 }, // U8
        LedCoordinate { x_led: 7328.0, y_led: 1738.0 }, // U9
        LedCoordinate { x_led: 7369.0, y_led: 2173.0 }, // U10
        LedCoordinate { x_led: 7024.0, y_led: 2448.0 }, // U11
        LedCoordinate { x_led: 6592.0, y_led: 2505.0 }, // U12
        LedCoordinate { x_led: 6159.0, y_led: 2530.0 }, // U13
        LedCoordinate { x_led: 5725.0, y_led: 2525.0 }, // U14
        LedCoordinate { x_led: 5288.0, y_led: 2489.0 }, // U15
        LedCoordinate { x_led: 4857.0, y_led: 2434.0 }, // U16
        LedCoordinate { x_led: 4429.0, y_led: 2356.0 }, // U17
        LedCoordinate { x_led: 4004.0, y_led: 2249.0 }, // U18
        LedCoordinate { x_led: 3592.0, y_led: 2122.0 }, // U19
        LedCoordinate { x_led: 3181.0, y_led: 1977.0 }, // U20
        LedCoordinate { x_led: 2779.0, y_led: 1812.0 }, // U21
        LedCoordinate { x_led: 2387.0, y_led: 1624.0 }, // U22
        LedCoordinate { x_led: 1988.0, y_led: 1453.0 }, // U23
        LedCoordinate { x_led: 1703.0, y_led: 1779.0 }, // U24
        LedCoordinate { x_led: 1271.0, y_led: 1738.0 }, // U25
        LedCoordinate { x_led: 1189.0, y_led: 1314.0 }, // U26
        LedCoordinate { x_led: 1257.0, y_led: 884.0 }, // U27
        LedCoordinate { x_led: 1333.0, y_led: 454.0 }, // U28
        LedCoordinate { x_led: 1409.0, y_led: 25.0 }, // U29
        LedCoordinate { x_led: 1485.0, y_led: -405.0 }, // U30
        LedCoordinate { x_led: 1558.0, y_led: -835.0 }, // U31
        LedCoordinate { x_led: 1537.0, y_led: -1267.0 }, // U32
        LedCoordinate { x_led: 1208.0, y_led: -1555.0 }, // U33
        LedCoordinate { x_led: 779.0, y_led: -1606.0 }, // U34
        LedCoordinate { x_led: 344.0, y_led: -1604.0 }, // U35
        LedCoordinate { x_led: -88.0, y_led: -1539.0 }, // U36
        LedCoordinate { x_led: -482.0, y_led: -1346.0 }, // U37
        LedCoordinate { x_led: -785.0, y_led: -1038.0 }, // U38
        LedCoordinate { x_led: -966.0, y_led: -644.0 }, // U39
        LedCoordinate { x_led: -1015.0, y_led: -206.0 }, // U40
        LedCoordinate { x_led: -923.0, y_led: 231.0 }, // U41
        LedCoordinate { x_led: -762.0, y_led: 650.0 }, // U42
        LedCoordinate { x_led: -591.0, y_led: 1078.0 }, // U43
        LedCoordinate { x_led: -423.0, y_led: 1497.0 }, // U44
        LedCoordinate { x_led: -254.0, y_led: 1915.0 }, // U45
        LedCoordinate { x_led: -86.0, y_led: 2329.0 }, // U46
        LedCoordinate { x_led: 83.0, y_led: 2744.0 }, // U47
        LedCoordinate { x_led: 251.0, y_led: 3158.0 }, // U48
        LedCoordinate { x_led: 416.0, y_led: 3574.0 }, // U49
        LedCoordinate { x_led: 588.0, y_led: 3990.0 }, // U50
        LedCoordinate { x_led: 755.0, y_led: 4396.0 }, // U51
        LedCoordinate { x_led: 920.0, y_led: 4804.0 }, // U52
        LedCoordinate { x_led: 1086.0, y_led: 5212.0 }, // U53
        LedCoordinate { x_led: 1250.0, y_led: 5615.0 }, // U54
        LedCoordinate { x_led: 1418.0, y_led: 6017.0 }, // U55
        LedCoordinate { x_led: 1583.0, y_led: 6419.0 }, // U56
        LedCoordinate { x_led: 1909.0, y_led: 6702.0 }, // U57
        LedCoordinate { x_led: 2306.0, y_led: 6512.0 }, // U58
        LedCoordinate { x_led: 2319.0, y_led: 6071.0 }, // U59
        LedCoordinate { x_led: 2152.0, y_led: 5660.0 }, // U60
        LedCoordinate { x_led: 1988.0, y_led: 5255.0 }, // U61
        LedCoordinate { x_led: 1853.0, y_led: 4836.0 }, // U62
        LedCoordinate { x_led: 1784.0, y_led: 4407.0 }, // U63
        LedCoordinate { x_led: 1779.0, y_led: 3971.0 }, // U64
        LedCoordinate { x_led: 1605.0, y_led: 3569.0 }, // U65
        LedCoordinate { x_led: 1211.0, y_led: 3375.0 }, // U66
        LedCoordinate { x_led: 811.0, y_led: 3188.0 }, // U67
        LedCoordinate { x_led: 710.0, y_led: 2755.0 }, // U68
        LedCoordinate { x_led: 1116.0, y_led: 2595.0 }, // U69
        LedCoordinate { x_led: 1529.0, y_led: 2717.0 }, // U70
        LedCoordinate { x_led: 1947.0, y_led: 2848.0 }, // U71
        LedCoordinate { x_led: 2371.0, y_led: 2946.0 }, // U72
        LedCoordinate { x_led: 2806.0, y_led: 2989.0 }, // U73
        LedCoordinate { x_led: 3239.0, y_led: 2946.0 }, // U74
        LedCoordinate { x_led: 3665.0, y_led: 2864.0 }, // U75
        LedCoordinate { x_led: 4092.0, y_led: 2791.0 }, // U76
        LedCoordinate { x_led: 4523.0, y_led: 2772.0 }, // U77
        LedCoordinate { x_led: 4945.0, y_led: 2886.0 }, // U78
        LedCoordinate { x_led: 5331.0, y_led: 3087.0 }, // U79
        LedCoordinate { x_led: 5703.0, y_led: 3315.0 }, // U80
        LedCoordinate { x_led: 6105.0, y_led: 3484.0 }, // U81
        LedCoordinate { x_led: 6538.0, y_led: 3545.0 }, // U82
        LedCoordinate { x_led: 6969.0, y_led: 3536.0 }, // U83
        LedCoordinate { x_led: 7402.0, y_led: 3511.0 }, // U84
        LedCoordinate { x_led: 7831.0, y_led: 3476.0 }, // U85
        LedCoordinate { x_led: 8241.0, y_led: 3335.0 }, // U86
        LedCoordinate { x_led: 8549.0, y_led: 3025.0 }, // U87
        LedCoordinate { x_led: 8703.0, y_led: 2612.0 }, // U88
        LedCoordinate { x_led: 8662.0, y_led: 2173.0 }, // U89
        LedCoordinate { x_led: 8451.0, y_led: 1785.0 }, // U90
        LedCoordinate { x_led: 8203.0, y_led: 1426.0 }, // U91
        LedCoordinate { x_led: 7973.0, y_led: 1053.0 }, // U92
        LedCoordinate { x_led: 7777.0, y_led: 664.0 }, // U93
        LedCoordinate { x_led: 7581.0, y_led: 275.0 }, // U94
        LedCoordinate { x_led: 7274.0, y_led: -35.0 }, // U95
        LedCoordinate { x_led: 6839.0, y_led: -46.0 }, // U96
    ])
}

fn generate_run_race_data(
    raw_data: &[LocationData],
    coordinates: &[LedCoordinate],
) -> Vec<RunRace> {
    raw_data
        .iter()
        .map(|data| {
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

            RunRace {
                date: data.date,
                driver_number: data.driver_number,
                x_led: nearest_coord.x_led,
                y_led: nearest_coord.y_led,
            }
        })
        .collect()
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
