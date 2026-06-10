//! Calendar arithmetic over FHIRPath's partial-precision date/dateTime text
//! forms. Pure text-in/text-out: precision is preserved, month/year addition
//! clamps to the end of the target month, time units carry across days, and
//! any timezone suffix passes through untouched.

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

struct Parts {
    // [year, month, day, hour, minute, second] - length encodes the precision
    fields: Vec<i64>,
    frac: String,
    zone: String,
}

// The zone suffix (Z or +/-hh:mm) can only appear after the time part.
fn split_zone(text: &str) -> (&str, &str) {
    if let Some(t_pos) = text.find('T') {
        for (i, c) in text[t_pos..].char_indices() {
            if matches!(c, 'Z' | '+' | '-') {
                return (&text[..t_pos + i], &text[t_pos + i..]);
            }
        }
    }
    (text, "")
}

fn parse(text: &str) -> Option<Parts> {
    let (main, zone) = split_zone(text);
    let (date_part, time_part) = match main.split_once('T') {
        Some((d, t)) => (d, Some(t)),
        None => (main, None),
    };
    let mut fields = Vec::new();
    for (i, seg) in date_part.split('-').enumerate() {
        let want = if i == 0 { 4 } else { 2 };
        if i > 2 || seg.len() != want || !seg.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        fields.push(seg.parse().ok()?);
    }
    let mut frac = String::new();
    if let Some(t) = time_part {
        let (hms, f) = match t.split_once('.') {
            Some((a, b)) => (a, format!(".{b}")),
            None => (t, String::new()),
        };
        frac = f;
        for (i, seg) in hms.split(':').enumerate() {
            if i > 2 || seg.len() != 2 || !seg.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            fields.push(seg.parse().ok()?);
        }
    }
    if fields.is_empty() || (fields.len() >= 2 && !(1..=12).contains(&fields[1])) {
        return None;
    }
    if fields.len() >= 3 && !(1..=31).contains(&fields[2]) {
        return None;
    }
    Some(Parts {
        fields,
        frac,
        zone: zone.to_string(),
    })
}

fn render(p: &Parts) -> String {
    let f = &p.fields;
    let mut s = format!("{:04}", f[0]);
    let seps = ["", "-", "-", "T", ":", ":"];
    for i in 1..f.len() {
        s += &format!("{}{:02}", seps[i], f[i]);
    }
    s + &p.frac + &p.zone
}

fn leap(y: i64) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if leap(y) {
                29
            } else {
                28
            }
        }
    }
}

// civil <-> day-number conversion (Howard Hinnant's algorithms)
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Add `amount` of `unit` to a date/dateTime text. None when the text does
/// not parse, the unit is finer than the text's precision, or the amount is
/// not a whole number.
pub(crate) fn add(text: &str, amount: Decimal, unit: &str) -> Option<String> {
    if !amount.fract().is_zero() {
        return None;
    }
    let n = amount.to_i64()?;
    let mut p = parse(text)?;
    let prec = p.fields.len();
    let needed = match unit {
        "year" => 1,
        "month" => 2,
        "week" | "day" => 3,
        "hour" => 4,
        "minute" => 5,
        "second" => 6,
        _ => return None,
    };
    if prec < needed {
        return None;
    }
    match unit {
        "year" => p.fields[0] += n,
        "month" => {
            let months = p.fields[0] * 12 + (p.fields[1] - 1) + n;
            p.fields[0] = months.div_euclid(12);
            p.fields[1] = months.rem_euclid(12) + 1;
        }
        "week" | "day" => {
            let mult = if unit == "week" { 7 } else { 1 };
            let days = days_from_civil(p.fields[0], p.fields[1], p.fields[2]) + n * mult;
            let (y, m, d) = civil_from_days(days);
            (p.fields[0], p.fields[1], p.fields[2]) = (y, m, d);
        }
        _ => {
            let per = match unit {
                "hour" => 3600,
                "minute" => 60,
                _ => 1,
            };
            let mi = p.fields.get(4).copied().unwrap_or(0);
            let ss = p.fields.get(5).copied().unwrap_or(0);
            let total = p.fields[3] * 3600 + mi * 60 + ss + n * per;
            let days = days_from_civil(p.fields[0], p.fields[1], p.fields[2])
                + total.div_euclid(86400);
            let tod = total.rem_euclid(86400);
            let (y, m, d) = civil_from_days(days);
            (p.fields[0], p.fields[1], p.fields[2]) = (y, m, d);
            p.fields[3] = tod / 3600;
            if prec >= 5 {
                p.fields[4] = (tod % 3600) / 60;
            }
            if prec >= 6 {
                p.fields[5] = tod % 60;
            }
        }
    }
    // year/month addition can land on a shorter month
    if prec >= 3 {
        p.fields[2] = p.fields[2].min(days_in_month(p.fields[0], p.fields[1]));
    }
    Some(render(&p))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(s: &str) -> Decimal {
        s.parse().unwrap()
    }

    #[test]
    fn adds_with_precision_preserved() {
        assert_eq!(add("2014", dec("1"), "year").unwrap(), "2015");
        assert_eq!(add("2014-01", dec("1"), "month").unwrap(), "2014-02");
        assert_eq!(add("2014-01-31", dec("1"), "day").unwrap(), "2014-02-01");
        assert_eq!(add("2014-12-31", dec("1"), "day").unwrap(), "2015-01-01");
        assert_eq!(add("2014-01-01", dec("2"), "week").unwrap(), "2014-01-15");
    }

    #[test]
    fn clamps_end_of_month() {
        assert_eq!(add("2014-01-31", dec("1"), "month").unwrap(), "2014-02-28");
        assert_eq!(add("2016-01-31", dec("1"), "month").unwrap(), "2016-02-29");
    }

    #[test]
    fn subtracts_via_negative_amounts() {
        assert_eq!(add("2015", dec("-1"), "year").unwrap(), "2014");
        assert_eq!(add("2014-03-01", dec("-1"), "day").unwrap(), "2014-02-28");
    }

    #[test]
    fn time_units_carry_and_keep_the_zone() {
        assert_eq!(
            add("2015-02-04T23:30:00Z", dec("1"), "hour").unwrap(),
            "2015-02-05T00:30:00Z"
        );
        assert_eq!(
            add("2015-02-04T14:00:00+01:00", dec("30"), "minute").unwrap(),
            "2015-02-04T14:30:00+01:00"
        );
    }

    #[test]
    fn rejects_units_finer_than_the_precision() {
        assert!(add("2014", dec("1"), "hour").is_none());
        assert!(add("bogus", dec("1"), "day").is_none());
        assert!(add("2014-01-01", dec("0.5"), "day").is_none());
    }
}
