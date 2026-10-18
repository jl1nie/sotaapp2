//! 周波数から、バンドの名前を引く(旧 FLE も使う)

use once_cell::sync::Lazy;

/// バンド(周波数の範囲と、各所での名前)
#[derive(Debug, Clone)]
pub struct BandInfo {
    pub lower: f64,
    pub upper: f64,
    pub band_air: &'static str,
    pub band_sota: &'static str,
    pub wavelength: &'static str,
}

/// 周波数(MHz)の範囲の表
pub static FREQ_TABLE: Lazy<Vec<BandInfo>> = Lazy::new(|| {
    vec![
        BandInfo {
            lower: 0.1357,
            upper: 0.1378,
            band_air: "135kHz",
            band_sota: "VLF",
            wavelength: "2190m",
        },
        BandInfo {
            lower: 0.472,
            upper: 0.479,
            band_air: "475kHz",
            band_sota: "VLF",
            wavelength: "630m",
        },
        BandInfo {
            lower: 1.8,
            upper: 1.9125,
            band_air: "1.9MHz",
            band_sota: "1.8MHz",
            wavelength: "160m",
        },
        BandInfo {
            lower: 3.5,
            upper: 3.805,
            band_air: "3.8MHz",
            band_sota: "3.5MHz",
            wavelength: "80m",
        },
        BandInfo {
            lower: 7.0,
            upper: 7.2,
            band_air: "7MHz",
            band_sota: "7MHz",
            wavelength: "40m",
        },
        BandInfo {
            lower: 10.0,
            upper: 10.150,
            band_air: "10MHz",
            band_sota: "10MHz",
            wavelength: "30m",
        },
        BandInfo {
            lower: 14.0,
            upper: 14.350,
            band_air: "14MHz",
            band_sota: "14MHz",
            wavelength: "20m",
        },
        BandInfo {
            lower: 18.0,
            upper: 18.168,
            band_air: "18MHz",
            band_sota: "18MHz",
            wavelength: "17m",
        },
        BandInfo {
            lower: 21.0,
            upper: 21.450,
            band_air: "21MHz",
            band_sota: "21MHz",
            wavelength: "15m",
        },
        BandInfo {
            lower: 24.0,
            upper: 24.990,
            band_air: "24MHz",
            band_sota: "24MHz",
            wavelength: "12m",
        },
        BandInfo {
            lower: 28.0,
            upper: 29.7,
            band_air: "28MHz",
            band_sota: "28MHz",
            wavelength: "10m",
        },
        BandInfo {
            lower: 50.0,
            upper: 54.0,
            band_air: "50MHz",
            band_sota: "50MHz",
            wavelength: "6m",
        },
        BandInfo {
            lower: 144.0,
            upper: 146.0,
            band_air: "144MHz",
            band_sota: "144MHz",
            wavelength: "2m",
        },
        BandInfo {
            lower: 430.0,
            upper: 440.0,
            band_air: "430MHz",
            band_sota: "433MHz",
            wavelength: "70cm",
        },
        BandInfo {
            lower: 1200.0,
            upper: 1300.0,
            band_air: "1200MHz",
            band_sota: "1290MHz",
            wavelength: "23cm",
        },
        BandInfo {
            lower: 2400.0,
            upper: 2450.0,
            band_air: "2400MHz",
            band_sota: "2.3GHz",
            wavelength: "13cm",
        },
        BandInfo {
            lower: 5650.0,
            upper: 5850.0,
            band_air: "5600MHz",
            band_sota: "5.6GHz",
            wavelength: "6cm",
        },
        BandInfo {
            lower: 10000.0,
            upper: 10250.0,
            band_air: "10.1GHz",
            band_sota: "10GHz",
            wavelength: "3cm",
        },
        BandInfo {
            lower: 10450.0,
            upper: 10500.0,
            band_air: "10.4GHz",
            band_sota: "10GHz",
            wavelength: "3cm",
        },
        // Non-amateur bands (Japanese specific)
        BandInfo {
            lower: 351.0,
            upper: 351.38125,
            band_air: "デジタル簡易(351MHz)",
            band_sota: "",
            wavelength: "",
        },
        BandInfo {
            lower: 421.0,
            upper: 454.19375,
            band_air: "特定小電力(422MHz)",
            band_sota: "",
            wavelength: "",
        },
        BandInfo {
            lower: 26.968,
            upper: 27.144,
            band_air: "CB(27MHz)",
            band_sota: "",
            wavelength: "",
        },
        BandInfo {
            lower: 142.0,
            upper: 147.0,
            band_air: "デジタル小電力コミュニティ(142/146MHz)",
            band_sota: "",
            wavelength: "",
        },
    ]
});

/// 周波数(MHz の文字列。`7.030/7.035` のように `/` があれば、最初の周波数)から、バンドの名前を引く
///
/// 戻り値は(アマチュア局の名前、SOTA での名前、波長での名前)。どのバンドにもなければ誤り。
pub fn freq_to_band(freq_str: &str) -> Result<(&'static str, &'static str, &'static str), String> {
    // Handle "freq1/freq2" format - use first frequency
    let freq_str = if let Some(idx) = freq_str.find('/') {
        &freq_str[..idx]
    } else {
        freq_str
    };

    let freq: f64 = freq_str.parse().unwrap_or(0.0);

    for band in FREQ_TABLE.iter() {
        if freq >= band.lower && freq <= band.upper {
            return Ok((band.band_air, band.band_sota, band.wavelength));
        }
    }

    Err(format!("Unknown frequency: {}", freq_str))
}

/// バンドの波長での名前(`40m`)から、周波数の代表値を引く
pub fn band_to_freq(band_str: &str, is_sota: bool) -> Option<&'static str> {
    let band_upper = band_str.to_uppercase();
    for band in FREQ_TABLE.iter() {
        if band.wavelength.to_uppercase() == band_upper {
            return Some(if is_sota {
                band.band_sota
            } else {
                band.band_air
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_freq_to_band() {
        let (air, sota, wlen) = freq_to_band("7.025").unwrap();
        assert_eq!((air, sota, wlen), ("7MHz", "7MHz", "40m"));

        let (air, sota, wlen) = freq_to_band("14.230").unwrap();
        assert_eq!((air, sota, wlen), ("14MHz", "14MHz", "20m"));

        assert_eq!(freq_to_band("144.500").unwrap().0, "144MHz");
        // SOTA での名前は、バンドによって違う(VLF、433MHz など)
        assert_eq!(freq_to_band("0.475").unwrap().1, "VLF");
        assert_eq!(freq_to_band("435.0").unwrap().1, "433MHz");
    }

    #[test]
    fn a_frequency_with_a_slash_uses_the_first() {
        assert_eq!(freq_to_band("7.030/7.035").unwrap().2, "40m");
    }

    #[test]
    fn an_unknown_frequency_is_an_error() {
        assert_eq!(
            freq_to_band("999.999").unwrap_err(),
            "Unknown frequency: 999.999"
        );
        assert!(freq_to_band("abc").is_err());
    }
}
