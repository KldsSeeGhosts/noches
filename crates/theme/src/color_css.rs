//! Bounded, dependency-free CSS color parsing for local theme files.
use crate::{Color, ColorParseError};

fn number(value: &str) -> Result<f32, ColorParseError> {
    value
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
        .ok_or(ColorParseError)
}

fn scaled(value: &str, percent_scale: f32) -> Result<f32, ColorParseError> {
    match value.strip_suffix('%') {
        Some(value) => Ok(number(value)? * percent_scale / 100.0),
        None => number(value),
    }
}

fn byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

pub(crate) fn parse(value: &str) -> Result<Color, ColorParseError> {
    let (function, body) = value.split_once('(').ok_or(ColorParseError)?;
    let body = body.strip_suffix(')').ok_or(ColorParseError)?.trim();
    match function.to_ascii_lowercase().as_str() {
        "rgb" | "rgba" => rgb(body),
        "oklch" => oklch(body),
        _ => Err(ColorParseError),
    }
}

fn components(body: &str) -> Result<(Vec<&str>, f32), ColorParseError> {
    let (channels, alpha) = match body.split_once('/') {
        Some((channels, alpha)) => (channels, scaled(alpha.trim(), 1.0)?),
        None => (body, 1.0),
    };
    Ok((channels.split_whitespace().collect(), alpha))
}

fn rgb(body: &str) -> Result<Color, ColorParseError> {
    let (channels, alpha) = if body.contains(',') {
        if body.contains('/') {
            return Err(ColorParseError);
        }
        let mut values = body.split(',').map(str::trim).collect::<Vec<_>>();
        let alpha = if values.len() == 4 {
            scaled(values.pop().ok_or(ColorParseError)?, 1.0)?
        } else {
            1.0
        };
        (values, alpha)
    } else {
        components(body)?
    };
    if channels.len() != 3 {
        return Err(ColorParseError);
    }
    Ok(Color::rgba(
        byte(scaled(channels[0], 255.0)? / 255.0),
        byte(scaled(channels[1], 255.0)? / 255.0),
        byte(scaled(channels[2], 255.0)? / 255.0),
        byte(alpha),
    ))
}

fn hue(value: &str) -> Result<f32, ColorParseError> {
    for (suffix, scale) in [
        ("grad", 0.9),
        ("deg", 1.0),
        ("turn", 360.0),
        ("rad", 180.0 / std::f32::consts::PI),
    ] {
        if let Some(value) = value.strip_suffix(suffix) {
            return Ok(number(value)? * scale);
        }
    }
    number(value)
}

fn oklch(body: &str) -> Result<Color, ColorParseError> {
    let (values, alpha) = components(body)?;
    if values.len() != 3 {
        return Err(ColorParseError);
    }
    let l = scaled(values[0], 1.0)?.clamp(0.0, 1.0);
    let c = scaled(values[1], 0.4)?.max(0.0);
    let h = hue(values[2])?.rem_euclid(360.0).to_radians();
    let (a, b) = (c * h.cos(), c * h.sin());
    let l3 = (l + 0.39633778 * a + 0.21580376 * b).powi(3);
    let m3 = (l - 0.105561346 * a - 0.06385417 * b).powi(3);
    let s3 = (l - 0.08948418 * a - 1.2914855 * b).powi(3);
    let srgb = |linear: f32| {
        byte(if linear <= 0.0031308 {
            12.92 * linear
        } else {
            1.055 * linear.powf(1.0 / 2.4) - 0.055
        })
    };
    Ok(Color::rgba(
        srgb(4.0767417 * l3 - 3.3077116 * m3 + 0.23096994 * s3),
        srgb(-1.268438 * l3 + 2.6097574 * m3 - 0.34131938 * s3),
        srgb(-0.0041960863 * l3 - 0.7034186 * m3 + 1.7076147 * s3),
        byte(alpha),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_rgb_and_alpha_forms() {
        for (source, expected) in [
            ("rgb(10, 20, 30)", Color::rgb(10, 20, 30)),
            ("RGBA(10,20,30,0.5)", Color::rgba(10, 20, 30, 128)),
            ("rgb(100% 0% 50% / 25%)", Color::rgba(255, 0, 128, 64)),
            ("rgb(-20 300 0)", Color::rgb(0, 255, 0)),
        ] {
            assert_eq!(source.parse::<Color>().unwrap(), expected, "{source}");
        }
    }

    #[test]
    fn oklch_conversion_and_units() {
        assert_eq!("oklch(0 0 0)".parse::<Color>().unwrap(), Color::BLACK);
        assert_eq!("oklch(100% 0 0)".parse::<Color>().unwrap(), Color::WHITE);
        assert_eq!(
            "oklch(62.8% 0.2577 29.23 / 50%)".parse::<Color>().unwrap(),
            Color::rgba(255, 0, 0, 128)
        );
        let reference: Color = "oklch(.7 .1 180)".parse().unwrap();
        for source in [
            "oklch(.7 .1 .5turn)",
            "oklch(.7 .1 200grad)",
            "oklch(.7 .1 180deg)",
        ] {
            assert_eq!(source.parse::<Color>().unwrap(), reference);
        }
    }

    #[test]
    fn malformed_and_nonfinite_values_are_rejected() {
        for source in [
            "",
            "red",
            "rgb(1 2)",
            "rgba(1,2,3,4,5)",
            "rgb(1,2,3 / .5)",
            "rgb(NaN 0 0)",
            "rgb(0 0 inf)",
            "oklch(.5 .1)",
            "oklch(.5 inf 10)",
            "oklch(.5 .1 10 / .5 / .3)",
            "#🔴",
        ] {
            assert!(source.parse::<Color>().is_err(), "{source}");
        }
    }
}
