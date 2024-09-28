use time::{format_description, Date, OffsetDateTime, PrimitiveDateTime};

const DATETIME_FORMAT: &str = "[day]/[month]/[year] [hour]:[minute]:[second]";
const DATE_FORMAT: &str = "[day]/[month]/[year]";

pub fn date_as_human_readable(date: Date) -> String {
    let format = format_description::parse_borrowed::<2>(DATE_FORMAT).unwrap();
    date.format(&format).unwrap()
}

pub fn datetime_as_human_readable(date: &Option<OffsetDateTime>) -> String {
    if let Some(date) = date {
        let format =
            format_description::parse_borrowed::<2>(DATETIME_FORMAT).unwrap();
        date.format(&format).unwrap()
    } else {
        "".to_string()
    }
}

pub fn datetime_from_human_readable(
    new_input: &str,
    old_date: &OffsetDateTime,
) -> time::Result<Option<OffsetDateTime>> {
    if new_input.is_empty() {
        return Ok(None);
    }
    let format =
        format_description::parse_borrowed::<2>(DATETIME_FORMAT).unwrap();
    let new_date = PrimitiveDateTime::parse(new_input, &format)?;
    Ok(Some(new_date.assume_offset(old_date.offset())))
}
