mod data;
use eframe::{egui, App, Frame};
use std::collections::HashMap;
use std::error::Error as StdError;
use std::time::Instant;

#[derive(Debug)]
struct DriverInfo {
    number: u32,
    name: &'static str,
    team: &'static str,
    color: egui::Color32,
}

struct PlotApp {
    update_rate_ms: u64,
    frames: Vec<data::UpdateFrame>,
    led_coordinates: Vec<data::LedCoordinate>,
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
        frames: Vec<data::UpdateFrame>,
        led_coordinates: Vec<data::LedCoordinate>,
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
    // Import data from data.rs
    let visualization_data = &data::VISUALIZATION_DATA;
    let update_rate_ms = visualization_data.update_rate_ms;
    let frames = visualization_data.frames.to_vec();
    let led_coordinates = read_coordinates()?;

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

    let app = PlotApp::new(update_rate_ms.into(), frames, led_coordinates, driver_info);

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "F1-LED-CIRCUIT SIMULATION",
        native_options,
        Box::new(|_cc| Box::new(app)),
    )?;

    Ok(())
}

fn read_coordinates() -> Result<Vec<data::LedCoordinate>, Box<dyn StdError>> {
    Ok(vec![
        data::LedCoordinate { x_led: 6413.0, y_led: 33.0, led_number: 1 }, // U1
        data::LedCoordinate { x_led: 6007.0, y_led: 197.0, led_number: 2 }, // U2
        data::LedCoordinate { x_led: 5652.0, y_led: 444.0, led_number: 3 }, // U3
        data::LedCoordinate { x_led: 5431.0, y_led: 822.0, led_number: 4 }, // U4
        data::LedCoordinate { x_led: 5727.0, y_led: 1143.0, led_number: 5 }, // U5
        data::LedCoordinate { x_led: 6141.0, y_led: 1268.0, led_number: 6 }, // U6
        data::LedCoordinate { x_led: 6567.0, y_led: 1355.0, led_number: 7 }, // U7
        data::LedCoordinate { x_led: 6975.0, y_led: 1482.0, led_number: 8 }, // U8
        data::LedCoordinate { x_led: 7328.0, y_led: 1738.0, led_number: 9 }, // U9
        data::LedCoordinate { x_led: 7369.0, y_led: 2173.0, led_number: 10 }, // U10
        data::LedCoordinate { x_led: 7024.0, y_led: 2448.0, led_number: 11 }, // U11
        data::LedCoordinate { x_led: 6592.0, y_led: 2505.0, led_number: 12 }, // U12
        data::LedCoordinate { x_led: 6159.0, y_led: 2530.0, led_number: 13 }, // U13
        data::LedCoordinate { x_led: 5725.0, y_led: 2525.0, led_number: 14 }, // U14
        data::LedCoordinate { x_led: 5288.0, y_led: 2489.0, led_number: 15 }, // U15
        data::LedCoordinate { x_led: 4857.0, y_led: 2434.0, led_number: 16 }, // U16
        data::LedCoordinate { x_led: 4429.0, y_led: 2356.0, led_number: 17 }, // U17
        data::LedCoordinate { x_led: 4004.0, y_led: 2249.0, led_number: 18 }, // U18
        data::LedCoordinate { x_led: 3592.0, y_led: 2122.0, led_number: 19 }, // U19
        data::LedCoordinate { x_led: 3181.0, y_led: 1977.0, led_number: 20 }, // U20
        data::LedCoordinate { x_led: 2779.0, y_led: 1812.0, led_number: 21 }, // U21
        data::LedCoordinate { x_led: 2387.0, y_led: 1624.0, led_number: 22 }, // U22
        data::LedCoordinate { x_led: 1988.0, y_led: 1453.0, led_number: 23 }, // U23
        data::LedCoordinate { x_led: 1703.0, y_led: 1779.0, led_number: 24 }, // U24
        data::LedCoordinate { x_led: 1271.0, y_led: 1738.0, led_number: 25 }, // U25
        data::LedCoordinate { x_led: 1189.0, y_led: 1314.0, led_number: 26 }, // U26
        data::LedCoordinate { x_led: 1257.0, y_led: 884.0, led_number: 27 }, // U27
        data::LedCoordinate { x_led: 1333.0, y_led: 454.0, led_number: 28 }, // U28
        data::LedCoordinate { x_led: 1409.0, y_led: 25.0, led_number: 29 }, // U29
        data::LedCoordinate { x_led: 1485.0, y_led: -405.0, led_number: 30 }, // U30
        data::LedCoordinate { x_led: 1558.0, y_led: -835.0, led_number: 31 }, // U31
        data::LedCoordinate { x_led: 1537.0, y_led: -1267.0, led_number: 32 }, // U32
        data::LedCoordinate { x_led: 1208.0, y_led: -1555.0, led_number: 33 }, // U33
        data::LedCoordinate { x_led: 779.0, y_led: -1606.0, led_number: 34 }, // U34
        data::LedCoordinate { x_led: 344.0, y_led: -1604.0, led_number: 35 }, // U35
        data::LedCoordinate { x_led: -88.0, y_led: -1539.0, led_number: 36 }, // U36
        data::LedCoordinate { x_led: -482.0, y_led: -1346.0, led_number: 37 }, // U37
        data::LedCoordinate { x_led: -785.0, y_led: -1038.0, led_number: 38 }, // U38
        data::LedCoordinate { x_led: -966.0, y_led: -644.0, led_number: 39 }, // U39
        data::LedCoordinate { x_led: -1015.0, y_led: -206.0, led_number: 40 }, // U40
        data::LedCoordinate { x_led: -923.0, y_led: 231.0, led_number: 41 }, // U41
        data::LedCoordinate { x_led: -762.0, y_led: 650.0, led_number: 42 }, // U42
        data::LedCoordinate { x_led: -591.0, y_led: 1078.0, led_number: 43 }, // U43
        data::LedCoordinate { x_led: -423.0, y_led: 1497.0, led_number: 44 }, // U44
        data::LedCoordinate { x_led: -254.0, y_led: 1915.0, led_number: 45 }, // U45
        data::LedCoordinate { x_led: -86.0, y_led: 2329.0, led_number: 46 }, // U46
        data::LedCoordinate { x_led: 83.0, y_led: 2744.0, led_number: 47 }, // U47
        data::LedCoordinate { x_led: 251.0, y_led: 3158.0, led_number: 48 }, // U48
        data::LedCoordinate { x_led: 416.0, y_led: 3574.0, led_number: 49 }, // U49
        data::LedCoordinate { x_led: 588.0, y_led: 3990.0, led_number: 50 }, // U50
        data::LedCoordinate { x_led: 755.0, y_led: 4396.0, led_number: 51 }, // U51
        data::LedCoordinate { x_led: 920.0, y_led: 4804.0, led_number: 52 }, // U52
        data::LedCoordinate { x_led: 1086.0, y_led: 5212.0, led_number: 53 }, // U53
        data::LedCoordinate { x_led: 1250.0, y_led: 5615.0, led_number: 54 }, // U54
        data::LedCoordinate { x_led: 1418.0, y_led: 6017.0, led_number: 55 }, // U55
        data::LedCoordinate { x_led: 1583.0, y_led: 6419.0, led_number: 56 }, // U56
        data::LedCoordinate { x_led: 1909.0, y_led: 6702.0, led_number: 57 }, // U57
        data::LedCoordinate { x_led: 2306.0, y_led: 6512.0, led_number: 58 }, // U58
        data::LedCoordinate { x_led: 2319.0, y_led: 6071.0, led_number: 59 }, // U59
        data::LedCoordinate { x_led: 2152.0, y_led: 5660.0, led_number: 60 }, // U60
        data::LedCoordinate { x_led: 1988.0, y_led: 5255.0, led_number: 61 }, // U61
        data::LedCoordinate { x_led: 1853.0, y_led: 4836.0, led_number: 62 }, // U62
        data::LedCoordinate { x_led: 1784.0, y_led: 4407.0, led_number: 63 }, // U63
        data::LedCoordinate { x_led: 1779.0, y_led: 3971.0, led_number: 64 }, // U64
        data::LedCoordinate { x_led: 1605.0, y_led: 3569.0, led_number: 65 }, // U65
        data::LedCoordinate { x_led: 1211.0, y_led: 3375.0, led_number: 66 }, // U66
        data::LedCoordinate { x_led: 811.0, y_led: 3188.0, led_number: 67 }, // U67
        data::LedCoordinate { x_led: 710.0, y_led: 2755.0, led_number: 68 }, // U68
        data::LedCoordinate { x_led: 1116.0, y_led: 2595.0, led_number: 69 }, // U69
        data::LedCoordinate { x_led: 1529.0, y_led: 2717.0, led_number: 70 }, // U70
        data::LedCoordinate { x_led: 1947.0, y_led: 2848.0, led_number: 71 }, // U71
        data::LedCoordinate { x_led: 2371.0, y_led: 2946.0, led_number: 72 }, // U72
        data::LedCoordinate { x_led: 2806.0, y_led: 2989.0, led_number: 73 }, // U73
        data::LedCoordinate { x_led: 3239.0, y_led: 2946.0, led_number: 74 }, // U74
        data::LedCoordinate { x_led: 3665.0, y_led: 2864.0, led_number: 75 }, // U75
        data::LedCoordinate { x_led: 4092.0, y_led: 2791.0, led_number: 76 }, // U76
        data::LedCoordinate { x_led: 4523.0, y_led: 2772.0, led_number: 77 }, // U77
        data::LedCoordinate { x_led: 4945.0, y_led: 2886.0, led_number: 78 }, // U78
        data::LedCoordinate { x_led: 5331.0, y_led: 3087.0, led_number: 79 }, // U79
        data::LedCoordinate { x_led: 5703.0, y_led: 3315.0, led_number: 80 }, // U80
        data::LedCoordinate { x_led: 6105.0, y_led: 3484.0, led_number: 81 }, // U81
        data::LedCoordinate { x_led: 6538.0, y_led: 3545.0, led_number: 82 }, // U82
        data::LedCoordinate { x_led: 6969.0, y_led: 3536.0, led_number: 83 }, // U83
        data::LedCoordinate { x_led: 7402.0, y_led: 3511.0, led_number: 84 }, // U84
        data::LedCoordinate { x_led: 7831.0, y_led: 3476.0, led_number: 85 }, // U85
        data::LedCoordinate { x_led: 8241.0, y_led: 3335.0, led_number: 86 }, // U86
        data::LedCoordinate { x_led: 8549.0, y_led: 3025.0, led_number: 87 }, // U87
        data::LedCoordinate { x_led: 8703.0, y_led: 2612.0, led_number: 88 }, // U88
        data::LedCoordinate { x_led: 8662.0, y_led: 2173.0, led_number: 89 }, // U89
        data::LedCoordinate { x_led: 8451.0, y_led: 1785.0, led_number: 90 }, // U90
        data::LedCoordinate { x_led: 8203.0, y_led: 1426.0, led_number: 91 }, // U91
        data::LedCoordinate { x_led: 7973.0, y_led: 1053.0, led_number: 92 }, // U92
        data::LedCoordinate { x_led: 7777.0, y_led: 664.0, led_number: 93 }, // U93
        data::LedCoordinate { x_led: 7581.0, y_led: 275.0, led_number: 94 }, // U94
        data::LedCoordinate { x_led: 7274.0, y_led: -35.0, led_number: 95 }, // U95
        data::LedCoordinate { x_led: 6839.0, y_led: -46.0, led_number: 96 }, // U96
    ])
}
