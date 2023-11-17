use chrono::{DateTime, NaiveDateTime, Utc};
use zerucontent::Number;

pub fn datetime_from_number(modified: Number) -> DateTime<Utc> {
    let epoch = if let Number::Integer(epoch) = modified {
        let count = epoch.checked_ilog10().unwrap_or(0) + 1;
        if count == 13 {
            epoch as i64
        } else if count == 10 {
            (epoch * 1000) as i64
        } else {
            unreachable!("epoch is not 10 or 13 digits");
        }
    } else if let Number::Float(epoch) = modified {
        (epoch * 1000.0) as i64
    } else {
        unreachable!("modified is not integer or float");
    };
    DateTime::from_utc(NaiveDateTime::from_timestamp_millis(epoch).unwrap(), Utc)
}

pub fn number_from_datetime(modified: DateTime<Utc>) -> Number {
    let epoch = modified.timestamp();
    Number::Integer(epoch as usize)
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, NaiveDateTime};

    #[test]
    fn test_datetime_from_number() {
        use super::datetime_from_number;
        use zerucontent::Number;

        let modified = Number::Integer(1610000000000);
        let datetime = datetime_from_number(modified);
        assert_eq!(datetime.timestamp_millis(), 1610000000000);

        let modified = Number::Float(1_610_000_000.000);
        let datetime = datetime_from_number(modified);
        assert_eq!(datetime.timestamp_millis(), 1610000000000);
    }

    #[test]
    fn test_number_from_datetime() {
        use super::number_from_datetime;
        use zerucontent::Number;

        let modified = DateTime::from_utc(
            NaiveDateTime::from_timestamp_millis(1609977600000).unwrap(),
            chrono::Utc,
        );
        let number = number_from_datetime(modified);
        if let Number::Integer(number) = number {
            assert_eq!(number, 1609977600);
        } else {
            unreachable!("number is not integer");
        }
    }
}
