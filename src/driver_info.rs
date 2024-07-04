use eframe::egui;

#[derive(Debug, Clone)]
pub struct DriverInfo {
    pub number: u32,
    pub name: &'static str,
    pub team: &'static str,
    pub color: egui::Color32,
}

pub fn get_driver_info() -> Vec<DriverInfo> {
    vec![
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
    ]
}
