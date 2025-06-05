use serde::{Deserialize, Deserializer};

pub fn maybe_vec_deserialize<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    data: D,
) -> Result<Vec<T>, D::Error> {
    let maybe: Option<Vec<T>> = Deserialize::deserialize(data)?;
    Ok(maybe.unwrap_or_default())
}
