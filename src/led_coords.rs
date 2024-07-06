use serde::{Deserialize, Serialize};
use std::error::Error as StdError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedCoordinate {
    pub x_led: f64,
    pub y_led: f64,
    pub led_number: usize,
}

pub fn read_coordinates() -> Result<Vec<LedCoordinate>, Box<dyn StdError>> {
    Ok(vec![
        LedCoordinate {
            x_led: 6413.0,
            y_led: 33.0,
            led_number: 1,
        }, // U1
        LedCoordinate {
            x_led: 6007.0,
            y_led: 197.0,
            led_number: 2,
        }, // U2
        LedCoordinate {
            x_led: 5652.0,
            y_led: 444.0,
            led_number: 3,
        }, // U3
        LedCoordinate {
            x_led: 5431.0,
            y_led: 822.0,
            led_number: 4,
        }, // U4
        LedCoordinate {
            x_led: 5727.0,
            y_led: 1143.0,
            led_number: 5,
        }, // U5
        LedCoordinate {
            x_led: 6141.0,
            y_led: 1268.0,
            led_number: 6,
        }, // U6
        LedCoordinate {
            x_led: 6567.0,
            y_led: 1355.0,
            led_number: 7,
        }, // U7
        LedCoordinate {
            x_led: 6975.0,
            y_led: 1482.0,
            led_number: 8,
        }, // U8
        LedCoordinate {
            x_led: 7328.0,
            y_led: 1738.0,
            led_number: 9,
        }, // U9
        LedCoordinate {
            x_led: 7369.0,
            y_led: 2173.0,
            led_number: 10,
        }, // U10
        LedCoordinate {
            x_led: 7024.0,
            y_led: 2448.0,
            led_number: 11,
        }, // U11
        LedCoordinate {
            x_led: 6592.0,
            y_led: 2505.0,
            led_number: 12,
        }, // U12
        LedCoordinate {
            x_led: 6159.0,
            y_led: 2530.0,
            led_number: 13,
        }, // U13
        LedCoordinate {
            x_led: 5725.0,
            y_led: 2525.0,
            led_number: 14,
        }, // U14
        LedCoordinate {
            x_led: 5288.0,
            y_led: 2489.0,
            led_number: 15,
        }, // U15
        LedCoordinate {
            x_led: 4857.0,
            y_led: 2434.0,
            led_number: 16,
        }, // U16
        LedCoordinate {
            x_led: 4429.0,
            y_led: 2356.0,
            led_number: 17,
        }, // U17
        LedCoordinate {
            x_led: 4004.0,
            y_led: 2249.0,
            led_number: 18,
        }, // U18
        LedCoordinate {
            x_led: 3592.0,
            y_led: 2122.0,
            led_number: 19,
        }, // U19
        LedCoordinate {
            x_led: 3181.0,
            y_led: 1977.0,
            led_number: 20,
        }, // U20
        LedCoordinate {
            x_led: 2779.0,
            y_led: 1812.0,
            led_number: 21,
        }, // U21
        LedCoordinate {
            x_led: 2387.0,
            y_led: 1624.0,
            led_number: 22,
        }, // U22
        LedCoordinate {
            x_led: 1988.0,
            y_led: 1453.0,
            led_number: 23,
        }, // U23
        LedCoordinate {
            x_led: 1703.0,
            y_led: 1779.0,
            led_number: 24,
        }, // U24
        LedCoordinate {
            x_led: 1271.0,
            y_led: 1738.0,
            led_number: 25,
        }, // U25
        LedCoordinate {
            x_led: 1189.0,
            y_led: 1314.0,
            led_number: 26,
        }, // U26
        LedCoordinate {
            x_led: 1257.0,
            y_led: 884.0,
            led_number: 27,
        }, // U27
        LedCoordinate {
            x_led: 1333.0,
            y_led: 454.0,
            led_number: 28,
        }, // U28
        LedCoordinate {
            x_led: 1409.0,
            y_led: 25.0,
            led_number: 29,
        }, // U29
        LedCoordinate {
            x_led: 1485.0,
            y_led: -405.0,
            led_number: 30,
        }, // U30
        LedCoordinate {
            x_led: 1558.0,
            y_led: -835.0,
            led_number: 31,
        }, // U31
        LedCoordinate {
            x_led: 1537.0,
            y_led: -1267.0,
            led_number: 32,
        }, // U32
        LedCoordinate {
            x_led: 1208.0,
            y_led: -1555.0,
            led_number: 33,
        }, // U33
        LedCoordinate {
            x_led: 779.0,
            y_led: -1606.0,
            led_number: 34,
        }, // U34
        LedCoordinate {
            x_led: 344.0,
            y_led: -1604.0,
            led_number: 35,
        }, // U35
        LedCoordinate {
            x_led: -88.0,
            y_led: -1539.0,
            led_number: 36,
        }, // U36
        LedCoordinate {
            x_led: -482.0,
            y_led: -1346.0,
            led_number: 37,
        }, // U37
        LedCoordinate {
            x_led: -785.0,
            y_led: -1038.0,
            led_number: 38,
        }, // U38
        LedCoordinate {
            x_led: -966.0,
            y_led: -644.0,
            led_number: 39,
        }, // U39
        LedCoordinate {
            x_led: -1015.0,
            y_led: -206.0,
            led_number: 40,
        }, // U40
        LedCoordinate {
            x_led: -923.0,
            y_led: 231.0,
            led_number: 41,
        }, // U41
        LedCoordinate {
            x_led: -762.0,
            y_led: 650.0,
            led_number: 42,
        }, // U42
        LedCoordinate {
            x_led: -591.0,
            y_led: 1078.0,
            led_number: 43,
        }, // U43
        LedCoordinate {
            x_led: -423.0,
            y_led: 1497.0,
            led_number: 44,
        }, // U44
        LedCoordinate {
            x_led: -254.0,
            y_led: 1915.0,
            led_number: 45,
        }, // U45
        LedCoordinate {
            x_led: -86.0,
            y_led: 2329.0,
            led_number: 46,
        }, // U46
        LedCoordinate {
            x_led: 83.0,
            y_led: 2744.0,
            led_number: 47,
        }, // U47
        LedCoordinate {
            x_led: 251.0,
            y_led: 3158.0,
            led_number: 48,
        }, // U48
        LedCoordinate {
            x_led: 416.0,
            y_led: 3574.0,
            led_number: 49,
        }, // U49
        LedCoordinate {
            x_led: 588.0,
            y_led: 3990.0,
            led_number: 50,
        }, // U50
        LedCoordinate {
            x_led: 755.0,
            y_led: 4396.0,
            led_number: 51,
        }, // U51
        LedCoordinate {
            x_led: 920.0,
            y_led: 4804.0,
            led_number: 52,
        }, // U52
        LedCoordinate {
            x_led: 1086.0,
            y_led: 5212.0,
            led_number: 53,
        }, // U53
        LedCoordinate {
            x_led: 1250.0,
            y_led: 5615.0,
            led_number: 54,
        }, // U54
        LedCoordinate {
            x_led: 1418.0,
            y_led: 6017.0,
            led_number: 55,
        }, // U55
        LedCoordinate {
            x_led: 1583.0,
            y_led: 6419.0,
            led_number: 56,
        }, // U56
        LedCoordinate {
            x_led: 1909.0,
            y_led: 6702.0,
            led_number: 57,
        }, // U57
        LedCoordinate {
            x_led: 2306.0,
            y_led: 6512.0,
            led_number: 58,
        }, // U58
        LedCoordinate {
            x_led: 2319.0,
            y_led: 6071.0,
            led_number: 59,
        }, // U59
        LedCoordinate {
            x_led: 2152.0,
            y_led: 5660.0,
            led_number: 60,
        }, // U60
        LedCoordinate {
            x_led: 1988.0,
            y_led: 5255.0,
            led_number: 61,
        }, // U61
        LedCoordinate {
            x_led: 1853.0,
            y_led: 4836.0,
            led_number: 62,
        }, // U62
        LedCoordinate {
            x_led: 1784.0,
            y_led: 4407.0,
            led_number: 63,
        }, // U63
        LedCoordinate {
            x_led: 1779.0,
            y_led: 3971.0,
            led_number: 64,
        }, // U64
        LedCoordinate {
            x_led: 1605.0,
            y_led: 3569.0,
            led_number: 65,
        }, // U65
        LedCoordinate {
            x_led: 1211.0,
            y_led: 3375.0,
            led_number: 66,
        }, // U66
        LedCoordinate {
            x_led: 811.0,
            y_led: 3188.0,
            led_number: 67,
        }, // U67
        LedCoordinate {
            x_led: 710.0,
            y_led: 2755.0,
            led_number: 68,
        }, // U68
        LedCoordinate {
            x_led: 1116.0,
            y_led: 2595.0,
            led_number: 69,
        }, // U69
        LedCoordinate {
            x_led: 1529.0,
            y_led: 2717.0,
            led_number: 70,
        }, // U70
        LedCoordinate {
            x_led: 1947.0,
            y_led: 2848.0,
            led_number: 71,
        }, // U71
        LedCoordinate {
            x_led: 2371.0,
            y_led: 2946.0,
            led_number: 72,
        }, // U72
        LedCoordinate {
            x_led: 2806.0,
            y_led: 2989.0,
            led_number: 73,
        }, // U73
        LedCoordinate {
            x_led: 3239.0,
            y_led: 2946.0,
            led_number: 74,
        }, // U74
        LedCoordinate {
            x_led: 3665.0,
            y_led: 2864.0,
            led_number: 75,
        }, // U75
        LedCoordinate {
            x_led: 4092.0,
            y_led: 2791.0,
            led_number: 76,
        }, // U76
        LedCoordinate {
            x_led: 4523.0,
            y_led: 2772.0,
            led_number: 77,
        }, // U77
        LedCoordinate {
            x_led: 4945.0,
            y_led: 2886.0,
            led_number: 78,
        }, // U78
        LedCoordinate {
            x_led: 5331.0,
            y_led: 3087.0,
            led_number: 79,
        }, // U79
        LedCoordinate {
            x_led: 5703.0,
            y_led: 3315.0,
            led_number: 80,
        }, // U80
        LedCoordinate {
            x_led: 6105.0,
            y_led: 3484.0,
            led_number: 81,
        }, // U81
        LedCoordinate {
            x_led: 6538.0,
            y_led: 3545.0,
            led_number: 82,
        }, // U82
        LedCoordinate {
            x_led: 6969.0,
            y_led: 3536.0,
            led_number: 83,
        }, // U83
        LedCoordinate {
            x_led: 7402.0,
            y_led: 3511.0,
            led_number: 84,
        }, // U84
        LedCoordinate {
            x_led: 7831.0,
            y_led: 3476.0,
            led_number: 85,
        }, // U85
        LedCoordinate {
            x_led: 8241.0,
            y_led: 3335.0,
            led_number: 86,
        }, // U86
        LedCoordinate {
            x_led: 8549.0,
            y_led: 3025.0,
            led_number: 87,
        }, // U87
        LedCoordinate {
            x_led: 8703.0,
            y_led: 2612.0,
            led_number: 88,
        }, // U88
        LedCoordinate {
            x_led: 8662.0,
            y_led: 2173.0,
            led_number: 89,
        }, // U89
        LedCoordinate {
            x_led: 8451.0,
            y_led: 1785.0,
            led_number: 90,
        }, // U90
        LedCoordinate {
            x_led: 8203.0,
            y_led: 1426.0,
            led_number: 91,
        }, // U91
        LedCoordinate {
            x_led: 7973.0,
            y_led: 1053.0,
            led_number: 92,
        }, // U92
        LedCoordinate {
            x_led: 7777.0,
            y_led: 664.0,
            led_number: 93,
        }, // U93
        LedCoordinate {
            x_led: 7581.0,
            y_led: 275.0,
            led_number: 94,
        }, // U94
        LedCoordinate {
            x_led: 7274.0,
            y_led: -35.0,
            led_number: 95,
        }, // U95
        LedCoordinate {
            x_led: 6839.0,
            y_led: -46.0,
            led_number: 96,
        }, // U96
    ])
}
