use std::io;

#[derive(Debug)]
pub enum Error {
    RequestSerializationFailed,
    RequestDeserializationFailed,
    ResponseSerializationFailed,
    ResponseDeserializationFailed,
}

impl From<Error> for io::Error {
    fn from(err: Error) -> Self {
        io::Error::new(io::ErrorKind::Other, format!("{:?}", err))
    }
}