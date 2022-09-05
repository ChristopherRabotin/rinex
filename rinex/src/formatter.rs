#[derive(Debug)]
pub enum ParseError {
    /// Failed to parse (x, y, z) triplet
    Point3dXyz, 
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Point3dXyz => write!(f, "failed to parse (x,y,z) triplet"),
        }
    }
}

#[cfg(feature = "with-serde")]
pub mod point3d {
    use super::ParseError;
    use std::str::FromStr;
    use serde::{Serializer, Deserializer, Deserialize, de::Error};
    pub fn serialize<S>(point3d: &Option<rust_3d::Point3D>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let p = point3d.as_ref().unwrap_or(
            &rust_3d::Point3D {
                x: 0.0_f64,
                y: 0.0_f64,
                z: 0.0_f64,
            }
        );
        let s = format!("{},{},{}",p.x,p.y,p.z); 
        serializer.serialize_str(&s)
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<rust_3d::Point3D, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let items: Vec<&str> = s.split(",").collect();
        if let Ok(x) = f64::from_str(items[0]) {
            if let Ok(y) = f64::from_str(items[1]) {
                if let Ok(z) = f64::from_str(items[2]) {
                    return Ok(rust_3d::Point3D {x, y, z })
                }
            }
        }
        Err(ParseError::Point3dXyz)
            .map_err(D::Error::custom)
    }
}


#[cfg(feature = "with-serde")]
pub mod datetime {
    use serde::{Serializer};
    pub fn serialize<S>(datetime: &chrono::NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", datetime.format("%Y-%m-%d %H:%M:%S"));
        serializer.serialize_str(&s)
    }

    /*pub fn deserialize<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>, 
    {
        let s = String::deserialize(deserializer)?;
        chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")?
    }*/
}

#[cfg(feature = "with-serde")]
pub mod opt_datetime {
    use serde::Serializer;
    pub fn serialize<S>(datetime: &Option<chrono::NaiveDateTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(datetime) = datetime {
            let s = format!("{}", datetime.format("%Y-%m-%d %H:%M:%S"));
            serializer.serialize_str(&s)
        } else {
            serializer.serialize_str("")
        }
    }
}
