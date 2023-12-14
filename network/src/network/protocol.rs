use std::io;

use async_trait::async_trait;
use futures::{AsyncReadExt, AsyncWriteExt};
use libp2p::{
    futures::{AsyncRead, AsyncWrite},
    request_response::{Codec as RequestResponseCodec, ProtocolSupport},
};
use rkyv::{
    archived_root,
    ser::{serializers::AllocSerializer, Serializer},
    Archive, Deserialize, Infallible, Serialize,
};

use super::{NetworkNode, error::Error};

#[derive(Clone, Debug, Archive, Deserialize, Serialize)]
pub struct NetworkNodeRecord {
    pub nodes: Vec<NetworkNode>,
}

#[derive(Clone, Default)]
pub struct DecentNetProtocol();

impl AsRef<str> for DecentNetProtocol {
    fn as_ref(&self) -> &str {
        "/decentnet/0.0.1"
    }
}

impl Iterator for DecentNetProtocol {
    type Item = (Self, ProtocolSupport);

    fn next(&mut self) -> Option<Self::Item> {
        Some((DecentNetProtocol(), ProtocolSupport::Full))
    }
}

#[derive(Clone, Debug, Archive, Deserialize, Serialize)]
pub enum DecentNetRequest {
    Ping,
    GetNetworkNodes,
    SendNodeRecord(NetworkNodeRecord),
}

impl TryFrom<Vec<u8>> for DecentNetRequest {
    type Error = Error;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let archived = unsafe { archived_root::<DecentNetRequest>(&bytes[..]) };
        let req = archived.deserialize(&mut Infallible);
        req.map_err(|_| Error::RequestDeserializationFailed)
    }
}

impl TryFrom<DecentNetRequest> for Vec<u8> {
    type Error = Error;

    fn try_from(value: DecentNetRequest) -> Result<Self, Self::Error> {
        let mut serializer = AllocSerializer::<256>::default();
        match serializer.serialize_value(&value) {
            Ok(_) => {
                Ok(serializer.into_serializer().into_inner().to_vec())
            }
            Err(_e) => {
                Err(Error::RequestSerializationFailed)
            }
        }
    }
}

#[derive(Clone, Debug, Archive, Deserialize, Serialize)]
pub enum DecentNetResponse {
    Pong,
    Record(NetworkNodeRecord),
    GotNetworkRecord,
}

impl TryFrom<Vec<u8>> for DecentNetResponse {
    type Error = Error;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let archived = unsafe { archived_root::<DecentNetResponse>(&bytes[..]) };
        let res = archived.deserialize(&mut Infallible);
        res.map_err(|_| Error::ResponseDeserializationFailed)
    }
}

impl TryFrom<DecentNetResponse> for Vec<u8> {
    type Error = Error;

    fn try_from(res: DecentNetResponse) -> Result<Self, Self::Error> {
        let mut serializer = AllocSerializer::<256>::default();
        match serializer.serialize_value(&res) {
            Ok(_) => {
                Ok(serializer.into_serializer().into_inner().to_vec())
            }
            Err(_e) => {
                Err(Error::ResponseSerializationFailed)
            }
        }
    }
}

#[async_trait]
impl RequestResponseCodec for DecentNetProtocol {
    type Protocol = DecentNetProtocol;
    type Request = DecentNetRequest;
    type Response = DecentNetResponse;

    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut buf = Vec::new();
        match io.take(2048).read_to_end(&mut buf).await {
            Ok(_) => {}
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, e));
            }
        }
        DecentNetRequest::try_from(buf).map_err(|err| err.into() )
    }

    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut buf = Vec::new();
        match io.take(2048).read_to_end(&mut buf).await {
            Ok(_) => {}
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, e));
            }
        }
        DecentNetResponse::try_from(buf).map_err(|err| err.into() )
    }

    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let buf = Vec::try_from(req);
        if let Err(e) = buf {
            return Err(e.into());
        } else {
            io.write_all(&buf.unwrap()).await
        }
    }

    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let buf = Vec::try_from(res);
        if let Err(e) = buf {
            return Err(e.into());
        } else {
            io.write_all(&buf.unwrap()).await
        }
    }
}
