use chrono::Duration;

pub fn duration_to_hms(duration: &Duration) -> String {
    let total_seconds = duration.num_seconds();
    let seconds = total_seconds % 60;
    let minutes = (total_seconds / 60) % 60;
    let hours = (total_seconds / 60) / 60;
    format!("{}:{:0>2}:{:0>2}", hours, minutes, seconds)
}
